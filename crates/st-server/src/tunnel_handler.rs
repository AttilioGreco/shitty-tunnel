use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_util::codec::{FramedRead, FramedWrite};

use st_domain::model::request::ProxiedResponse;
use st_protocol::codec::TunnelCodec;
use st_protocol::message::TunnelMessage;

use crate::app::{AppState, PendingRequest, TunnelHandle};

pub async fn accept_loop(state: Arc<AppState>, listener: TcpListener) -> Result<()> {
    loop {
        let (stream, addr) = listener.accept().await?;
        tracing::info!("tunnel connection from {addr}");
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_tunnel_connection(state, stream).await {
                tracing::error!("tunnel error from {addr}: {e}");
            }
        });
    }
}

async fn handle_tunnel_connection(state: Arc<AppState>, stream: TcpStream) -> Result<()> {
    let (read_half, write_half) = stream.into_split();
    let mut reader = FramedRead::new(read_half, TunnelCodec);
    let mut writer = FramedWrite::new(write_half, TunnelCodec);

    // --- Authentication ---
    let auth_msg = reader
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("connection closed before auth"))??;

    let (public_key, timestamp, signature) = match auth_msg {
        TunnelMessage::AuthRequest {
            public_key,
            timestamp,
            signature,
        } => {
            let sig: [u8; 64] = signature
                .try_into()
                .map_err(|_| anyhow::anyhow!("invalid signature length"))?;
            (public_key, timestamp, sig)
        }
        _ => anyhow::bail!("expected AuthRequest, got {:?}", auth_msg),
    };

    let peer = state
        .authenticator
        .verify_peer(&public_key, timestamp, &signature)
        .await
        .map_err(|e| anyhow::anyhow!("auth failed: {e}"))?;

    let domain = peer.domain.clone();

    // Check if domain already has an active tunnel
    {
        let tunnels = state.tunnels.read().await;
        if tunnels.contains_key(&domain) {
            writer
                .send(TunnelMessage::AuthResponse {
                    success: false,
                    domain: None,
                    server_public_key: state.authenticator.public_key(),
                    server_signature: vec![0u8; 64],
                })
                .await?;
            anyhow::bail!("domain {domain} already has an active tunnel");
        }
    }

    // Send auth response
    let server_signature = state.authenticator.sign_challenge(&timestamp.to_be_bytes());
    writer
        .send(TunnelMessage::AuthResponse {
            success: true,
            domain: Some(domain.clone()),
            server_public_key: state.authenticator.public_key(),
            server_signature: server_signature.to_vec(),
        })
        .await?;

    tracing::info!("peer authenticated, tunnel active for {domain}");

    // --- Register tunnel ---
    let (request_tx, mut request_rx) = mpsc::channel::<PendingRequest>(32);
    state
        .tunnels
        .write()
        .await
        .insert(domain.clone(), TunnelHandle { request_tx });

    // Pending responses map: request_id -> oneshot sender
    let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<ProxiedResponse>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let pending_read = pending.clone();

    // Task: read responses from client
    let read_task = async {
        while let Some(msg) = reader.next().await {
            let msg = msg?;
            match msg {
                TunnelMessage::HttpResponse(resp) => {
                    let request_id = resp.request_id;
                    if let Some(tx) = pending_read.lock().await.remove(&request_id) {
                        let _ = tx.send(resp);
                    } else {
                        tracing::warn!("received response for unknown request_id={request_id}");
                    }
                }
                TunnelMessage::Pong { .. } => {}
                TunnelMessage::Disconnect { reason } => {
                    tracing::info!("client disconnected: {reason}");
                    break;
                }
                other => {
                    tracing::warn!("unexpected message from client: {other:?}");
                }
            }
        }
        Ok::<_, anyhow::Error>(())
    };

    // Task: forward requests to client
    let write_task = async {
        while let Some(pending_req) = request_rx.recv().await {
            let request_id = pending_req.request.request_id;
            pending
                .lock()
                .await
                .insert(request_id, pending_req.response_tx);

            if let Err(e) = writer
                .send(TunnelMessage::HttpRequest(pending_req.request))
                .await
            {
                tracing::error!("failed to send request to client: {e}");
                pending.lock().await.remove(&request_id);
                break;
            }
        }
        Ok::<_, anyhow::Error>(())
    };

    tokio::select! {
        r = read_task => {
            if let Err(e) = r {
                tracing::error!("tunnel read error for {domain}: {e}");
            }
        }
        r = write_task => {
            if let Err(e) = r {
                tracing::error!("tunnel write error for {domain}: {e}");
            }
        }
    }

    // Cleanup
    state.tunnels.write().await.remove(&domain);
    tracing::info!("tunnel closed for {domain}");

    Ok(())
}

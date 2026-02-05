use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status, Streaming};

use st_domain::model::request::ProxiedResponse;
use st_protocol::proto;
use st_protocol::proto::shitty_tunnel_server::ShittyTunnel;
use st_protocol::proto::{
    client_message, server_message, AuthResponse, ClientMessage, ServerMessage,
};

use crate::app::{AppState, PendingRequest, TunnelHandle};

pub struct TunnelGrpcService {
    state: Arc<AppState>,
}

impl TunnelGrpcService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl ShittyTunnel for TunnelGrpcService {
    type OpenTunnelStream =
        Pin<Box<dyn tokio_stream::Stream<Item = Result<ServerMessage, Status>> + Send>>;

    async fn open_tunnel(
        &self,
        request: Request<Streaming<ClientMessage>>,
    ) -> Result<Response<Self::OpenTunnelStream>, Status> {
        let mut in_stream = request.into_inner();

        // --- Authentication ---
        let first_msg = in_stream
            .next()
            .await
            .ok_or_else(|| Status::aborted("connection closed before auth"))?
            .map_err(|e| Status::internal(format!("stream error: {e}")))?;

        let auth_req = match first_msg.msg {
            Some(client_message::Msg::AuthRequest(a)) => a,
            _ => return Err(Status::invalid_argument("expected AuthRequest")),
        };

        let pk: [u8; 32] = auth_req
            .public_key
            .try_into()
            .map_err(|_| Status::invalid_argument("public key must be 32 bytes"))?;
        let sig: [u8; 64] = auth_req
            .signature
            .try_into()
            .map_err(|_| Status::invalid_argument("signature must be 64 bytes"))?;

        let peer = self
            .state
            .authenticator
            .verify_peer(&pk, auth_req.timestamp, &sig)
            .await
            .map_err(|e| Status::unauthenticated(e.to_string()))?;

        let domain = peer.domain;

        // Check domain not already connected
        {
            let tunnels = self.state.tunnels.read().await;
            if tunnels.contains_key(&domain) {
                return Err(Status::already_exists(format!(
                    "domain {domain} already has an active tunnel"
                )));
            }
        }

        tracing::info!("peer authenticated, tunnel active for {domain}");

        // Create output channel for server -> client messages
        let (out_tx, out_rx) = mpsc::channel::<Result<ServerMessage, Status>>(32);

        // Send auth response
        let server_sig = self
            .state
            .authenticator
            .sign_challenge(&auth_req.timestamp.to_be_bytes());
        out_tx
            .send(Ok(ServerMessage {
                msg: Some(server_message::Msg::AuthResponse(AuthResponse {
                    success: true,
                    domain: domain.clone(),
                    server_public_key: self.state.authenticator.public_key().to_vec(),
                    server_signature: server_sig.to_vec(),
                })),
            }))
            .await
            .map_err(|_| Status::internal("failed to send auth response"))?;

        // Register tunnel
        let (request_tx, mut request_rx) = mpsc::channel::<PendingRequest>(32);
        self.state
            .tunnels
            .write()
            .await
            .insert(domain.clone(), TunnelHandle { request_tx });

        // Pending responses map
        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<ProxiedResponse>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let state = self.state.clone();
        let domain_clone = domain.clone();

        // Spawn tunnel lifecycle task
        tokio::spawn(async move {
            let pending_read = pending.clone();

            let read_task = async {
                while let Some(result) = in_stream.next().await {
                    let msg = match result {
                        Ok(m) => m,
                        Err(e) => {
                            tracing::error!("stream read error: {e}");
                            break;
                        }
                    };
                    match msg.msg {
                        Some(client_message::Msg::HttpResponse(resp)) => {
                            let domain_resp: ProxiedResponse = resp.into();
                            let request_id = domain_resp.request_id;
                            if let Some(tx) = pending_read.lock().await.remove(&request_id) {
                                let _ = tx.send(domain_resp);
                            } else {
                                tracing::warn!("response for unknown request_id={request_id}");
                            }
                        }
                        Some(client_message::Msg::Pong(_)) => {}
                        _ => {}
                    }
                }
            };

            let write_task = async {
                while let Some(pending_req) = request_rx.recv().await {
                    let request_id = pending_req.request.request_id;
                    pending
                        .lock()
                        .await
                        .insert(request_id, pending_req.response_tx);

                    let proto_req: proto::HttpRequest = pending_req.request.into();
                    let msg = ServerMessage {
                        msg: Some(server_message::Msg::HttpRequest(proto_req)),
                    };

                    if out_tx.send(Ok(msg)).await.is_err() {
                        tracing::error!("failed to send request to client");
                        pending.lock().await.remove(&request_id);
                        break;
                    }
                }
            };

            tokio::select! {
                _ = read_task => {}
                _ = write_task => {}
            }

            state.tunnels.write().await.remove(&domain_clone);
            tracing::info!("tunnel closed for {domain_clone}");
        });

        let output_stream = ReceiverStream::new(out_rx);
        Ok(Response::new(Box::pin(output_stream) as Self::OpenTunnelStream))
    }
}

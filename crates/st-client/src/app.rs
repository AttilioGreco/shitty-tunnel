use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::codec::{FramedRead, FramedWrite};

use st_infra::config::client::ClientConfig;
use st_infra::crypto::keys::KeyPair;
use st_protocol::codec::TunnelCodec;
use st_protocol::message::TunnelMessage;

use crate::forwarder;

pub struct ClientApp {
    pub config: ClientConfig,
    pub key_pair: KeyPair,
    pub server_public_key: [u8; 32],
}

impl ClientApp {
    pub async fn run(&self) -> Result<()> {
        let reconnect = self
            .config
            .reconnect
            .clone()
            .unwrap_or_default();

        let mut delay = Duration::from_millis(reconnect.initial_delay_ms);
        let max_delay = Duration::from_millis(reconnect.max_delay_ms);

        loop {
            match self.connect_and_serve().await {
                Ok(()) => {
                    tracing::info!("tunnel closed gracefully");
                    break;
                }
                Err(e) => {
                    tracing::warn!("tunnel error: {e}");
                    if !reconnect.enabled {
                        return Err(e);
                    }
                    tracing::info!("reconnecting in {}ms...", delay.as_millis());
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(max_delay);
                }
            }
        }

        Ok(())
    }

    async fn connect_and_serve(&self) -> Result<()> {
        let addr = format!(
            "{}:{}",
            self.config.client.server_host, self.config.client.server_port
        );

        tracing::info!("connecting to {addr}");
        let stream = TcpStream::connect(&addr).await?;
        tracing::info!("connected to {addr}");

        let (read_half, write_half) = stream.into_split();
        let mut reader = FramedRead::new(read_half, TunnelCodec);
        let mut writer = FramedWrite::new(write_half, TunnelCodec);

        // --- Authentication ---
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let signature = self.key_pair.sign(&timestamp.to_be_bytes());

        writer
            .send(TunnelMessage::AuthRequest {
                public_key: self.key_pair.public_key_bytes(),
                timestamp,
                signature: signature.to_vec(),
            })
            .await?;

        let auth_resp = reader
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("connection closed during auth"))??;

        let domain = match auth_resp {
            TunnelMessage::AuthResponse {
                success: true,
                domain: Some(domain),
                server_public_key,
                server_signature,
            } => {
                // Verify server identity
                if server_public_key != self.server_public_key {
                    anyhow::bail!("server public key mismatch");
                }
                let sig: [u8; 64] = server_signature
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("invalid server signature length"))?;
                let valid = st_infra::crypto::keys::verify_signature(
                    &server_public_key,
                    &timestamp.to_be_bytes(),
                    &sig,
                );
                if !valid {
                    anyhow::bail!("server signature invalid");
                }
                domain
            }
            TunnelMessage::AuthResponse {
                success: false, ..
            } => {
                anyhow::bail!("server rejected authentication");
            }
            other => {
                anyhow::bail!("unexpected auth response: {other:?}");
            }
        };

        tracing::info!("authenticated, tunnel active for {domain}");
        tracing::info!(
            "forwarding to {}:{}",
            self.config.local.host,
            self.config.local.port
        );

        // --- Tunnel active ---
        let local_url = format!(
            "http://{}:{}",
            self.config.local.host, self.config.local.port
        );

        let http_client = reqwest::Client::builder()
            .no_proxy()
            .build()?;

        let (response_tx, mut response_rx) = mpsc::channel::<TunnelMessage>(32);

        // Task: read requests from server, spawn forwarding tasks
        let read_task = {
            let response_tx = response_tx.clone();
            let http_client = http_client.clone();
            let local_url = local_url.clone();
            async move {
                while let Some(msg) = reader.next().await {
                    let msg = msg?;
                    match msg {
                        TunnelMessage::HttpRequest(req) => {
                            let tx = response_tx.clone();
                            let client = http_client.clone();
                            let url = local_url.clone();
                            tokio::spawn(async move {
                                let resp = forwarder::forward(&client, &url, req).await;
                                let _ = tx.send(TunnelMessage::HttpResponse(resp)).await;
                            });
                        }
                        TunnelMessage::Ping { nonce } => {
                            let _ = response_tx.send(TunnelMessage::Pong { nonce }).await;
                        }
                        TunnelMessage::Disconnect { reason } => {
                            tracing::info!("server disconnect: {reason}");
                            break;
                        }
                        _ => {}
                    }
                }
                Ok::<_, anyhow::Error>(())
            }
        };

        // Task: write responses back to server
        let write_task = async move {
            while let Some(msg) = response_rx.recv().await {
                writer.send(msg).await?;
            }
            Ok::<_, anyhow::Error>(())
        };

        tokio::select! {
            r = read_task => r?,
            r = write_task => r?,
        }

        Ok(())
    }
}

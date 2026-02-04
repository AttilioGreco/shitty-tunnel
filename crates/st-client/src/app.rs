use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use st_infra::config::client::ClientConfig;
use st_infra::crypto::keys::KeyPair;
use st_protocol::proto;
use st_protocol::proto::shitty_tunnel_client::ShittyTunnelClient;
use st_protocol::proto::{
    client_message, server_message, AuthRequest, ClientMessage, ServerMessage,
};

use crate::forwarder;

pub struct ClientApp {
    pub config: ClientConfig,
    pub key_pair: KeyPair,
    pub server_public_key: [u8; 32],
}

impl ClientApp {
    pub async fn run(&self) -> Result<()> {
        let reconnect = self.config.reconnect.clone().unwrap_or_default();

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
        let endpoint = format!(
            "http://{}:{}",
            self.config.client.server_host, self.config.client.server_port
        );

        tracing::info!("connecting to {endpoint}");

        let mut client = ShittyTunnelClient::connect(endpoint.clone()).await?;

        tracing::info!("connected to {endpoint}");

        // Create outgoing channel (client -> server)
        let (out_tx, out_rx) = mpsc::channel::<ClientMessage>(32);

        // --- Authentication ---
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let signature = self.key_pair.sign(&timestamp.to_be_bytes());

        out_tx
            .send(ClientMessage {
                msg: Some(client_message::Msg::AuthRequest(AuthRequest {
                    public_key: self.key_pair.public_key_bytes().to_vec(),
                    timestamp,
                    signature: signature.to_vec(),
                })),
            })
            .await?;

        // Start bidirectional stream
        let response = client
            .open_tunnel(ReceiverStream::new(out_rx))
            .await?;

        let mut in_stream = response.into_inner();

        // Read auth response
        let first_msg = in_stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("connection closed during auth"))??;

        let domain = match first_msg.msg {
            Some(server_message::Msg::AuthResponse(auth_resp)) => {
                if !auth_resp.success {
                    anyhow::bail!("server rejected authentication");
                }

                // Verify server identity
                let server_pk: [u8; 32] = auth_resp
                    .server_public_key
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("invalid server public key length"))?;

                if server_pk != self.server_public_key {
                    anyhow::bail!("server public key mismatch");
                }

                let sig: [u8; 64] = auth_resp
                    .server_signature
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("invalid server signature length"))?;

                if !st_infra::crypto::keys::verify_signature(
                    &server_pk,
                    &timestamp.to_be_bytes(),
                    &sig,
                ) {
                    anyhow::bail!("server signature invalid");
                }

                auth_resp.domain
            }
            _ => anyhow::bail!("expected AuthResponse"),
        };

        tracing::info!("authenticated, tunnel active for {domain}");
        tracing::info!(
            "forwarding to {}:{}",
            self.config.local.host, self.config.local.port
        );

        // --- Tunnel active ---
        let local_url = format!(
            "http://{}:{}",
            self.config.local.host, self.config.local.port
        );

        let http_client = reqwest::Client::builder().no_proxy().build()?;

        // Read messages from server, spawn forwarding tasks
        while let Some(result) = in_stream.next().await {
            let msg: ServerMessage = result?;
            match msg.msg {
                Some(server_message::Msg::HttpRequest(req)) => {
                    let tx = out_tx.clone();
                    let client = http_client.clone();
                    let url = local_url.clone();
                    tokio::spawn(async move {
                        let domain_req = req.into();
                        let resp = forwarder::forward(&client, &url, domain_req).await;
                        let proto_resp: proto::HttpResponse = resp.into();
                        let _ = tx
                            .send(ClientMessage {
                                msg: Some(client_message::Msg::HttpResponse(proto_resp)),
                            })
                            .await;
                    });
                }
                Some(server_message::Msg::Ping(ping)) => {
                    let _ = out_tx
                        .send(ClientMessage {
                            msg: Some(client_message::Msg::Pong(proto::Pong {
                                nonce: ping.nonce,
                            })),
                        })
                        .await;
                }
                Some(server_message::Msg::Disconnect(d)) => {
                    tracing::info!("server disconnect: {}", d.reason);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }
}

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

use crate::client_forwarder as forwarder;
use crate::dashboard::buffer::EventBuffer;

pub struct ClientApp {
    pub config: ClientConfig,
    pub key_pair: KeyPair,
    pub server_public_key: [u8; 32],
    pub event_buffer: Option<std::sync::Arc<EventBuffer>>,
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
                    tracing::warn!("tunnel error: {e:#}");
                    tracing::debug!("tunnel error details: {e:?}");
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
        // Build endpoint URL - parse server_host as full URL if it contains schema
        let endpoint = if self.config.client.server_host.contains("://") {
            // Parse as full URL
            let parsed = url::Url::parse(&self.config.client.server_host)
                .map_err(|e| anyhow::anyhow!("invalid server URL: {}", e))?;

            let scheme = parsed.scheme();
            let host = parsed
                .host_str()
                .ok_or_else(|| anyhow::anyhow!("missing host in server URL"))?;

            // Use port from URL if present, otherwise use default for scheme
            let port = parsed.port().unwrap_or_else(|| {
                if scheme == "https" {
                    443
                } else {
                    80
                }
            });

            format!("{}://{}:{}", scheme, host, port)
        } else {
            // No schema in server_host, build URL from separate host + port
            let port = self
                .config
                .client
                .server_port
                .ok_or_else(|| anyhow::anyhow!("server_port is required when server_host is not a full URL"))?;

            let scheme = if port == 443 { "https" } else { "http" };
            format!("{}://{}:{}", scheme, self.config.client.server_host, port)
        };

        tracing::info!("connecting to {endpoint}");

        // Parse endpoint to extract hostname for SNI
        let endpoint_url = endpoint.parse::<tonic::transport::Endpoint>()?;

        // For HTTPS, configure TLS with proper SNI
        let endpoint_url = if endpoint.starts_with("https://") {
            let parsed = url::Url::parse(&endpoint)?;
            let domain = parsed.host_str()
                .ok_or_else(|| anyhow::anyhow!("missing host in endpoint"))?;

            tracing::debug!("configuring TLS with domain: {}", domain);

            endpoint_url
                .tls_config(tonic::transport::ClientTlsConfig::new()
                    .domain_name(domain))?
        } else {
            endpoint_url
        };

        // Configure HTTP/2 keepalive to prevent connection timeouts
        let endpoint_url = endpoint_url
            .http2_keep_alive_interval(Duration::from_secs(20))
            .keep_alive_timeout(Duration::from_secs(60))
            .keep_alive_while_idle(true);

        tracing::debug!("HTTP/2 keepalive configured: interval=20s, timeout=60s");

        let mut client = ShittyTunnelClient::connect(endpoint_url).await
            .map_err(|e| anyhow::anyhow!("failed to connect to {}: {}", endpoint, e))?;

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

        if self.config.local.basic_auth.is_empty() {
            tracing::warn!("basic_auth is NOT configured — requests will pass through without authentication");
        } else {
            tracing::info!("basic_auth is configured: \"{}\"", self.config.local.basic_auth);
        }

        if let Some(add) = &self.config.local.add_headers {
            for (k, v) in &add.0 {
                tracing::info!("header inject: {k} = {v}");
            }
        }
        if let Some(remove) = &self.config.local.remove_headers {
            for name in &remove.names {
                tracing::info!("header strip:  {name}");
            }
        }

        // --- Tunnel active ---
        let local_url = format!(
            "http://{}:{}",
            self.config.local.host, self.config.local.port
        );

        let http_client = reqwest::Client::builder().no_proxy().build()?;
        let basic_auth = self.config.local.basic_auth.clone();

        // Read messages from server, spawn forwarding tasks
        while let Some(result) = in_stream.next().await {
            let msg: ServerMessage = result?;
            match msg.msg {
                Some(server_message::Msg::HttpRequest(req)) => {
                    let tx = out_tx.clone();
                    let client = http_client.clone();
                    let url = local_url.clone();
                    let auth = basic_auth.clone();
                    let add_headers = self.config.local.add_headers.as_ref().cloned();
                    let remove_headers = self.config.local.remove_headers.as_ref().cloned();
                    let buffer = self.event_buffer.clone();
                    tokio::spawn(async move {
                        let domain_req: st_domain::model::request::ProxiedRequest = req.into();

                        let event_id = if let Some(buf) = &buffer {
                            Some(
                                buf.record_request_started(
                                    &domain_req.method,
                                    &domain_req.uri,
                                    &domain_req.headers,
                                    &domain_req.body,
                                )
                                .await,
                            )
                        } else {
                            None
                        };

                        let start = std::time::Instant::now();
                        let resp = forwarder::forward(
                            &client,
                            &url,
                            domain_req,
                            &auth,
                            add_headers.as_ref(),
                            remove_headers.as_ref(),
                        )
                        .await;
                        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

                        if let (Some(id), Some(buf)) = (event_id, &buffer) {
                            buf.record_request_completed(
                                id,
                                resp.status,
                                &resp.headers,
                                &resp.body,
                                duration_ms,
                            )
                            .await;
                        }

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

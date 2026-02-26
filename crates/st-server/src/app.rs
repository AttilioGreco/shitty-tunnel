use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::{mpsc, oneshot, RwLock};

use st_domain::model::request::{ProxiedRequest, ProxiedResponse};
use st_domain::port::auth::Authenticator;
use st_infra::config::server::ServerConfig;
use st_protocol::proto::shitty_tunnel_server::ShittyTunnelServer;

pub struct AppState {
    pub tunnels: RwLock<HashMap<String, TunnelHandle>>,
    pub authenticator: Arc<dyn Authenticator>,
    pub config: ServerConfig,
}

pub struct TunnelHandle {
    pub request_tx: mpsc::Sender<PendingRequest>,
}

pub struct PendingRequest {
    pub request: ProxiedRequest,
    pub response_tx: oneshot::Sender<ProxiedResponse>,
}

pub async fn run(state: Arc<AppState>) -> Result<()> {
    let public_addr = format!("0.0.0.0:{}", state.config.server.public_port);
    let tunnel_addr: std::net::SocketAddr = format!("0.0.0.0:{}", state.config.server.tunnel_port)
        .parse()?;

    let public_listener = tokio::net::TcpListener::bind(&public_addr).await?;
    tracing::info!("public HTTP listening on {}", public_addr);
    tracing::info!("gRPC tunnel listening on {}", tunnel_addr);

    let public_app = crate::public_handler::router(state.clone());

    let tunnel_service = crate::tunnel_handler::TunnelGrpcService::new(state.clone());
    let grpc_server = tonic::transport::Server::builder()
        .add_service(
            ShittyTunnelServer::new(tunnel_service)
                .max_decoding_message_size(st_protocol::GRPC_MAX_MESSAGE_SIZE),
        )
        .serve(tunnel_addr);

    tokio::select! {
        r = axum::serve(public_listener, public_app) => {
            r.map_err(|e| anyhow::anyhow!("public server error: {e}"))?;
        }
        r = grpc_server => {
            r.map_err(|e| anyhow::anyhow!("gRPC server error: {e}"))?;
        }
    }

    Ok(())
}

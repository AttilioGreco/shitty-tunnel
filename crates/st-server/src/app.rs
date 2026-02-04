use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::{mpsc, oneshot, RwLock};

use st_domain::model::request::{ProxiedRequest, ProxiedResponse};
use st_domain::port::auth::Authenticator;
use st_infra::config::server::ServerConfig;

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
    let tunnel_addr = format!("0.0.0.0:{}", state.config.server.tunnel_port);

    let public_listener = tokio::net::TcpListener::bind(&public_addr).await?;
    tracing::info!("public HTTP listening on {}", public_addr);

    let tunnel_listener = tokio::net::TcpListener::bind(&tunnel_addr).await?;
    tracing::info!("tunnel listener on {}", tunnel_addr);

    let public_app = crate::public_handler::router(state.clone());

    tokio::select! {
        r = axum::serve(public_listener, public_app) => {
            r.map_err(|e| anyhow::anyhow!("public server error: {e}"))?;
        }
        r = crate::tunnel_handler::accept_loop(state.clone(), tunnel_listener) => {
            r?;
        }
    }

    Ok(())
}

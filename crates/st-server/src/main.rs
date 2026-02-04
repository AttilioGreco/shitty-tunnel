use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use base64::prelude::*;
use clap::Parser;
use tokio::sync::RwLock;

use st_domain::model::peer::PeerIdentity;
use st_domain::port::auth::Authenticator;
use st_infra::config::server::ServerConfig;
use st_infra::crypto::auth::Ed25519Authenticator;
use st_infra::crypto::keys::KeyPair;

mod app;
mod public_handler;
mod tunnel_handler;

#[derive(Parser)]
#[command(name = "shitty-server", about = "shittyTunnel server")]
struct Cli {
    #[arg(short, long, default_value = "/etc/shittyTunnel/server.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let config_str = std::fs::read_to_string(&cli.config)
        .with_context(|| format!("failed to read config: {}", cli.config.display()))?;
    let config: ServerConfig =
        toml::from_str(&config_str).context("failed to parse server config")?;

    tracing::info!("loaded config from {}", cli.config.display());
    tracing::info!("registered {} peers", config.peers.len());

    let key_pair = KeyPair::from_base64(&config.server.private_key)
        .context("invalid server private key")?;

    let allowed_peers: Vec<PeerIdentity> = config
        .peers
        .iter()
        .map(|p| {
            let pk_bytes = BASE64_STANDARD
                .decode(p.public_key.trim())
                .with_context(|| format!("invalid base64 for peer {}", p.domain))?;
            let pk: [u8; 32] = pk_bytes
                .try_into()
                .map_err(|_| anyhow::anyhow!("invalid key length for peer {}", p.domain))?;
            Ok(PeerIdentity {
                public_key: pk,
                domain: p.domain.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let authenticator: Arc<dyn Authenticator> =
        Arc::new(Ed25519Authenticator::new(key_pair, allowed_peers));

    let state = Arc::new(app::AppState {
        tunnels: RwLock::new(HashMap::new()),
        authenticator,
        config,
    });

    app::run(state).await
}

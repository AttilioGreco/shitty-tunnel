use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use base64::prelude::*;
use tokio::sync::RwLock;

use st_domain::model::peer::PeerIdentity;
use st_domain::port::auth::Authenticator;
use st_domain::port::peer::PeerRepository;
use st_infra::config::provider::FileServerConfigProvider;
use st_infra::config::server::ServerConfig;
use st_infra::crypto::auth::Ed25519Authenticator;
use st_infra::crypto::keys::KeyPair;
use st_infra::peer::in_memory::InMemoryPeerRepository;

pub async fn run_server(config_path: PathBuf) -> Result<()> {
    let config_str = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read config: {}", config_path.display()))?;

    // Expand environment variables in config (e.g., ${VAR_NAME})
    let config_str = st_infra::config::env::expand_env_vars(&config_str)
        .context("failed to expand environment variables in config")?;

    let config: ServerConfig =
        toml::from_str(&config_str).context("failed to parse server config")?;

    tracing::info!("loaded config from {}", config_path.display());
    tracing::info!("registered {} peers", config.peers.len());

    let key_pair = KeyPair::from_base64(&config.server.private_key)
        .context("invalid server private key")?;

    let initial_peers: Vec<PeerIdentity> = config
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

    let peer_repository: Arc<dyn PeerRepository> =
        Arc::new(InMemoryPeerRepository::new(initial_peers));

    let authenticator: Arc<dyn Authenticator> =
        Arc::new(Ed25519Authenticator::new(key_pair, peer_repository.clone()));

    let config_provider = Arc::new(FileServerConfigProvider::from_settings(&config.server));

    let state = Arc::new(crate::app::AppState {
        tunnels: RwLock::new(HashMap::new()),
        authenticator,
        config: config_provider,
    });

    crate::app::run(state).await
}

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;

use st_infra::config::client::ClientConfig;
use st_infra::crypto::keys::KeyPair;

mod app;
mod forwarder;

#[derive(Parser)]
#[command(name = "shitty-client", about = "shittyTunnel client")]
struct Cli {
    #[arg(
        short,
        long,
        default_value = "~/.config/shittyTunnel.toml"
    )]
    config: PathBuf,
}

fn expand_tilde(path: &PathBuf) -> PathBuf {
    if let Some(s) = path.to_str() {
        if s.starts_with("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                return PathBuf::from(home).join(&s[2..]);
            }
        }
    }
    path.clone()
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let config_path = expand_tilde(&cli.config);

    let config_str = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read config: {}", config_path.display()))?;
    let config: ClientConfig =
        toml::from_str(&config_str).context("failed to parse client config")?;

    let key_pair = KeyPair::from_base64(&config.client.private_key)
        .context("invalid client private key")?;

    let server_pk = KeyPair::from_base64(&config.client.server_public_key)
        .context("invalid server public key")?;
    let server_public_key = server_pk.public_key_bytes();

    let client_app = Arc::new(app::ClientApp {
        config,
        key_pair,
        server_public_key,
    });

    client_app.run().await
}

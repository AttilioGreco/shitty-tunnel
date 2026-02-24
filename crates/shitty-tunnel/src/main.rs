use anyhow::Result;
use clap::{Parser, Subcommand};

mod server_main;
mod client_app;
mod client_forwarder;
mod dashboard;

// Server modules
mod app;
mod public_handler;
mod tunnel_handler;

#[derive(Parser)]
#[command(name = "shitty-tunnel", about = "Self-hosted HTTP tunnel")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the server
    Server {
        #[arg(short, long, default_value = "/etc/shittyTunnel/server.toml")]
        config: std::path::PathBuf,
    },
    /// Run the client
    Client {
        #[arg(short, long, default_value = "~/.config/shittyTunnel.toml")]
        config: std::path::PathBuf,
    },
    /// Generate Ed25519 keypair
    Keygen,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Server { config } => server_main::run_server(config).await,
        Commands::Client { config } => run_client(config).await,
        Commands::Keygen => {
            run_keygen();
            Ok(())
        }
    }
}

async fn run_client(config_path: std::path::PathBuf) -> Result<()> {
    use anyhow::Context;
    use st_infra::config::client::ClientConfig;
    use st_infra::crypto::keys::KeyPair;
    use std::sync::Arc;

    let config_path = expand_tilde(&config_path);

    let config_str = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read config: {}", config_path.display()))?;

    // Expand environment variables in config (e.g., ${VAR_NAME})
    let config_str = st_infra::config::env::expand_env_vars(&config_str)
        .context("failed to expand environment variables in config")?;

    let config: ClientConfig =
        toml::from_str(&config_str).context("failed to parse client config")?;

    let key_pair =
        KeyPair::from_base64(&config.client.private_key).context("invalid client private key")?;

    let server_public_key: [u8; 32] = {
        use base64::Engine;
        base64::prelude::BASE64_STANDARD
            .decode(config.client.server_public_key.trim())
    }
        .context("invalid server public key base64")?
        .try_into()
        .map_err(|_| anyhow::anyhow!("server public key must be 32 bytes"))?;

    let dashboard_settings = config
        .dashboard
        .clone()
        .unwrap_or_default();

    let event_buffer = if dashboard_settings.enabled {
        Some(Arc::new(dashboard::buffer::EventBuffer::new(
            dashboard_settings.max_events,
        )))
    } else {
        None
    };

    let client_app = Arc::new(client_app::ClientApp {
        config,
        key_pair,
        server_public_key,
        event_buffer: event_buffer.clone(),
    });

    if let Some(buffer) = event_buffer {
        tokio::select! {
            r = client_app.run() => r,
            r = dashboard::server::run(buffer, dashboard_settings.port) => r,
        }
    } else {
        client_app.run().await
    }
}

fn run_keygen() {
    use st_infra::crypto::keys::KeyPair;
    let kp = KeyPair::generate();
    println!("Private key: {}", kp.private_to_base64());
    println!("Public key:  {}", kp.public_to_base64());
}

fn expand_tilde(path: &std::path::PathBuf) -> std::path::PathBuf {
    if let Some(s) = path.to_str() {
        if s.starts_with("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                return std::path::PathBuf::from(home).join(&s[2..]);
            }
        }
    }
    path.clone()
}

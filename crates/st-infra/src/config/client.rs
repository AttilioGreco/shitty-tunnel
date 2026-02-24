use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ClientConfig {
    pub client: ClientSettings,
    pub local: LocalSettings,
    pub reconnect: Option<ReconnectSettings>,
    pub dashboard: Option<DashboardSettings>,
}

#[derive(Debug, Deserialize)]
pub struct ClientSettings {
    pub server_host: String,
    #[serde(default)]
    pub server_port: Option<u16>,
    pub private_key: String,
    pub server_public_key: String,
}

#[derive(Debug, Deserialize)]
pub struct LocalSettings {
    pub host: String,
    pub port: u16,
    /// Basic auth credentials in "user:password" format. Empty = no auth.
    #[serde(default)]
    pub basic_auth: String,
    /// Headers to add (or overwrite) on every proxied request and response.
    pub add_headers: Option<AddHeaders>,
    /// Headers to remove from every proxied request and response.
    pub remove_headers: Option<RemoveHeaders>,
}

/// Map of header name → value to inject.
/// In TOML: [local.add_headers] / "X-My-Header" = "value"
#[derive(Debug, Clone, Deserialize)]
pub struct AddHeaders(pub HashMap<String, String>);

/// List of header names to strip.
/// In TOML: [local.remove_headers] / names = ["Authorization", "Cookie"]
#[derive(Debug, Clone, Deserialize)]
pub struct RemoveHeaders {
    pub names: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReconnectSettings {
    pub enabled: bool,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for ReconnectSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DashboardSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_dashboard_port")]
    pub port: u16,
    #[serde(default = "default_max_events")]
    pub max_events: usize,
}

fn default_true() -> bool {
    true
}
fn default_dashboard_port() -> u16 {
    3001
}
fn default_max_events() -> usize {
    500
}

impl Default for DashboardSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 3001,
            max_events: 500,
        }
    }
}

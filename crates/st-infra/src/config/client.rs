use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ClientConfig {
    pub client: ClientSettings,
    pub local: LocalSettings,
    pub reconnect: Option<ReconnectSettings>,
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

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub server: ServerSettings,
    pub peers: Vec<PeerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ServerSettings {
    pub public_port: u16,
    pub tunnel_port: u16,
    pub private_key: String,
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PeerConfig {
    pub public_key: String,
    pub domain: String,
}

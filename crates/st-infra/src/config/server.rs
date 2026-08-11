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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_server_config_with_multiple_peers() {
        let cfg: ServerConfig = toml::from_str(
            r#"
            [server]
            public_port = 8080
            tunnel_port = 50051
            private_key = "PRIV"

            [[peers]]
            public_key = "KEY_A"
            domain = "a.example.com"

            [[peers]]
            public_key = "KEY_B"
            domain = "b.example.com"
        "#,
        )
        .unwrap();

        assert_eq!(cfg.server.public_port, 8080);
        assert_eq!(cfg.server.tunnel_port, 50051);
        assert!(cfg.server.tls.is_none());
        assert_eq!(cfg.peers.len(), 2);
        assert_eq!(cfg.peers[1].domain, "b.example.com");
    }

    #[test]
    fn a_server_with_no_peers_parses_but_authorises_nobody() {
        // `peers` is a top-level key: nesting it under [server] silently makes
        // it a different field, so it stays above the table here.
        let cfg: ServerConfig = toml::from_str(
            r#"
            peers = []

            [server]
            public_port = 8080
            tunnel_port = 50051
            private_key = "PRIV"
        "#,
        )
        .unwrap();

        assert!(cfg.peers.is_empty());
    }

    #[test]
    fn rejects_a_config_without_a_private_key() {
        let cfg = r#"
            peers = []

            [server]
            public_port = 8080
            tunnel_port = 50051
        "#;

        let err = toml::from_str::<ServerConfig>(cfg).unwrap_err();
        assert!(err.to_string().contains("private_key"), "{err}");
    }

    #[test]
    fn rejects_a_port_outside_the_u16_range() {
        let cfg = r#"
            peers = []

            [server]
            public_port = 70000
            tunnel_port = 50051
            private_key = "PRIV"
        "#;

        let err = toml::from_str::<ServerConfig>(cfg).unwrap_err();
        assert!(err.to_string().contains("public_port"), "{err}");
    }

    #[test]
    fn a_missing_peers_key_is_an_error_rather_than_an_empty_list() {
        let cfg = r#"
            [server]
            public_port = 8080
            tunnel_port = 50051
            private_key = "PRIV"
        "#;

        assert!(toml::from_str::<ServerConfig>(cfg).is_err());
    }
}

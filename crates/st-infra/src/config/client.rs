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
    /// Maximum response body size in bytes. Responses exceeding this limit
    /// will return a 502 error. Defaults to 100 MiB.
    #[serde(default = "default_max_body_size")]
    pub max_body_size: usize,
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
fn default_max_body_size() -> usize {
    100 * 1024 * 1024 // 100 MiB
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

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
        [client]
        server_host = "tunnel.example.com"
        server_port = 443
        private_key = "PRIV"
        server_public_key = "PUB"

        [local]
        host = "127.0.0.1"
        port = 3000
    "#;

    #[test]
    fn parses_a_minimal_config_and_fills_documented_defaults() {
        let cfg: ClientConfig = toml::from_str(MINIMAL).unwrap();

        assert_eq!(cfg.client.server_host, "tunnel.example.com");
        assert_eq!(cfg.client.server_port, Some(443));
        assert_eq!(cfg.local.port, 3000);
        assert_eq!(cfg.local.basic_auth, "", "auth must be off unless configured");
        assert_eq!(cfg.local.max_body_size, 100 * 1024 * 1024);
        assert!(cfg.local.add_headers.is_none());
        assert!(cfg.local.remove_headers.is_none());
        assert!(cfg.reconnect.is_none());
        assert!(cfg.dashboard.is_none());
    }

    #[test]
    fn server_port_is_optional_so_a_full_url_can_carry_it() {
        let cfg: ClientConfig = toml::from_str(
            r#"
            [client]
            server_host = "https://tunnel.example.com"
            private_key = "PRIV"
            server_public_key = "PUB"

            [local]
            host = "127.0.0.1"
            port = 3000
        "#,
        )
        .unwrap();

        assert_eq!(cfg.client.server_port, None);
    }

    #[test]
    fn rejects_a_config_missing_required_credentials() {
        let missing_key = r#"
            [client]
            server_host = "tunnel.example.com"
            server_public_key = "PUB"

            [local]
            host = "127.0.0.1"
            port = 3000
        "#;

        assert!(toml::from_str::<ClientConfig>(missing_key).is_err());
    }

    #[test]
    fn parses_header_manipulation_tables() {
        let cfg: ClientConfig = toml::from_str(
            r#"
            [client]
            server_host = "tunnel.example.com"
            server_port = 443
            private_key = "PRIV"
            server_public_key = "PUB"

            [local]
            host = "127.0.0.1"
            port = 3000
            basic_auth = "user:pass"
            max_body_size = 1024

            [local.add_headers]
            "X-Forwarded-Proto" = "https"

            [local.remove_headers]
            names = ["Cookie", "Authorization"]
        "#,
        )
        .unwrap();

        assert_eq!(cfg.local.basic_auth, "user:pass");
        assert_eq!(cfg.local.max_body_size, 1024);
        assert_eq!(
            cfg.local.add_headers.unwrap().0.get("X-Forwarded-Proto"),
            Some(&"https".to_string())
        );
        assert_eq!(cfg.local.remove_headers.unwrap().names, ["Cookie", "Authorization"]);
    }

    #[test]
    fn dashboard_defaults_apply_to_a_bare_table() {
        let cfg: ClientConfig =
            toml::from_str(&format!("{MINIMAL}\n[dashboard]\n")).unwrap();

        let dashboard = cfg.dashboard.unwrap();
        assert!(dashboard.enabled);
        assert_eq!(dashboard.port, 3001);
        assert_eq!(dashboard.max_events, 500);
    }

    #[test]
    fn an_absent_dashboard_table_matches_the_default_impl() {
        // main.rs falls back to `unwrap_or_default()`, so the two paths must agree.
        let explicit: DashboardSettings = toml::from_str("").unwrap();
        let implicit = DashboardSettings::default();

        assert_eq!(explicit.enabled, implicit.enabled);
        assert_eq!(explicit.port, implicit.port);
        assert_eq!(explicit.max_events, implicit.max_events);
    }

    #[test]
    fn reconnect_defaults_are_a_bounded_backoff() {
        let r = ReconnectSettings::default();

        assert!(r.enabled);
        assert!(
            r.initial_delay_ms < r.max_delay_ms,
            "backoff must have room to grow"
        );
    }
}

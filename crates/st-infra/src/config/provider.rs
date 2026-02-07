use st_domain::port::config::ServerConfigProvider;

use super::server::ServerSettings;

pub struct FileServerConfigProvider {
    public_port: u16,
    tunnel_port: u16,
}

impl FileServerConfigProvider {
    pub fn from_settings(settings: &ServerSettings) -> Self {
        Self {
            public_port: settings.public_port,
            tunnel_port: settings.tunnel_port,
        }
    }
}

impl ServerConfigProvider for FileServerConfigProvider {
    fn public_port(&self) -> u16 {
        self.public_port
    }

    fn tunnel_port(&self) -> u16 {
        self.tunnel_port
    }
}

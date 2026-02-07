pub trait ServerConfigProvider: Send + Sync {
    fn public_port(&self) -> u16;
    fn tunnel_port(&self) -> u16;
}

use super::peer::PeerId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TunnelId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TunnelState {
    Connecting,
    Authenticating,
    Active,
    Disconnected,
}

#[derive(Debug, Clone)]
pub struct TunnelInfo {
    pub id: TunnelId,
    pub state: TunnelState,
    pub peer: PeerId,
    pub connected_at: Option<u64>,
}

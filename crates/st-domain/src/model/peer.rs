use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub [u8; 32]);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerIdentity {
    pub public_key: [u8; 32],
    pub domain: String,
}

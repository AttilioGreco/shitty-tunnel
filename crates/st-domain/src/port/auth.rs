use async_trait::async_trait;

use crate::error::DomainError;
use crate::model::peer::PeerIdentity;

#[async_trait]
pub trait Authenticator: Send + Sync {
    async fn verify_peer(
        &self,
        public_key: &[u8; 32],
        timestamp: u64,
        signature: &[u8; 64],
    ) -> Result<PeerIdentity, DomainError>;

    fn sign_challenge(&self, data: &[u8]) -> [u8; 64];

    fn public_key(&self) -> [u8; 32];
}

use async_trait::async_trait;

use crate::error::DomainError;
use crate::model::peer::PeerIdentity;

#[async_trait]
pub trait PeerRepository: Send + Sync {
    async fn find_by_public_key(&self, public_key: &[u8; 32])
        -> Result<PeerIdentity, DomainError>;

    async fn list_all(&self) -> Result<Vec<PeerIdentity>, DomainError>;

    async fn add(&self, peer: PeerIdentity) -> Result<(), DomainError>;

    async fn remove(&self, public_key: &[u8; 32]) -> Result<(), DomainError>;
}

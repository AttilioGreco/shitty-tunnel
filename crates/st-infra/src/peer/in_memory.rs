use async_trait::async_trait;
use tokio::sync::RwLock;

use st_domain::error::DomainError;
use st_domain::model::peer::PeerIdentity;
use st_domain::port::peer::PeerRepository;

pub struct InMemoryPeerRepository {
    peers: RwLock<Vec<PeerIdentity>>,
}

impl InMemoryPeerRepository {
    pub fn new(initial_peers: Vec<PeerIdentity>) -> Self {
        Self {
            peers: RwLock::new(initial_peers),
        }
    }
}

#[async_trait]
impl PeerRepository for InMemoryPeerRepository {
    async fn find_by_public_key(
        &self,
        public_key: &[u8; 32],
    ) -> Result<PeerIdentity, DomainError> {
        self.peers
            .read()
            .await
            .iter()
            .find(|p| &p.public_key == public_key)
            .cloned()
            .ok_or(DomainError::PeerNotFound)
    }

    async fn list_all(&self) -> Result<Vec<PeerIdentity>, DomainError> {
        Ok(self.peers.read().await.clone())
    }

    async fn add(&self, peer: PeerIdentity) -> Result<(), DomainError> {
        let mut peers = self.peers.write().await;
        if peers.iter().any(|p| p.public_key == peer.public_key) {
            return Err(DomainError::PeerAlreadyExists);
        }
        peers.push(peer);
        Ok(())
    }

    async fn remove(&self, public_key: &[u8; 32]) -> Result<(), DomainError> {
        let mut peers = self.peers.write().await;
        let len_before = peers.len();
        peers.retain(|p| &p.public_key != public_key);
        if peers.len() == len_before {
            return Err(DomainError::PeerNotFound);
        }
        Ok(())
    }
}

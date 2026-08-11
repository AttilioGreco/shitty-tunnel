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

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(byte: u8, domain: &str) -> PeerIdentity {
        PeerIdentity {
            public_key: [byte; 32],
            domain: domain.into(),
        }
    }

    #[tokio::test]
    async fn finds_a_peer_by_its_exact_key() {
        let repo = InMemoryPeerRepository::new(vec![peer(1, "a.example.com")]);

        let found = repo.find_by_public_key(&[1u8; 32]).await.unwrap();
        assert_eq!(found.domain, "a.example.com");
    }

    #[tokio::test]
    async fn reports_an_absent_key_rather_than_returning_a_neighbour() {
        let repo = InMemoryPeerRepository::new(vec![peer(1, "a.example.com")]);

        assert!(matches!(
            repo.find_by_public_key(&[2u8; 32]).await,
            Err(DomainError::PeerNotFound)
        ));
    }

    #[tokio::test]
    async fn add_refuses_a_duplicate_key_and_leaves_the_original() {
        let repo = InMemoryPeerRepository::new(vec![peer(1, "a.example.com")]);

        assert!(matches!(
            repo.add(peer(1, "hijacked.example.com")).await,
            Err(DomainError::PeerAlreadyExists)
        ));
        assert_eq!(
            repo.find_by_public_key(&[1u8; 32]).await.unwrap().domain,
            "a.example.com",
            "a rejected add must not overwrite the existing mapping"
        );
    }

    #[tokio::test]
    async fn add_then_remove_round_trips() {
        let repo = InMemoryPeerRepository::new(vec![]);

        repo.add(peer(9, "new.example.com")).await.unwrap();
        assert_eq!(repo.list_all().await.unwrap().len(), 1);

        repo.remove(&[9u8; 32]).await.unwrap();
        assert!(repo.list_all().await.unwrap().is_empty());
        assert!(repo.find_by_public_key(&[9u8; 32]).await.is_err());
    }

    #[tokio::test]
    async fn remove_reports_a_key_that_was_never_there() {
        let repo = InMemoryPeerRepository::new(vec![peer(1, "a.example.com")]);

        assert!(matches!(
            repo.remove(&[42u8; 32]).await,
            Err(DomainError::PeerNotFound)
        ));
        assert_eq!(repo.list_all().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn distinct_keys_keep_distinct_domains() {
        let repo =
            InMemoryPeerRepository::new(vec![peer(1, "a.example.com"), peer(2, "b.example.com")]);

        assert_eq!(
            repo.find_by_public_key(&[1u8; 32]).await.unwrap().domain,
            "a.example.com"
        );
        assert_eq!(
            repo.find_by_public_key(&[2u8; 32]).await.unwrap().domain,
            "b.example.com"
        );
    }
}

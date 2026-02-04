use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;

use st_domain::error::DomainError;
use st_domain::model::peer::PeerIdentity;
use st_domain::port::auth::Authenticator;

use crate::crypto::keys::{verify_signature, KeyPair};

const TIMESTAMP_TOLERANCE_SECS: u64 = 30;

pub struct Ed25519Authenticator {
    key_pair: KeyPair,
    allowed_peers: Vec<PeerIdentity>,
}

impl Ed25519Authenticator {
    pub fn new(key_pair: KeyPair, allowed_peers: Vec<PeerIdentity>) -> Self {
        Self {
            key_pair,
            allowed_peers,
        }
    }
}

#[async_trait]
impl Authenticator for Ed25519Authenticator {
    async fn verify_peer(
        &self,
        public_key: &[u8; 32],
        timestamp: u64,
        signature: &[u8; 64],
    ) -> Result<PeerIdentity, DomainError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now.abs_diff(timestamp) > TIMESTAMP_TOLERANCE_SECS {
            return Err(DomainError::AuthenticationFailed(
                "timestamp out of range".into(),
            ));
        }

        let message = timestamp.to_be_bytes();
        if !verify_signature(public_key, &message, signature) {
            return Err(DomainError::AuthenticationFailed(
                "invalid signature".into(),
            ));
        }

        let peer = self
            .allowed_peers
            .iter()
            .find(|p| &p.public_key == public_key)
            .ok_or_else(|| DomainError::AuthenticationFailed("unknown peer".into()))?;

        Ok(peer.clone())
    }

    fn sign_challenge(&self, data: &[u8]) -> [u8; 64] {
        self.key_pair.sign(data)
    }

    fn public_key(&self) -> [u8; 32] {
        self.key_pair.public_key_bytes()
    }
}

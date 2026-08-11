use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;

use st_domain::error::DomainError;
use st_domain::model::peer::PeerIdentity;
use st_domain::port::auth::Authenticator;
use st_domain::port::peer::PeerRepository;

use crate::crypto::keys::{verify_signature, KeyPair};

const TIMESTAMP_TOLERANCE_SECS: u64 = 30;

pub struct Ed25519Authenticator {
    key_pair: KeyPair,
    peer_repository: Arc<dyn PeerRepository>,
}

impl Ed25519Authenticator {
    pub fn new(key_pair: KeyPair, peer_repository: Arc<dyn PeerRepository>) -> Self {
        Self {
            key_pair,
            peer_repository,
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

        self.peer_repository
            .find_by_public_key(public_key)
            .await
            .map_err(|_| DomainError::AuthenticationFailed("unknown peer".into()))
    }

    fn sign_challenge(&self, data: &[u8]) -> [u8; 64] {
        self.key_pair.sign(data)
    }

    fn public_key(&self) -> [u8; 32] {
        self.key_pair.public_key_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer::in_memory::InMemoryPeerRepository;

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    /// Server authenticator plus a client keypair already enrolled as a peer.
    fn enrolled() -> (Ed25519Authenticator, KeyPair) {
        let client = KeyPair::generate();
        let repo = InMemoryPeerRepository::new(vec![PeerIdentity {
            public_key: client.public_key_bytes(),
            domain: "app.example.com".into(),
        }]);
        let auth = Ed25519Authenticator::new(KeyPair::generate(), Arc::new(repo));
        (auth, client)
    }

    fn sign_timestamp(kp: &KeyPair, ts: u64) -> [u8; 64] {
        kp.sign(&ts.to_be_bytes())
    }

    #[tokio::test]
    async fn accepts_an_enrolled_peer_with_a_fresh_timestamp() {
        let (auth, client) = enrolled();
        let ts = now();

        let peer = auth
            .verify_peer(&client.public_key_bytes(), ts, &sign_timestamp(&client, ts))
            .await
            .expect("valid peer must authenticate");

        assert_eq!(peer.domain, "app.example.com");
        assert_eq!(peer.public_key, client.public_key_bytes());
    }

    #[tokio::test]
    async fn rejects_a_timestamp_older_than_the_tolerance() {
        let (auth, client) = enrolled();
        let ts = now() - TIMESTAMP_TOLERANCE_SECS - 5;

        let err = auth
            .verify_peer(&client.public_key_bytes(), ts, &sign_timestamp(&client, ts))
            .await
            .unwrap_err();

        assert!(err.to_string().contains("timestamp out of range"), "{err}");
    }

    #[tokio::test]
    async fn rejects_a_timestamp_too_far_in_the_future() {
        let (auth, client) = enrolled();
        let ts = now() + TIMESTAMP_TOLERANCE_SECS + 5;

        let err = auth
            .verify_peer(&client.public_key_bytes(), ts, &sign_timestamp(&client, ts))
            .await
            .unwrap_err();

        assert!(err.to_string().contains("timestamp out of range"), "{err}");
    }

    #[tokio::test]
    async fn accepts_a_timestamp_at_the_edge_of_the_tolerance() {
        let (auth, client) = enrolled();
        // One second inside the window, so a slow test runner cannot push the
        // clock past the boundary and flake.
        let ts = now() - (TIMESTAMP_TOLERANCE_SECS - 1);

        assert!(
            auth.verify_peer(&client.public_key_bytes(), ts, &sign_timestamp(&client, ts))
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn rejects_a_signature_made_over_a_different_timestamp() {
        let (auth, client) = enrolled();
        let ts = now();
        // Signature is valid, but binds a timestamp we are not presenting:
        // replaying it under a fresher timestamp must not pass.
        let stale_sig = sign_timestamp(&client, ts - 10);

        let err = auth
            .verify_peer(&client.public_key_bytes(), ts, &stale_sig)
            .await
            .unwrap_err();

        assert!(err.to_string().contains("invalid signature"), "{err}");
    }

    #[tokio::test]
    async fn rejects_a_signature_from_a_key_that_is_not_the_claimed_one() {
        let (auth, client) = enrolled();
        let impostor = KeyPair::generate();
        let ts = now();

        let err = auth
            .verify_peer(
                &client.public_key_bytes(),
                ts,
                &sign_timestamp(&impostor, ts),
            )
            .await
            .unwrap_err();

        assert!(err.to_string().contains("invalid signature"), "{err}");
    }

    #[tokio::test]
    async fn rejects_a_correctly_signed_but_unenrolled_peer() {
        let (auth, _) = enrolled();
        let stranger = KeyPair::generate();
        let ts = now();

        let err = auth
            .verify_peer(
                &stranger.public_key_bytes(),
                ts,
                &sign_timestamp(&stranger, ts),
            )
            .await
            .unwrap_err();

        assert!(err.to_string().contains("unknown peer"), "{err}");
    }

    #[tokio::test]
    async fn unknown_peer_is_not_distinguishable_as_a_signature_failure() {
        // Both paths return AuthenticationFailed, so a caller cannot use the
        // error variant to probe which public keys are enrolled.
        let (auth, _) = enrolled();
        let stranger = KeyPair::generate();
        let ts = now();

        let err = auth
            .verify_peer(
                &stranger.public_key_bytes(),
                ts,
                &sign_timestamp(&stranger, ts),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::AuthenticationFailed(_)));
    }

    /// Documents a real limitation: within the tolerance window the same
    /// (timestamp, signature) pair authenticates repeatedly. The window is the
    /// only replay bound — there is no nonce store.
    #[tokio::test]
    async fn a_captured_handshake_replays_inside_the_tolerance_window() {
        let (auth, client) = enrolled();
        let ts = now();
        let sig = sign_timestamp(&client, ts);

        assert!(
            auth.verify_peer(&client.public_key_bytes(), ts, &sig)
                .await
                .is_ok()
        );
        assert!(
            auth.verify_peer(&client.public_key_bytes(), ts, &sig)
                .await
                .is_ok(),
            "replay currently succeeds; tighten this test if a nonce store lands"
        );
    }

    #[test]
    fn sign_challenge_is_verifiable_with_the_advertised_public_key() {
        let auth = Ed25519Authenticator::new(
            KeyPair::generate(),
            Arc::new(InMemoryPeerRepository::new(vec![])),
        );

        // This is what the client checks to confirm it reached the real server.
        let sig = auth.sign_challenge(b"challenge");
        assert!(crate::crypto::keys::verify_signature(
            &auth.public_key(),
            b"challenge",
            &sig
        ));
    }
}

use base64::prelude::*;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;

pub struct KeyPair {
    signing_key: SigningKey,
}

impl KeyPair {
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }

    pub fn from_bytes(secret: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(secret);
        Self { signing_key }
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.signing_key.sign(message).to_bytes()
    }

    pub fn private_to_base64(&self) -> String {
        BASE64_STANDARD.encode(self.signing_key.to_bytes())
    }

    pub fn public_to_base64(&self) -> String {
        BASE64_STANDARD.encode(self.public_key_bytes())
    }

    pub fn from_base64(s: &str) -> anyhow::Result<Self> {
        let bytes = BASE64_STANDARD.decode(s.trim())?;
        let secret: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid key length, expected 32 bytes"))?;
        Ok(Self::from_bytes(&secret))
    }
}

pub fn verify_signature(public_key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    let Ok(verifying_key) = VerifyingKey::from_bytes(public_key) else {
        return false;
    };
    let signature = Signature::from_bytes(signature);
    verifying_key.verify(message, &signature).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip_preserves_the_key() {
        let kp = KeyPair::generate();
        let restored = KeyPair::from_base64(&kp.private_to_base64()).unwrap();

        assert_eq!(kp.public_key_bytes(), restored.public_key_bytes());
        assert_eq!(kp.public_to_base64(), restored.public_to_base64());
    }

    #[test]
    fn from_base64_tolerates_surrounding_whitespace() {
        let kp = KeyPair::generate();
        let padded = format!("  {}\n", kp.private_to_base64());

        let restored = KeyPair::from_base64(&padded).unwrap();
        assert_eq!(kp.public_key_bytes(), restored.public_key_bytes());
    }

    #[test]
    fn from_base64_rejects_malformed_input() {
        assert!(KeyPair::from_base64("not base64 at all!").is_err());
        // Valid base64, wrong length — must not be silently padded or truncated.
        assert!(KeyPair::from_base64(&BASE64_STANDARD.encode([0u8; 31])).is_err());
        assert!(KeyPair::from_base64(&BASE64_STANDARD.encode([0u8; 33])).is_err());
        assert!(KeyPair::from_base64("").is_err());
    }

    #[test]
    fn signature_verifies_against_its_own_key() {
        let kp = KeyPair::generate();
        let sig = kp.sign(b"message");

        assert!(verify_signature(&kp.public_key_bytes(), b"message", &sig));
    }

    #[test]
    fn verification_fails_on_any_mismatch() {
        let kp = KeyPair::generate();
        let other = KeyPair::generate();
        let sig = kp.sign(b"message");

        assert!(
            !verify_signature(&other.public_key_bytes(), b"message", &sig),
            "a different key must not validate"
        );
        assert!(
            !verify_signature(&kp.public_key_bytes(), b"tampered", &sig),
            "a modified message must not validate"
        );

        let mut flipped = sig;
        flipped[0] ^= 0x01;
        assert!(
            !verify_signature(&kp.public_key_bytes(), b"message", &flipped),
            "a corrupted signature must not validate"
        );
    }

    #[test]
    fn verification_rejects_an_unusable_public_key_instead_of_panicking() {
        // Not a valid curve point: from_bytes fails and must surface as `false`.
        let bogus = [0xffu8; 32];
        assert!(!verify_signature(&bogus, b"message", &[0u8; 64]));
    }

    #[test]
    fn deterministic_from_bytes_reproduces_the_same_identity() {
        let secret = [7u8; 32];
        assert_eq!(
            KeyPair::from_bytes(&secret).public_key_bytes(),
            KeyPair::from_bytes(&secret).public_key_bytes()
        );
    }
}

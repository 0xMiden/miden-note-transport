//! X25519 + AES256-GCM encryption scheme

pub use x25519_dalek::PublicKey as X25519PublicKey;
use x25519_dalek::{EphemeralSecret, SharedSecret, StaticSecret};

use super::{EncryptionScheme, aes::Aes256GcmKey};
use crate::{Error, Result};

pub struct X25519;

/// X25519 keypair for asymmetric encryption
#[derive(Clone)]
pub struct X25519KeyPair {
    public_key: X25519PublicKey,
    private_key: StaticSecret,
}

impl X25519KeyPair {
    /// Generate a new X25519 keypair
    pub fn generate() -> Self {
        let private_key = StaticSecret::random();
        let public_key = X25519PublicKey::from(&private_key);
        Self { public_key, private_key }
    }

    /// Get the public key
    pub fn public_key(&self) -> &X25519PublicKey {
        &self.public_key
    }

    /// Derive a shared secret with another party's public key
    pub fn derive_shared_secret(&self, other_public_key: &X25519PublicKey) -> SharedSecret {
        self.private_key.diffie_hellman(other_public_key)
    }
}

/// Hybrid encryption using X25519 + AES-256-GCM
/// Encrypts data using a random ephemeral key and the recipient's public key
pub fn encrypt_data(data: &[u8], recipient_public_key: &X25519PublicKey) -> Result<Vec<u8>> {
    // Generate ephemeral keypair for this encryption
    let ephemeral_secret = EphemeralSecret::random();
    let ephemeral_public_key = X25519PublicKey::from(&ephemeral_secret);

    // Derive shared secret
    let shared_secret = ephemeral_secret.diffie_hellman(recipient_public_key);

    // Derive AES key from shared secret using HKDF-like approach
    let aes_key = derive_aes_key_from_shared_secret(&shared_secret);

    // Encrypt data with AES-GCM
    let encrypted_data = aes_key.encrypt_data(data)?;

    // Combine ephemeral public key and encrypted data
    let mut result = Vec::with_capacity(32 + encrypted_data.len());
    result.extend_from_slice(ephemeral_public_key.as_bytes());
    result.extend_from_slice(&encrypted_data);

    Ok(result)
}

/// Hybrid decryption using X25519 + AES-256-GCM
/// Decrypts data using the recipient's private key and ephemeral public key
pub fn decrypt_data(encrypted_data: &[u8], keypair: &X25519KeyPair) -> Result<Vec<u8>> {
    if encrypted_data.len() < 32 {
        return Err(Error::Decryption("Encrypted data too short for decryption".to_string()));
    }

    // Extract ephemeral public key and encrypted data
    let ephemeral_public_key_bytes = &encrypted_data[..32];
    let encrypted_data = &encrypted_data[32..];

    let ephemeral_public_key = X25519PublicKey::from(
        TryInto::<[u8; 32]>::try_into(ephemeral_public_key_bytes)
            .map_err(|_| Error::Decryption("Invalid ephemeral public key".to_string()))?,
    );

    let shared_secret = keypair.derive_shared_secret(&ephemeral_public_key);

    let aes_key = derive_aes_key_from_shared_secret(&shared_secret);

    aes_key.decrypt_data(encrypted_data)
}

/// Derive a 32-byte AES key from a X25519 shared secret
fn derive_aes_key_from_shared_secret(shared_secret: &SharedSecret) -> Aes256GcmKey {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(shared_secret.as_bytes());
    let hash_result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash_result);
    Aes256GcmKey::new(key)
}

impl EncryptionScheme for X25519 {
    type Key = X25519KeyPair;
    type PublicKey = X25519PublicKey;

    fn generate() -> Self::Key {
        X25519KeyPair::generate()
    }

    fn public_key(key: &Self::Key) -> Self::PublicKey {
        key.public_key
    }

    fn encrypt(key: &Self::PublicKey, plaintext: &[u8]) -> Result<Vec<u8>> {
        encrypt_data(plaintext, key)
    }

    fn decrypt(key: &Self::Key, ciphertext: &[u8]) -> Result<Vec<u8>> {
        decrypt_data(ciphertext, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_encryption_decryption() {
        let _alice_keypair = X25519KeyPair::generate();
        let bob_keypair = X25519KeyPair::generate();

        let data = b"Secret message for Bob!";

        let encrypted = encrypt_data(data, &bob_keypair.public_key).unwrap();

        let decrypted = decrypt_data(&encrypted, &bob_keypair).unwrap();

        assert_eq!(data, &decrypted[..]);
    }

    #[test]
    fn test_hybrid_wrong_recipient() {
        let _alice_keypair = X25519KeyPair::generate();
        let bob_keypair = X25519KeyPair::generate();
        let eve_keypair = X25519KeyPair::generate();

        let data = b"Secret message for Bob!";

        let encrypted = encrypt_data(data, &bob_keypair.public_key).unwrap();

        let result = decrypt_data(&encrypted, &eve_keypair);
        assert!(result.is_err());
    }

    #[test]
    fn test_keypair_generation() {
        let keypair1 = X25519KeyPair::generate();
        let keypair2 = X25519KeyPair::generate();

        assert_ne!(keypair1.public_key.as_bytes(), keypair2.public_key.as_bytes());
        assert_eq!(keypair1.public_key.as_bytes().len(), 32);
    }
}

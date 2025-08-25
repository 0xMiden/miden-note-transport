//! AES256-GCM encryption scheme

use aes_gcm::{
    Aes256Gcm as AesGcm, Key, Nonce,
    aead::{Aead, KeyInit},
};
use rand::RngCore;

use super::EncryptionKey;
use crate::{Error, Result};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Aes256GcmKey([u8; 32]);

impl Aes256GcmKey {
    /// Create a new key using the provided key data
    pub fn new(key: [u8; 32]) -> Self {
        Self(key)
    }

    /// Generate a random encryption key
    pub fn generate() -> Self {
        let mut key = [0u8; 32];
        rand::rng().fill_bytes(&mut key);
        Self::new(key)
    }

    /// Encrypt data using AES-GCM with a random nonce
    pub fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        let cipher = AesGcm::new(Key::<AesGcm>::from_slice(&self.0));
        let mut nonce_bytes = [0u8; 12];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| Error::Encryption(format!("Encryption failed: {e}")))?;

        // Combine nonce and ciphertext
        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    /// Decrypt data using AES-GCM
    pub fn decrypt_data(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        if encrypted_data.len() < 12 {
            return Err(Error::Decryption("Encrypted data too short".to_string()));
        }

        let cipher = AesGcm::new(Key::<AesGcm>::from_slice(&self.0));

        // Extract nonce and ciphertext
        let nonce = Nonce::from_slice(&encrypted_data[..12]);
        let ciphertext = &encrypted_data[12..];

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| Error::Decryption(format!("Decryption failed: {e}")))?;

        Ok(plaintext)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

// Implement the unified EncryptionKey trait
impl EncryptionKey for Aes256GcmKey {
    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        self.encrypt_data(plaintext)
    }

    fn decrypt(&self, ciphertext: &[u8]) -> Option<Result<Vec<u8>>> {
        Some(self.decrypt_data(ciphertext))
    }

    fn generate() -> Option<Self> {
        Some(Self::generate())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes_encryption_decryption() {
        let key = Aes256GcmKey::generate();
        let data = b"Hello, Miden Transport!";

        let encrypted = key.encrypt(data).unwrap();
        let decrypted = key.decrypt(&encrypted).unwrap().unwrap();

        assert_eq!(data, &decrypted[..]);
    }

    #[test]
    fn test_aes_wrong_key() {
        let key1 = Aes256GcmKey::generate();
        let key2 = Aes256GcmKey::generate();
        let data = b"Test data";

        let encrypted = key1.encrypt(data).unwrap();
        let result = key2.decrypt(&encrypted).unwrap();

        assert!(result.is_err());
    }
}

//! AES256-GCM encryption scheme

use aes_gcm::{
    Aes256Gcm as AesGcm, Key, Nonce,
    aead::{Aead, KeyInit},
};
use rand::RngCore;

use super::EncryptionScheme;
use crate::{Error, Result};

pub struct Aes256Gcm;

#[derive(Clone, Debug)]
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

impl EncryptionScheme for Aes256Gcm {
    type Key = Aes256GcmKey;
    type PublicKey = Aes256GcmKey;

    fn generate() -> Self::Key {
        Aes256GcmKey::generate()
    }

    fn public_key(key: &Self::Key) -> Self::PublicKey {
        key.clone()
    }

    fn encrypt(key: &Self::PublicKey, plaintext: &[u8]) -> Result<Vec<u8>> {
        key.encrypt_data(plaintext)
    }

    fn decrypt(key: &Self::Key, ciphertext: &[u8]) -> Result<Vec<u8>> {
        key.decrypt_data(ciphertext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes_encryption_decryption() {
        let key = Aes256GcmKey::generate();
        let data = b"Hello, Miden Transport!";

        let encrypted = Aes256Gcm::encrypt(&key, data).unwrap();
        let decrypted = Aes256Gcm::decrypt(&key, &encrypted).unwrap();

        assert_eq!(data, &decrypted[..]);
    }

    #[test]
    fn test_aes_wrong_key() {
        let key1 = Aes256GcmKey::generate();
        let key2 = Aes256GcmKey::generate();
        let data = b"Test data";

        let encrypted = Aes256Gcm::encrypt(&key1, data).unwrap();
        let result = Aes256Gcm::decrypt(&key2, &encrypted);

        assert!(result.is_err());
    }
}

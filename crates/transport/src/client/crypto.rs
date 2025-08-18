use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit},
};
use rand::{Rng, RngCore};

use crate::{Error, Result};

/// Encrypt data using AES-GCM with a random nonce
/// For PoC purposes, we use a simple symmetric encryption scheme
/// In production, this would be replaced with proper asymmetric encryption
pub fn encrypt(data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    if key.len() != 32 {
        return Err(Error::Encryption("Key must be 32 bytes for AES-256".to_string()));
    }

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
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
pub fn decrypt(encrypted_data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    if key.len() != 32 {
        return Err(Error::Decryption("Key must be 32 bytes for AES-256".to_string()));
    }

    if encrypted_data.len() < 12 {
        return Err(Error::Decryption("Encrypted data too short".to_string()));
    }

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

    // Extract nonce and ciphertext
    let nonce = Nonce::from_slice(&encrypted_data[..12]);
    let ciphertext = &encrypted_data[12..];

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| Error::Decryption(format!("Decryption failed: {e}")))?;

    Ok(plaintext)
}

/// Generate a random encryption key
pub fn generate_key() -> Vec<u8> {
    let mut key = vec![0u8; 32];
    rand::rng().fill_bytes(&mut key);
    key
}

/// Generate a random note tag
pub fn generate_note_tag() -> u32 {
    rand::rng().random()
}

/// Validate encryption key format
pub fn is_valid_encryption_key(key: &[u8]) -> bool {
    // For PoC, we expect 32-byte keys for AES-256
    key.len() == 32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let key = generate_key();
        let data = b"Hello, Miden Transport!";

        let encrypted = encrypt(data, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();

        assert_eq!(data, &decrypted[..]);
    }

    #[test]
    fn test_wrong_key() {
        let key1 = generate_key();
        let key2 = generate_key();
        let data = b"Test data";

        let encrypted = encrypt(data, &key1).unwrap();
        let result = decrypt(&encrypted, &key2);

        assert!(result.is_err());
    }

    #[test]
    fn test_note_tag_generation() {
        let tag1 = generate_note_tag();
        let tag2 = generate_note_tag();

        assert_ne!(tag1, tag2);
    }
}

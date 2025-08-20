pub mod aes;
pub mod hybrid;

use crate::Result;

/// A trait for encryption schemes (symmetric or asymmetric).
pub trait EncryptionScheme {
    /// The type of the key (symmetric key, or private/public key pair).
    type Key;

    /// The type of the public component (for asymmetric) or same as Key for symmetric.
    type PublicKey;

    /// Generate a new key (or keypair).
    fn generate() -> Self::Key;

    /// Get the public key, if applicable (for symmetric just return the key itself).
    fn public_key(key: &Self::Key) -> Self::PublicKey;

    /// Encrypt plaintext with the given key (or public key).
    fn encrypt(key: &Self::PublicKey, plaintext: &[u8]) -> Result<Vec<u8>>;

    /// Decrypt ciphertext with the given key (private for asymmetric, full key for symmetric).
    fn decrypt(key: &Self::Key, ciphertext: &[u8]) -> Result<Vec<u8>>;
}

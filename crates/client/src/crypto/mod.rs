pub mod aes;
pub mod hybrid;

use crate::Result;

/// Encryption key that performs encryption, and optionally decryption
///
/// In principle, symmetric encryption schemes use the same key so that key should be able to both
/// encrypt and decrypt.
/// In asymmetric encryption schemes, only when the `EncryptionKey` is a key pair, encryption and
/// decryption is supported. Public keys support only encryption.
pub trait EncryptionKey {
    /// Encrypt plaintext data (required - all keys must support encryption)
    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>>;

    /// Decrypt ciphertext data (optional - returns None if not supported)
    fn decrypt(&self, ciphertext: &[u8]) -> Option<Result<Vec<u8>>>;

    /// Generate a new key of this type (optional - returns None if not supported)
    fn generate() -> Option<Self>
    where
        Self: Sized;
}

/// Key serialization helper
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub enum SerializableKey {
    Aes256Gcm(self::aes::Aes256GcmKey),
    X25519(hybrid::X25519KeyPair),
    X25519Pub(hybrid::X25519PublicKey),
}

impl SerializableKey {
    /// Get the public key component if available
    pub fn public_key(&self) -> Option<Self> {
        match self {
            Self::Aes256Gcm(key) => Some(Self::Aes256Gcm(key.clone())), /* For symmetric, public */
            // = private
            Self::X25519(keypair) => Some(Self::X25519Pub(*keypair.public_key())),
            Self::X25519Pub(_) => Some(self.clone()),
        }
    }

    /// Check if this key can be used for encryption
    pub fn can_encrypt(&self) -> bool {
        matches!(self, Self::Aes256Gcm(_) | Self::X25519Pub(_))
    }

    /// Check if this key can be used for decryption
    pub fn can_decrypt(&self) -> bool {
        matches!(self, Self::Aes256Gcm(_) | Self::X25519(_))
    }

    /// Generate a new AES-256-GCM key
    pub fn generate_aes() -> Self {
        Self::Aes256Gcm(self::aes::Aes256GcmKey::generate())
    }

    /// Generate a new X25519 keypair
    pub fn generate_x25519() -> Self {
        Self::X25519(hybrid::X25519KeyPair::generate())
    }
}

impl EncryptionKey for SerializableKey {
    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        match self {
            Self::Aes256Gcm(key) => key.encrypt(plaintext),
            Self::X25519Pub(public_key) => public_key.encrypt(plaintext),
            Self::X25519(_) => {
                Err(crate::Error::Encryption("Cannot encrypt with private key".to_string()))
            },
        }
    }

    fn decrypt(&self, ciphertext: &[u8]) -> Option<Result<Vec<u8>>> {
        match self {
            Self::Aes256Gcm(key) => key.decrypt(ciphertext),
            Self::X25519(keypair) => keypair.decrypt(ciphertext),
            Self::X25519Pub(_) => None, // Public keys can't decrypt
        }
    }

    fn generate() -> Option<Self> {
        None
    }
}

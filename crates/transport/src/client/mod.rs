use crate::types::{
    EncryptedDetails, Note, NoteDetails, NoteHeader, NoteId, NoteInfo, NoteStatus, NoteTag,
};
use crate::{Error, Result};
use miden_objects::utils::{Deserializable, Serializable};

pub mod crypto;
pub mod grpc;

/// The main transport client trait for sending and receiving encrypted notes
#[async_trait::async_trait]
pub trait TransportClient: Send + Sync {
    /// Send a note with encrypted details
    async fn send_note(
        &mut self,
        header: NoteHeader,
        encrypted_note: EncryptedDetails,
    ) -> Result<(NoteId, NoteStatus)>;

    /// Fetch all notes for a given tag
    async fn fetch_notes(&mut self, tag: NoteTag) -> Result<Vec<NoteInfo>>;

    /// Mark a note as received to prevent re-downloading
    async fn mark_received(&mut self, note_ids: &[NoteId]) -> Result<()>;
}

/// Encryption store trait for managing encryption keys
pub trait EncryptionStore: Send + Sync {
    /// Decrypt a message using the given public encryption key
    fn decrypt(&self, pub_enc_key: &[u8], msg: &[u8]) -> Result<Vec<u8>>;

    /// Encrypt data for a recipient using their public key
    fn encrypt(&self, data: &[u8], recipient_pub_key: &[u8]) -> Result<Vec<u8>>;
}

/// Filesystem-based encryption store
pub struct FilesystemEncryptionStore {
    key_dir: std::path::PathBuf,
}

impl FilesystemEncryptionStore {
    pub fn new<P: AsRef<std::path::Path>>(key_dir: P) -> Result<Self> {
        let key_dir = key_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&key_dir)?;
        Ok(Self { key_dir })
    }

    pub fn add_key(&self, key_id: &str, key_data: &[u8]) -> Result<()> {
        let key_path = self.key_dir.join(format!("{key_id}.key"));
        std::fs::write(key_path, key_data)?;
        Ok(())
    }

    pub fn get_key(&self, key_id: &str) -> Result<Option<Vec<u8>>> {
        let key_path = self.key_dir.join(format!("{key_id}.key"));
        if key_path.exists() {
            Ok(Some(std::fs::read(key_path)?))
        } else {
            Ok(None)
        }
    }
}

impl EncryptionStore for FilesystemEncryptionStore {
    fn decrypt(&self, pub_enc_key: &[u8], msg: &[u8]) -> Result<Vec<u8>> {
        // TODO use self/use stored key
        crate::client::crypto::decrypt(msg, pub_enc_key)
    }

    fn encrypt(&self, data: &[u8], recipient_pub_key: &[u8]) -> Result<Vec<u8>> {
        // TODO use self/use stored key
        crate::client::crypto::encrypt(data, recipient_pub_key)
    }
}

/// Client for interacting with the transport layer
pub struct TransportLayerClient {
    transport_client: Box<dyn TransportClient>,
    encryption_store: Box<dyn EncryptionStore>,
}

impl TransportLayerClient {
    pub fn new(
        transport_client: Box<dyn TransportClient>,
        encryption_store: Box<dyn EncryptionStore>,
    ) -> Self {
        Self {
            transport_client,
            encryption_store,
        }
    }

    /// Send a note to a recipient
    pub async fn send_note(
        &mut self,
        note: Note,
        recipient_pub_key: &[u8],
    ) -> Result<(NoteId, NoteStatus)> {
        let header = *note.header();
        let details: NoteDetails = note.into();
        let details_bytes = details.to_bytes();
        let encrypted = self
            .encryption_store
            .encrypt(&details_bytes, recipient_pub_key)?;
        self.transport_client
            .send_note(header, EncryptedDetails(encrypted))
            .await
    }

    /// Fetch and decrypt notes for a tag
    pub async fn fetch_notes(
        &mut self,
        tag: NoteTag,
        pub_enc_key: &[u8],
    ) -> Result<Vec<(NoteHeader, NoteDetails)>> {
        let infos = self.transport_client.fetch_notes(tag).await?;
        let mut decrypted_notes = Vec::new();

        for info in infos {
            match self
                .encryption_store
                .decrypt(pub_enc_key, &info.encrypted_data)
            {
                Ok(decrypted) => {
                    let details = NoteDetails::read_from_bytes(&decrypted).map_err(|e| {
                        Error::Decryption(format!("Failed to deserialized decrypted details: {e}"))
                    })?;
                    decrypted_notes.push((info.header, details))
                }
                Err(_) => {
                    // Skip notes that can't be decrypted with this key
                    continue;
                }
            }
        }

        Ok(decrypted_notes)
    }

    /// Mark a note as received
    pub async fn mark_received(&mut self, note_ids: &[NoteId]) -> Result<()> {
        self.transport_client.mark_received(note_ids).await
    }
}

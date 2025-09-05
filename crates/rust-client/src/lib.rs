#![no_std]

#[macro_use]
extern crate alloc;
use alloc::{boxed::Box, vec::Vec};

#[cfg(feature = "std")]
extern crate std;

pub mod database;
pub mod error;
pub mod grpc;
#[cfg(feature = "std")]
pub mod logging;
pub mod types;

use futures::Stream;
use miden_objects::{
    address::Address,
    utils::{Deserializable, Serializable},
};

use self::{
    database::Database,
    types::{Note, NoteDetails, NoteHeader, NoteId, NoteInfo, NoteTag},
};
pub use self::{
    error::{Error, Result},
    grpc::GrpcClient,
};

/// The main transport client trait for sending and receiving encrypted notes
#[cfg_attr(not(feature = "web-tonic"), async_trait::async_trait)]
#[cfg_attr(feature = "web-tonic", async_trait::async_trait(?Send))]
pub trait TransportClient: Send + Sync {
    /// Send a note with optionally encrypted details
    async fn send_note(&mut self, header: NoteHeader, details: Vec<u8>) -> Result<NoteId>;

    /// Fetch all notes for a given tag
    async fn fetch_notes(&mut self, tag: NoteTag) -> Result<Vec<NoteInfo>>;

    /// Stream notes for a given tag
    async fn stream_notes(&mut self, tag: NoteTag) -> Result<Box<dyn NoteStream>>;
}

/// Stream trait for note streaming
pub trait NoteStream: Stream<Item = Result<Vec<NoteInfo>>> + Send + Unpin {}

/// Client for interacting with the transport layer
pub struct TransportLayerClient {
    transport_client: Box<dyn TransportClient>,
    /// Client database for persistent state
    database: Database,
    /// Owned addresses
    addresses: Vec<Address>,
}

impl TransportLayerClient {
    pub fn new(
        transport_client: Box<dyn TransportClient>,
        database: Database,
        addresses: Vec<Address>,
    ) -> Self {
        Self { transport_client, database, addresses }
    }

    /// Send a note to a recipient
    ///
    /// If the note tag in the provided note is different than the recipient's [`Address`] note tag,
    /// the provided note' tag is updated.
    pub async fn send_note(&mut self, note: Note, _address: &Address) -> Result<NoteId> {
        let header = *note.header();
        let details: NoteDetails = note.into();
        let details_bytes = details.to_bytes();
        self.transport_client.send_note(header, details_bytes).await
    }

    /// Fetch and decrypt notes for a tag
    pub async fn fetch_notes(&mut self, tag: NoteTag) -> Result<Vec<Note>> {
        let infos = self.transport_client.fetch_notes(tag).await?;
        let mut decrypted_notes = Vec::new();

        for info in infos {
            // Check if we've already fetched this note
            if !self.database.note_fetched(&info.header.id()).await? {
                // Mark note as fetched
                self.database.record_fetched_note(&info.header.id(), tag).await?;

                let details = NoteDetails::read_from_bytes(&info.details).map_err(|e| {
                    Error::Decryption(format!("Failed to deserialize decrypted details: {e}"))
                })?;
                let note = Note::new(
                    details.assets().clone(),
                    *info.header.metadata(),
                    details.recipient().clone(),
                );
                decrypted_notes.push(note);

                // Store the encrypted note
                self.database.store_note(&info.header, &info.details, info.created_at).await?;
            }
        }

        Ok(decrypted_notes)
    }

    /// Continuously fetch notes
    pub async fn stream_notes(&mut self, tag: NoteTag) -> Result<Box<dyn NoteStream>> {
        self.transport_client.stream_notes(tag).await
    }

    /// Adds an owned address
    pub fn add_address(&mut self, address: Address) {
        self.addresses.push(address);
    }

    /// Check if a note has been fetched before
    pub async fn note_fetched(&self, note_id: &NoteId) -> Result<bool> {
        self.database.note_fetched(note_id).await.map_err(Error::from)
    }

    /// Get all fetched note IDs for a specific tag
    pub async fn get_fetched_notes_for_tag(&self, tag: NoteTag) -> Result<Vec<NoteId>> {
        self.database.get_fetched_notes_for_tag(tag).await.map_err(Error::from)
    }

    /// Get an stored note from the database
    pub async fn get_stored_note(&self, note_id: &NoteId) -> Result<Option<database::StoredNote>> {
        self.database.get_stored_note(note_id).await.map_err(Error::from)
    }

    /// Get all stored notes for a specific tag
    pub async fn get_stored_notes_for_tag(
        &self,
        tag: NoteTag,
    ) -> Result<Vec<database::StoredNote>> {
        self.database.get_stored_notes_for_tag(tag).await.map_err(Error::from)
    }

    /// Get database statistics
    pub async fn get_database_stats(&self) -> Result<database::DatabaseStats> {
        self.database.get_stats().await.map_err(Error::from)
    }

    /// Clean up old data based on retention policy
    pub async fn cleanup_old_data(&self, retention_days: u32) -> Result<u64> {
        self.database.cleanup_old_data(retention_days).await.map_err(Error::from)
    }

    /// Register a tag
    pub fn register_tag(&self, _tag: NoteTag) -> Result<()> {
        // The purpose of this function will change, from encryption key -pairing focus to a
        // subscription purpose.
        // For now it does nothing.
        Ok(())
    }
}

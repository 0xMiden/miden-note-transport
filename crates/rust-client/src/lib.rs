//! # Miden Transport Layer Client Library
//!
//! This crate provides a lightweight client to communicate optionally-encrypted private notes with
//! the Miden Transport Layer.
//!
//! `no-std` is supported, with support also for WASM environments.
//!
//! ## Overview
//!
//! Notes are exchanged with the Transport Layer Node (and other users) where the
//! [`NoteTag`](`miden_objects::note::NoteTag`) serves as principal identifier for note routing.
//!
//! - **Sending a note**: to send a note call the [`TransportLayerClient::send_note`] function with
//!   the recipient's address. In the future, the note will be encrypted internally, to enable
//!   end-to-end encryption;
//! - **Fetching notes**: retrieve notes by their [`NoteTag`] using
//!   [`TransportLayerClient::fetch_notes`]. Previously fetched notes will not be returned, a
//!   feature enabled by a internal pagination mechanism;
//! - **Streaming notes**: similarly to fetching notes, but based on a real-time subscription
//!   mechanism.
//!
//! A local database keeps track of fetched notes and other client state.
//!
//! Communications with the Transport Layer Node are made through gRPC using `tonic`.
//! A database implementation is provided (`SQLite` for a `std` compilation, and `IndexedDB` for
//! WASM). Both the client-node gRPC communications and database implementations can be changed by
//! employing other strucs implementing the [`TransportClient`] and
//! [`DatabaseBackend`](`database::DatabaseBackend`) traits,
//! respectively.
//!
//! ## Example
//!
//! Below is a brief example on how to send and fetch notes:
//!
//! ```rust, no_run
//! use miden_objects::{address::Address, note::{Note, NoteTag}};
//! use miden_private_transport_client::{
//!     Error, Result, TransportLayerClient,
//!     database::{Database, DatabaseConfig},
//!     grpc::GrpcClient,
//!     test_utils::{mock_address, mock_note_p2id_with_addresses},
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Initialize the client
//!     let db_config = DatabaseConfig::default();
//!     let db = Database::new_sqlite(db_config).await?;
//!     let grpc = GrpcClient::connect("http://localhost:8080".to_string(), 1000).await?;
//!     let mut client = TransportLayerClient::new(Box::new(grpc), db, vec![]);
//!
//!     // Random data for this example
//!     let sender: Address = mock_address();
//!     let recipient: Address = mock_address();
//!     let note: Note = mock_note_p2id_with_addresses(&sender, &recipient);
//!
//!     // Send a note (needs a running server)
//!     client.send_note(note, &recipient).await?;
//!
//!     // Fetch notes (needs a running server)
//!     let tag = recipient.to_note_tag();
//!     let notes = client.fetch_notes(tag).await?;
//!
//!     Ok(())
//! }
//! ```

#![no_std]
#![deny(missing_docs)]

#[macro_use]
extern crate alloc;
use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};

#[cfg(feature = "std")]
extern crate std;

/// Database
pub mod database;
/// Error management
pub mod error;
/// gRPC client
pub mod grpc;
/// Tracing configuration
#[cfg(feature = "std")]
pub mod logging;
/// Testing utilities
///
/// Gated through the `testing` feature.
#[cfg(feature = "testing")]
pub mod test_utils;
/// Types used
pub mod types;

use chrono::Utc;
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
    async fn send_note(&mut self, header: NoteHeader, details: Vec<u8>) -> Result<()>;

    /// Fetch all notes with cursor greater than the provided cursor for a given tag
    async fn fetch_notes(&mut self, tag: NoteTag, cursor: u64) -> Result<Vec<NoteInfo>>;

    /// Stream notes for a given tag
    async fn stream_notes(&mut self, tag: NoteTag, cursor: u64) -> Result<Box<dyn NoteStream>>;
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
    /// Last fetched cursor
    lts: BTreeMap<NoteTag, u64>,
}

impl TransportLayerClient {
    /// Main client constructor
    pub fn new(
        transport_client: Box<dyn TransportClient>,
        database: Database,
        addresses: Vec<Address>,
    ) -> Self {
        let lts = BTreeMap::new();
        Self {
            transport_client,
            database,
            addresses,
            lts,
        }
    }

    /// Send a note to a recipient
    ///
    /// If the note tag in the provided note is different than the recipient's [`Address`] note tag,
    /// the provided note' tag is updated.
    pub async fn send_note(&mut self, note: Note, _address: &Address) -> Result<()> {
        let header = *note.header();
        let details: NoteDetails = note.into();
        let details_bytes = details.to_bytes();
        self.transport_client.send_note(header, details_bytes).await
    }

    /// Fetch and decrypt notes for a tag
    pub async fn fetch_notes(&mut self, tag: NoteTag) -> Result<Vec<Note>> {
        let cursor = self.lts.get(&tag).copied().unwrap_or(0);
        let infos = self.transport_client.fetch_notes(tag, cursor).await?;
        let mut decrypted_notes = Vec::new();

        let mut latest_cursor = cursor;
        for info in infos {
            // Check if we've already fetched this note
            if !self.database.note_fetched(&info.header.id()).await? {
                // Mark note as fetched
                self.database.record_fetched_note(&info.header.id(), tag).await?;

                let details = NoteDetails::read_from_bytes(&info.details)
                    .map_err(|e| Error::Internal(format!("Failed to deserialize details: {e}")))?;
                let note = Note::new(
                    details.assets().clone(),
                    *info.header.metadata(),
                    details.recipient().clone(),
                );
                decrypted_notes.push(note);

                // Use current time for created_at when storing notes
                let created_at = Utc::now();

                // Store the encrypted note
                self.database.store_note(&info.header, &info.details, created_at).await?;
            }

            // Update the latest received cursor
            let info_cursor = info.cursor;
            if info_cursor > latest_cursor {
                latest_cursor = info_cursor;
            }
        }

        // Update the last cursor to the most recent received cursor
        self.lts.insert(tag, latest_cursor);

        Ok(decrypted_notes)
    }

    /// Continuously fetch notes
    pub async fn stream_notes(&mut self, tag: NoteTag) -> Result<Box<dyn NoteStream>> {
        let cursor = self.lts.get(&tag).copied().unwrap_or(0);
        self.transport_client.stream_notes(tag, cursor).await
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

//! Provides an IndexedDB-backed implementation of the [DatabaseBackend] trait for web environments.
//!
//! This module enables persistence of client data (notes, tags) when running in a browser.
//! It uses wasm-bindgen to interface with JavaScript and `IndexedDB`, allowing the Miden client to
//! store and retrieve data asynchronously.
//!
//! **Note:** This implementation is only available when targeting WebAssembly with the `idxdb`
//! feature enabled.

use alloc::{boxed::Box, vec::Vec};

use chrono::{DateTime, Utc};
use miden_objects::note::{NoteHeader, NoteId, NoteTag};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, js_sys};

use crate::database::{DatabaseBackend, DatabaseError, DatabaseStats, StoredNote};

#[cfg(not(target_arch = "wasm32"))]
compile_error!("The `idxdb` feature is only supported when targeting wasm32.");

pub mod mgmt;
pub mod note;

// Initialize IndexedDB
#[wasm_bindgen(module = "/src/database/idxdb/js/schema.js")]
extern "C" {
    #[wasm_bindgen(js_name = openDatabase)]
    fn setup_indexed_db() -> js_sys::Promise;
}

pub struct IndexedDb {}

impl IndexedDb {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn init() -> Result<Self, JsValue> {
        JsFuture::from(setup_indexed_db()).await?;
        Ok(Self {})
    }
}

impl IndexedDb {
    // Note operations
    pub async fn store_note(
        &self,
        header: &NoteHeader,
        encrypted_data: &[u8],
        created_at: DateTime<Utc>,
    ) -> Result<(), DatabaseError> {
        note::store_note(header, encrypted_data, created_at).await
    }

    pub async fn get_stored_note(
        &self,
        note_id: &NoteId,
    ) -> Result<Option<StoredNote>, DatabaseError> {
        note::get_stored_note(note_id).await
    }

    pub async fn get_stored_notes_for_tag(
        &self,
        tag: NoteTag,
    ) -> Result<Vec<StoredNote>, DatabaseError> {
        note::get_stored_notes_for_tag(tag).await
    }

    pub async fn record_fetched_note(
        &self,
        note_id: &NoteId,
        tag: NoteTag,
    ) -> Result<(), DatabaseError> {
        note::record_fetched_note(note_id, tag).await
    }

    pub async fn note_fetched(&self, note_id: &NoteId) -> Result<bool, DatabaseError> {
        note::note_fetched(note_id).await
    }

    pub async fn get_fetched_notes_for_tag(
        &self,
        tag: NoteTag,
    ) -> Result<Vec<NoteId>, DatabaseError> {
        note::get_fetched_notes_for_tag(tag).await
    }

    // Stats and maintenance
    pub async fn get_stats(&self) -> Result<DatabaseStats, DatabaseError> {
        mgmt::get_stats().await
    }

    pub async fn cleanup_old_data(&self, retention_days: u32) -> Result<u64, DatabaseError> {
        mgmt::cleanup_old_data(retention_days).await
    }
}

#[async_trait::async_trait(?Send)]
impl DatabaseBackend for IndexedDb {
    /// Store a note
    async fn store_note(
        &self,
        header: &NoteHeader,
        encrypted_data: &[u8],
        created_at: DateTime<Utc>,
    ) -> Result<(), DatabaseError> {
        self.store_note(header, encrypted_data, created_at).await
    }

    /// Get an stored note by ID
    async fn get_stored_note(&self, note_id: &NoteId) -> Result<Option<StoredNote>, DatabaseError> {
        self.get_stored_note(note_id).await
    }

    /// Get all stored notes for a tag
    async fn get_stored_notes_for_tag(
        &self,
        tag: NoteTag,
    ) -> Result<Vec<StoredNote>, DatabaseError> {
        self.get_stored_notes_for_tag(tag).await
    }

    /// Record that a note has been fetched
    async fn record_fetched_note(
        &self,
        note_id: &NoteId,
        tag: NoteTag,
    ) -> Result<(), DatabaseError> {
        self.record_fetched_note(note_id, tag).await
    }

    /// Check if a note has been fetched before
    async fn note_fetched(&self, note_id: &NoteId) -> Result<bool, DatabaseError> {
        self.note_fetched(note_id).await
    }

    /// Get all fetched note IDs for a specific tag
    async fn get_fetched_notes_for_tag(&self, tag: NoteTag) -> Result<Vec<NoteId>, DatabaseError> {
        self.get_fetched_notes_for_tag(tag).await
    }

    /// Get database statistics
    async fn get_stats(&self) -> Result<DatabaseStats, DatabaseError> {
        self.get_stats().await
    }

    /// Clean up old data based on retention policy
    async fn cleanup_old_data(&self, retention_days: u32) -> Result<u64, DatabaseError> {
        self.cleanup_old_data(retention_days).await
    }
}

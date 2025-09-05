use wasm_bindgen_futures::JsFuture;

use crate::database::{DatabaseError, DatabaseStats};

mod js_bindings;
use js_bindings::{idxdb_cleanup_old_data, idxdb_get_stats};

mod models;
use models::DatabaseStatsIdxdbObject;

// Stats and maintenance operations
pub async fn get_stats() -> Result<DatabaseStats, DatabaseError> {
    let js_value = JsFuture::from(idxdb_get_stats())
        .await
        .map_err(|e| DatabaseError::Protocol(format!("Failed to get stats: {:?}", e)))?;

    let stats_data: DatabaseStatsIdxdbObject =
        serde_wasm_bindgen::from_value(js_value).map_err(|e| {
            DatabaseError::Encoding(format!("Failed to deserialize stats data: {:?}", e))
        })?;

    Ok(DatabaseStats {
        fetched_notes_count: stats_data.fetched_notes_count,
        stored_notes_count: stats_data.stored_notes_count,
        unique_tags_count: stats_data.unique_tags_count,
    })
}

pub async fn cleanup_old_data(retention_days: u32) -> Result<u64, DatabaseError> {
    let js_value = JsFuture::from(idxdb_cleanup_old_data(retention_days))
        .await
        .map_err(|e| DatabaseError::Protocol(format!("Failed to cleanup old data: {:?}", e)))?;

    let cleaned_count: u64 = serde_wasm_bindgen::from_value(js_value).map_err(|e| {
        DatabaseError::Encoding(format!("Failed to deserialize cleanup count: {:?}", e))
    })?;

    Ok(cleaned_count)
}

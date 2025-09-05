use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStatsIdxdbObject {
    pub fetched_notes_count: u64,
    pub stored_notes_count: u64,
    pub unique_tags_count: u64,
}

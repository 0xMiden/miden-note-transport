use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{js_sys, wasm_bindgen};

// Management IndexedDB Operations
#[wasm_bindgen(module = "/src/database/idxdb/js/mgmt.js")]
extern "C" {
    #[wasm_bindgen(js_name = getStats)]
    pub fn idxdb_get_stats() -> js_sys::Promise;

    #[wasm_bindgen(js_name = cleanupOldData)]
    pub fn idxdb_cleanup_old_data(retention_days: u32) -> js_sys::Promise;
}

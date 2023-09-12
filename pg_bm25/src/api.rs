use pgrx::{pg_sys::ItemPointerData, *};

use crate::manager::get_executor_manager;

#[pg_extern]
pub fn score_bm25(ctid: Option<ItemPointerData>) -> f32 {
    match ctid {
        Some(ctid) => get_executor_manager().get_score(ctid).unwrap_or(0.0f32),
        None => 0.0f32,
    }
}

#[pg_extern]
pub fn highlight_bm25(ctid: Option<ItemPointerData>, field_name: String) -> String {
    match ctid {
        Some(ctid) => get_executor_manager()
            .get_highlight(ctid, field_name)
            .unwrap_or("".to_string()),
        None => "".to_string(),
    }
}

use pgrx::*;

use crate::manager::get_executor_manager;

#[pg_extern]
pub fn score_bm25(ctid: Option<pg_sys::ItemPointerData>) -> f32 {
    match ctid {
        Some(ctid) => get_executor_manager().get_score(ctid).unwrap_or(0.0f32),
        None => 0.0f32,
    }
}

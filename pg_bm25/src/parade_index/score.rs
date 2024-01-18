use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct ParadeIndexScore {
    pub bm25: f32,
    pub key: i64,
}

// We do these custom trait impls, because we want these to be sorted so:
// - it's ordered descending by bm25 score.
// - in case of a tie, it's ordered by ascending key.

impl PartialEq for ParadeIndexScore {
    fn eq(&self, other: &Self) -> bool {
        self.bm25 == other.bm25 && self.key == other.key
    }
}

impl PartialOrd for ParadeIndexScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.bm25 == other.bm25 {
            other.key.partial_cmp(&self.key)
        } else {
            self.bm25.partial_cmp(&other.bm25)
        }
    }
}

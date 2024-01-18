use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct ParadeIndexScore {
    pub bm25: f32,
    pub key: i64,
}

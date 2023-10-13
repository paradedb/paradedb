use pgrx::*;
use serde::{Deserialize, Serialize};

#[derive(PostgresType, Serialize, Deserialize)]
pub struct Sparse {
    // Each entry is a tuple of (position, value), representing the position and value of a non-zero element
    pub entries: Vec<(i32, f64)>,
    // n is the length of the sparse vector
    pub n: i32,
}

#[derive(Clone)]
pub struct SparseIndex {
    pub name: String
}

pub struct ScanState {
    index: Option<SparseIndex>,
    curr: u32,
    n_results: u32,
    results: Option<pg_sys::ItemPointer>
}

impl SparseIndex {
    pub fn new(name: String) -> Self {
        // TODO: Create hnswlib index
        Self {
            name: name
        }
    }

    pub fn from_index_name(name: String) -> Self {
        // TODO: Retrieve hnswlib index
        Self {
            name: name
        }
    }

    pub fn insert(
        &mut self,
        heap_tid: pg_sys::ItemPointerData,
        vector: Sparse
    ) {
        // TODO: Insert sparse vector into HNSW index    
    }

    pub fn bulk_delete(
        &self,
        stats_binding: *mut pg_sys::IndexBulkDeleteResult,
        callback: pg_sys::IndexBulkDeleteCallback,
        callback_state: *mut ::std::os::raw::c_void,
    ) {
        // TODO: Delete from HNSW index
    }

    pub fn scan(&self) -> ScanState {
        // TODO: Return ScanState object which holds information for index scan
        ScanState {
            index: None,
            curr: 0,
            n_results: 0,
            results: None
        }
    }
}

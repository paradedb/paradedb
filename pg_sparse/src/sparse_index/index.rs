use pgrx::*;

use crate::sparse_index::sparse::Sparse;

#[derive(Debug, Clone)]
pub struct HNSWMeta {
    dim: usize,
    m: usize,
    ef_construction: usize,
    ef_search: usize,
}

#[derive(Debug, Clone)]
pub struct SparseIndex {
    pub index_name: String,
    pub meta: Option<HNSWMeta>
}

impl SparseIndex {
    pub fn new(index_name: String, meta: Option<HNSWMeta>) -> Self {
        info!("TODO: Create HNSW index");
        Self { index_name, meta }
    }

    pub fn from_index_name(name: String) -> Self {
        Self { index_name: name, meta: None }
    }

    pub fn insert(&mut self, sparse_vector: Sparse, heap_tid: pg_sys::ItemPointerData) {
        info!(
            "TODO: Insert {:?} with ID {:?} into index",
            sparse_vector, heap_tid
        );
    }

    pub fn search(&self, sparse_vector: &Sparse, k: usize) -> Vec<u64> {
        info!(
            "TODO: Implement HNSW search to return results sorted by ID {:?}",
            sparse_vector
        );
        vec![]
    }

    pub fn bulk_delete(
        &self,
        stats_binding: *mut pg_sys::IndexBulkDeleteResult,
        callback: pg_sys::IndexBulkDeleteCallback,
        callback_state: *mut ::std::os::raw::c_void,
    ) {
        info!("TODO: Implement delete");
    }
}

use pgrx::*;

use crate::sparse_index::sparse::Sparse;

pub struct SparseIndex {
    pub name: String,
}

impl SparseIndex {
    pub fn new(name: String) -> Self {
        info!("TODO: Create HNSW index");
        Self { name: name }
    }

    pub fn from_index_name(name: String) -> Self {
        info!("TODO: Retrieve HNSW index");
        Self { name: name }
    }

    pub fn insert(&mut self, sparse_vector: Sparse, heap_tid: pg_sys::ItemPointerData) {
        info!(
            "TODO: Insert {:?} with ID {:?} into index",
            sparse_vector, heap_tid
        );
    }

    pub fn search(self, sparse_vector: Sparse) -> Vec<pg_sys::ItemPointerData> {
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
        info!("TODO: Implement delete")
    }
}

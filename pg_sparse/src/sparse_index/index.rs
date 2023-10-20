use pgrx::*;

use crate::index_access::options::SparseOptions;
use crate::sparse_index::sparse::Sparse;

#[derive(Debug, Clone)]
pub struct SparseIndex {
    pub index_name: String,
}

impl SparseIndex {
    pub fn new(index: pg_sys::Relation) -> Self {
        let index_relation = unsafe { PgRelation::from_pg(index) };
        let index_name = index_relation.name().to_string();
        let rdopts: PgBox<SparseOptions> = if !index_relation.rd_options.is_null() {
            unsafe { PgBox::from_pg(index_relation.rd_options as *mut SparseOptions) }
        } else {
            let ops = unsafe { PgBox::<SparseOptions>::alloc0() };
            ops.into_pg_boxed()
        };

        info!("Creating SparseIndex with options {:?}", rdopts);

        Self { index_name }
    }

    pub fn from_index_name(index_name: String) -> Self {
        Self { index_name }
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

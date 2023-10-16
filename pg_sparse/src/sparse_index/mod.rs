use pgrx::pg_sys::{IndexBulkDeleteCallback, IndexBulkDeleteResult, ItemPointerData};
use pgrx::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fs::{create_dir_all, remove_dir_all};
use std::path::Path;
use tantivy::{
    query::{Query, QueryParser},
    schema::*,
    DocAddress, Document, Index, IndexSettings, Score, Searcher, SingleSegmentIndexWriter, Term,
};

use crate::index_access::options::ParadeOptions;

#[derive(PostgresType, Serialize, Deserialize, Debug)]
pub struct Sparse {
    // Each entry is a tuple of (position, value), representing the position and value of a non-zero element
    pub entries: Vec<(i32, f64)>,
    // n is the length of the sparse vector
    pub n: i32,
}

pub struct ScanState {
    index: Option<SparseIndex>,
    curr: u32,
    n_results: u32,
    results: Option<pg_sys::ItemPointer>,
}

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

    pub fn insert(&mut self, sparse_vector: Sparse, heap_tid: ItemPointerData) {
        info!(
            "TODO: Insert {:?} with ID {:?} into index",
            sparse_vector, heap_tid
        );
    }

    pub fn bulk_delete(
        &self,
        stats_binding: *mut IndexBulkDeleteResult,
        callback: IndexBulkDeleteCallback,
        callback_state: *mut ::std::os::raw::c_void,
    ) {
        info!("TODO: Implement delete")
    }

    pub fn scan(&self) -> ScanState {
        ScanState {
            index: None,
            curr: 0,
            n_results: 0,
            results: None,
        }
    }

    pub fn copy_tantivy_index(&self) -> tantivy::Index {
        let schema_builder = Schema::builder();
        let schema = schema_builder.build();
        Index::create_in_ram(schema.clone())
    }
}

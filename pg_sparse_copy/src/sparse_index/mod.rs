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
use crate::json::builder::JsonBuilder;

#[derive(PostgresType, Serialize, Deserialize)]
pub struct Sparse {
    // Each entry is a tuple of (position, value), representing the position and value of a non-zero element
    pub entries: Vec<(i32, f64)>,
    // n is the length of the sparse vector
    pub n: i32,
}

pub struct TantivyScanState {
    pub schema: Schema,
    pub query: Box<dyn Query>,
    pub query_parser: QueryParser,
    pub searcher: Searcher,
    pub iterator: *mut std::vec::IntoIter<(Score, DocAddress)>,
}

pub struct SparseIndex {
    pub name: String
}

impl SparseIndex {
    pub fn new(name: String) -> Self {
        Self {
            name: name
        }
    }

    pub fn from_index_name(name: String) -> Self {
        Self {
            name: name
        }
    }

    pub fn insert(
        &mut self,
        writer: &mut SingleSegmentIndexWriter,
        heap_tid: ItemPointerData,
        builder: JsonBuilder,
    ) {}

    pub fn bulk_delete(
        &self,
        stats_binding: *mut IndexBulkDeleteResult,
        callback: IndexBulkDeleteCallback,
        callback_state: *mut ::std::os::raw::c_void,
    ) {}

    pub fn scan(&self) -> TantivyScanState {
        let schema_builder = Schema::builder();
        let schema = schema_builder.build();
        let underlying_index = Index::create_in_ram(schema.clone());
        let reader = underlying_index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::Manual)
            .try_into()
            .expect("failed to create index reader");

        let searcher = reader.searcher();

        let query_parser = QueryParser::for_index(
            &underlying_index,
            schema.fields().map(|(field, _)| field).collect::<Vec<_>>(),
        );
        let empty_query = query_parser.parse_query("").unwrap();

        TantivyScanState {
            schema,
            query: empty_query,
            query_parser,
            searcher,
            iterator: std::ptr::null_mut(),
        }
    }

    pub fn copy_tantivy_index(&self) -> tantivy::Index {
        let schema_builder = Schema::builder();
        let schema = schema_builder.build();
        Index::create_in_ram(schema.clone())
    }
}

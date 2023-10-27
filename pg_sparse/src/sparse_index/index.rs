use hnswlib::Index;
use pgrx::*;
use std::ffi::{CStr, CString};
use std::fs::{create_dir_all, remove_dir_all};
use std::path::Path;

use crate::index_access::options::{
    SparseOptions, DEFAULT_EF_SEARCH, DEFAULT_EF_SEARCH_CONSTRUCTION, DEFAULT_M,
    DEFAULT_RANDOM_SEED,
};
use crate::sparse_index::sparse::Sparse;

const DEFAULT_INDEX_SIZE: usize = 1000;
const SPARSE_HNSW_DIR: &str = "sparse_hnsw";
const SPARSE_HNSW_FILENAME: &str = "index.bin";

pub fn create_index(index: pg_sys::Relation) -> Index {
    let index_relation = unsafe { PgRelation::from_pg(index) };
    let index_name = index_relation.name().to_string();
    let rdopts: PgBox<SparseOptions> = if !index_relation.rd_options.is_null() {
        unsafe { PgBox::from_pg(index_relation.rd_options as *mut SparseOptions) }
    } else {
        let mut ops = unsafe { PgBox::<SparseOptions>::alloc0() };
        ops.m = DEFAULT_M;
        ops.ef_search = DEFAULT_EF_SEARCH;
        ops.ef_construction = DEFAULT_EF_SEARCH_CONSTRUCTION;
        ops.random_seed = DEFAULT_RANDOM_SEED;
        ops.into_pg_boxed()
    };

    let mut hnsw_index = Index::new(
        DEFAULT_INDEX_SIZE,
        rdopts.m as usize,
        rdopts.ef_construction as usize,
        rdopts.random_seed as usize,
    );

    // Save index to disk
    let index_directory = get_index_directory(&index_name);
    let index_path = get_index_path(&index_name);

    let path = Path::new(&index_directory);
    if path.exists() {
        remove_dir_all(path).expect("Failed to remove sparse_hnsw directory");
    }
    create_dir_all(path).expect("Failed to create sparse_hnsw directory");

    hnsw_index.save_index(index_path);
    hnsw_index
}

pub fn bulk_delete(
    index_name: &str,
    stats_binding: *mut pg_sys::IndexBulkDeleteResult,
    callback: pg_sys::IndexBulkDeleteCallback,
    callback_state: *mut ::std::os::raw::c_void,
) {
    let mut index = from_index_name(index_name);
    let tids = index.get_current_labels();

    for tid in tids {
        if let Some(actual_callback) = callback {
            let should_delete =
                unsafe { actual_callback(tid as *mut pg_sys::ItemPointerData, callback_state) };

            if should_delete {
                index.mark_deleted(tid);
                unsafe {
                    (*stats_binding).tuples_removed += 1.0;
                }
            } else {
                unsafe {
                    (*stats_binding).num_index_tuples += 1.0;
                }
            }
        }
    }
}

pub fn from_index_name(index_name: &str) -> Index {
    let dir = get_index_directory(index_name);
    let index_path = format!("{}/{}", dir, SPARSE_HNSW_FILENAME);

    Index::load_index(index_path.clone())
}

pub fn get_index_directory(index_name: &str) -> String {
    format!("{}/{}", get_root_directory(), index_name)
}

pub fn get_index_path(index_name: &str) -> String {
    format!("{}/{}/{}", get_root_directory(), index_name, SPARSE_HNSW_FILENAME)
}

fn get_root_directory() -> String {
    unsafe {
        let option_name_cstr = CString::new("data_directory").expect("failed to create CString");
        let data_dir_str = String::from_utf8(
            CStr::from_ptr(pg_sys::GetConfigOptionByName(
                option_name_cstr.as_ptr(),
                std::ptr::null_mut(),
                true,
            ))
            .to_bytes()
            .to_vec(),
        )
        .expect("Failed to convert C string to Rust string");

        format!("{}/{}", data_dir_str, SPARSE_HNSW_DIR)
    }
}
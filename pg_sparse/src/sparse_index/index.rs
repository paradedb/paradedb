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
    let dir = get_data_directory(&index_name);
    let path = Path::new(&dir);
    if path.exists() {
        remove_dir_all(path).expect("Failed to remove sparse_hnsw directory");
    }
    create_dir_all(path).expect("Failed to create sparse_hnsw directory");

    let index_path = format!("{}/{}", dir, SPARSE_HNSW_FILENAME);
    hnsw_index.save_index(index_path.clone());

    info!("Created index {:?}", index_path.clone());

    hnsw_index
}

pub fn bulk_delete(
    index_name: String,
    stats_binding: *mut pg_sys::IndexBulkDeleteResult,
    callback: pg_sys::IndexBulkDeleteCallback,
    callback_state: *mut ::std::os::raw::c_void,
) {
    info!("TODO: Implement delete");
}

pub fn from_index_name(index_name: &str) -> Index {
    let dir = get_data_directory(&index_name);
    let index_path = format!("{}/{}", dir, SPARSE_HNSW_FILENAME);

    info!("Loading index from {:?}", index_path.clone());
    Index::load_index(index_path.clone())
}

pub fn get_data_directory(name: &str) -> String {
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

        format!("{}/{}/{}", data_dir_str, SPARSE_HNSW_DIR, name)
    }
}

// impl SparseIndex {
//     pub fn new(index: pg_sys::Relation) {
//         let index_relation = unsafe { PgRelation::from_pg(index) };
//         let index_name = index_relation.name().to_string();
//         let rdopts: PgBox<SparseOptions> = if !index_relation.rd_options.is_null() {
//             unsafe { PgBox::from_pg(index_relation.rd_options as *mut SparseOptions) }
//         } else {
//             let ops = unsafe { PgBox::<SparseOptions>::alloc0() };
//             ops.into_pg_boxed()
//         };

//         let mut hnsw_index = Index::new(
//             DEFAULT_INDEX_SIZE,
//             rdopts.m as usize,
//             rdopts.ef_construction as usize,
//             rdopts.random_seed as usize,
//         );

//         // Save index to disk
//         let dir = Self::get_data_directory(&index_name);
//         let path = Path::new(&dir);
//         if path.exists() {
//             remove_dir_all(path).expect("Failed to remove sparse_hnsw directory");
//         }

//         create_dir_all(path).expect("Failed to create sparse_hnsw directory");
//         hnsw_index.save_index(dir);
//     }

//     pub fn insert(index_name: &str, sparse_vector: Sparse, heap_tid: pg_sys::ItemPointerData) {
//         let tid = item_pointer_to_u64(heap_tid);
//         info!(
//             "TODO: Insert {:?} with ID {:?} into index",
//             sparse_vector, tid
//         );
//     }

//     pub fn search(index_name: &str, sparse_vector: &Sparse, k: usize) -> Vec<u64> {
//         info!(
//             "TODO: Implement HNSW search to return results sorted by ID {:?}",
//             sparse_vector
//         );
//         vec![]
//     }

//     pub fn bulk_delete(
//         index_name: String,
//         stats_binding: *mut pg_sys::IndexBulkDeleteResult,
//         callback: pg_sys::IndexBulkDeleteCallback,
//         callback_state: *mut ::std::os::raw::c_void,
//     ) {
//         info!("TODO: Implement delete");
//     }

//     fn get_data_directory(name: &str) -> String {
//         unsafe {
//             let option_name_cstr =
//                 CString::new("data_directory").expect("failed to create CString");
//             let data_dir_str = String::from_utf8(
//                 CStr::from_ptr(pg_sys::GetConfigOptionByName(
//                     option_name_cstr.as_ptr(),
//                     std::ptr::null_mut(),
//                     true,
//                 ))
//                 .to_bytes()
//                 .to_vec(),
//             )
//             .expect("Failed to convert C string to Rust string");

//             format!("{}/{}/{}", data_dir_str, SPARSE_HNSW_DIR, name)
//         }
//     }

//     fn from_index_name(index_name: &str) -> Index {
//         let dir = Self::get_data_directory(&index_name);
//         Index::load_index(dir)
//     }
// }

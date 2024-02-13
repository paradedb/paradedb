#![allow(clippy::too_many_arguments)]

mod create;
mod delete;
mod index;
mod insert;
mod plan;
mod scan;
mod update;
mod vacuum;

use pgrx::*;
use std::ptr::addr_of_mut;

use crate::tableam::create::*;
use crate::tableam::delete::*;
use crate::tableam::index::*;
use crate::tableam::insert::*;
use crate::tableam::plan::*;
use crate::tableam::scan::*;
use crate::tableam::update::*;
use crate::tableam::vacuum::*;

pub static mut DELTALAKE_TABLE_AM_ROUTINE: pg_sys::TableAmRoutine = pg_sys::TableAmRoutine {
    type_: pg_sys::NodeTag::T_TableAmRoutine,
    slot_callbacks: Some(deltalake_slot_callbacks),
    scan_begin: Some(deltalake_scan_begin),
    scan_end: Some(deltalake_scan_end),
    scan_rescan: Some(deltalake_scan_rescan),
    scan_getnextslot: Some(deltalake_scan_getnextslot),
    parallelscan_estimate: Some(deltalake_parallelscan_estimate),
    parallelscan_initialize: Some(deltalake_parallelscan_initialize),
    parallelscan_reinitialize: Some(deltalake_parallelscan_reinitialize),
    index_fetch_begin: Some(deltalake_index_fetch_begin),
    index_fetch_reset: Some(deltalake_index_fetch_reset),
    index_fetch_end: Some(deltalake_index_fetch_end),
    index_fetch_tuple: Some(deltalake_index_fetch_tuple),
    tuple_fetch_row_version: Some(deltalake_tuple_fetch_row_version),
    tuple_tid_valid: Some(deltalake_tuple_tid_valid),
    tuple_get_latest_tid: Some(deltalake_tuple_get_latest_tid),
    tuple_satisfies_snapshot: Some(deltalake_tuple_satisfies_snapshot),
    tuple_insert: Some(deltalake_tuple_insert),
    tuple_insert_speculative: Some(deltalake_tuple_insert_speculative),
    tuple_complete_speculative: Some(deltalake_tuple_complete_speculative),
    multi_insert: Some(deltalake_multi_insert),
    tuple_delete: Some(deltalake_tuple_delete),
    tuple_update: Some(deltalake_tuple_update),
    tuple_lock: Some(deltalake_tuple_lock),
    finish_bulk_insert: Some(deltalake_finish_bulk_insert),
    relation_nontransactional_truncate: Some(deltalake_relation_nontransactional_truncate),
    relation_copy_data: Some(deltalake_relation_copy_data),
    relation_copy_for_cluster: Some(deltalake_relation_copy_for_cluster),
    relation_vacuum: Some(deltalake_relation_vacuum),
    scan_analyze_next_block: Some(deltalake_scan_analyze_next_block),
    scan_analyze_next_tuple: Some(deltalake_scan_analyze_next_tuple),
    index_build_range_scan: Some(deltalake_index_build_range_scan),
    index_validate_scan: Some(deltalake_index_validate_scan),
    relation_size: Some(deltalake_relation_size),
    relation_needs_toast_table: Some(deltalake_relation_needs_toast_table),
    relation_estimate_size: Some(deltalake_relation_estimate_size),
    scan_bitmap_next_block: Some(deltalake_scan_bitmap_next_block),
    scan_bitmap_next_tuple: Some(deltalake_scan_bitmap_next_tuple),
    scan_sample_next_block: Some(deltalake_scan_sample_next_block),
    scan_sample_next_tuple: Some(deltalake_scan_sample_next_tuple),
    #[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
    relation_set_new_filenode: Some(deltalake_relation_set_new_filenode),
    #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
    relation_toast_am: Some(deltalake_relation_toast_am),
    #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
    relation_fetch_toast_slice: Some(deltalake_relation_fetch_toast_slice),
    #[cfg(any(feature = "pg12", feature = "pg13"))]
    compute_xid_horizon_for_tuples: Some(deltalake_compute_xid_horizon_for_tuples),
    #[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
    scan_set_tidrange: Some(deltalake_scan_set_tidrange),
    #[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
    scan_getnextslot_tidrange: Some(deltalake_scan_getnextslot_tidrange),
    #[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
    index_delete_tuples: Some(deltalake_index_delete_tuples),
    #[cfg(feature = "pg16")]
    relation_set_new_filelocator: Some(deltalake_relation_set_new_filelocator),
};

#[pg_guard]
#[no_mangle]
extern "C" fn pg_finfo_deltalake_tableam_handler() -> &'static pg_sys::Pg_finfo_record {
    const V1_API: pg_sys::Pg_finfo_record = pg_sys::Pg_finfo_record { api_version: 1 };
    &V1_API
}

extension_sql!(
    r#"
    CREATE FUNCTION deltalake_tableam_handler(internal)
    RETURNS table_am_handler AS 'MODULE_PATHNAME', 'deltalake_tableam_handler' LANGUAGE C STRICT;
    CREATE ACCESS METHOD parquet TYPE TABLE HANDLER deltalake_tableam_handler;
    COMMENT ON ACCESS METHOD parquet IS 'ParadeDB parquet table access method';
    "#,
    name = "deltalake_tableam_handler"
);
#[no_mangle]
#[pg_guard]
extern "C" fn deltalake_tableam_handler(
    _fcinfo: pg_sys::FunctionCallInfo,
) -> *mut pg_sys::TableAmRoutine {
    unsafe { addr_of_mut!(DELTALAKE_TABLE_AM_ROUTINE) }
}

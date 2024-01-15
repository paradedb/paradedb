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

pub static mut ANALYTICS_TABLE_AM_ROUTINE: pg_sys::TableAmRoutine = pg_sys::TableAmRoutine {
    type_: pg_sys::NodeTag::T_TableAmRoutine,
    slot_callbacks: Some(analytics_slot_callbacks),
    scan_begin: Some(analytics_scan_begin),
    scan_end: Some(analytics_scan_end),
    scan_rescan: Some(analytics_scan_rescan),
    scan_getnextslot: Some(analytics_scan_getnextslot),
    parallelscan_estimate: Some(analytics_parallelscan_estimate),
    parallelscan_initialize: Some(analytics_parallelscan_initialize),
    parallelscan_reinitialize: Some(analytics_parallelscan_reinitialize),
    index_fetch_begin: Some(analytics_index_fetch_begin),
    index_fetch_reset: Some(analytics_index_fetch_reset),
    index_fetch_end: Some(analytics_index_fetch_end),
    index_fetch_tuple: Some(analytics_index_fetch_tuple),
    tuple_fetch_row_version: Some(analytics_tuple_fetch_row_version),
    tuple_tid_valid: Some(analytics_tuple_tid_valid),
    tuple_get_latest_tid: Some(analytics_tuple_get_latest_tid),
    tuple_satisfies_snapshot: Some(analytics_tuple_satisfies_snapshot),
    tuple_insert: Some(analytics_tuple_insert),
    tuple_insert_speculative: Some(analytics_tuple_insert_speculative),
    tuple_complete_speculative: Some(analytics_tuple_complete_speculative),
    multi_insert: Some(analytics_multi_insert),
    tuple_delete: Some(analytics_tuple_delete),
    tuple_update: Some(analytics_tuple_update),
    tuple_lock: Some(analytics_tuple_lock),
    finish_bulk_insert: Some(analytics_finish_bulk_insert),
    relation_nontransactional_truncate: Some(analytics_relation_nontransactional_truncate),
    relation_copy_data: Some(analytics_relation_copy_data),
    relation_copy_for_cluster: Some(analytics_relation_copy_for_cluster),
    relation_vacuum: Some(analytics_relation_vacuum),
    scan_analyze_next_block: Some(analytics_scan_analyze_next_block),
    scan_analyze_next_tuple: Some(analytics_scan_analyze_next_tuple),
    index_build_range_scan: Some(analytics_index_build_range_scan),
    index_validate_scan: Some(analytics_index_validate_scan),
    relation_size: Some(analytics_relation_size),
    relation_needs_toast_table: Some(analytics_relation_needs_toast_table),
    relation_estimate_size: Some(analytics_relation_estimate_size),
    scan_bitmap_next_block: Some(analytics_scan_bitmap_next_block),
    scan_bitmap_next_tuple: Some(analytics_scan_bitmap_next_tuple),
    scan_sample_next_block: Some(analytics_scan_sample_next_block),
    scan_sample_next_tuple: Some(analytics_scan_sample_next_tuple),
    #[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
    relation_set_new_filenode: Some(analytics_relation_set_new_filenode),
    #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
    relation_toast_am: Some(analytics_relation_toast_am),
    #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
    relation_fetch_toast_slice: Some(analytics_relation_fetch_toast_slice),
    #[cfg(any(feature = "pg12", feature = "pg13"))]
    compute_xid_horizon_for_tuples: Some(analytics_compute_xid_horizon_for_tuples),
    #[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
    scan_set_tidrange: Some(analytics_scan_set_tidrange),
    #[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
    scan_getnextslot_tidrange: Some(analytics_scan_getnextslot_tidrange),
    #[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
    index_delete_tuples: Some(analytics_index_delete_tuples),
    #[cfg(feature = "pg16")]
    relation_set_new_filelocator: Some(analytics_relation_set_new_filelocator),
};

#[pg_guard]
#[no_mangle]
extern "C" fn pg_finfo_analytics_tableam_handler() -> &'static pg_sys::Pg_finfo_record {
    const V1_API: pg_sys::Pg_finfo_record = pg_sys::Pg_finfo_record { api_version: 1 };
    &V1_API
}

extension_sql!(
    r#"
    CREATE FUNCTION analytics_tableam_handler(internal)
    RETURNS table_am_handler AS 'MODULE_PATHNAME', 'analytics_tableam_handler' LANGUAGE C STRICT;
    CREATE ACCESS METHOD analytics TYPE TABLE HANDLER analytics_tableam_handler;
    COMMENT ON ACCESS METHOD analytics IS 'analytics table access method';
    "#,
    name = "analytics_tableam_handler"
);
#[no_mangle]
#[pg_guard]
extern "C" fn analytics_tableam_handler(
    _fcinfo: pg_sys::FunctionCallInfo,
) -> *mut pg_sys::TableAmRoutine {
    unsafe { addr_of_mut!(ANALYTICS_TABLE_AM_ROUTINE) }
}

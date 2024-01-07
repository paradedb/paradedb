mod build;
mod ignored;
pub mod insert;

use pgrx::*;

use crate::tableam::build::*;
use crate::tableam::ignored::*;
use crate::tableam::insert::*;

pub static mut MEM_TABLE_AM_ROUTINE: pg_sys::TableAmRoutine = pg_sys::TableAmRoutine {
    type_: pg_sys::NodeTag::T_TableAmRoutine,

    slot_callbacks: Some(memam_slot_callbacks),

    scan_begin: Some(memam_scan_begin),
    scan_end: Some(memam_scan_end),
    scan_rescan: Some(memam_scan_rescan),
    scan_getnextslot: Some(memam_scan_getnextslot),
    scan_set_tidrange: Some(memam_scan_set_tidrange),
    scan_getnextslot_tidrange: Some(memam_scan_getnextslot_tidrange),

    parallelscan_estimate: Some(memam_parallelscan_estimate),
    parallelscan_initialize: Some(memam_parallelscan_initialize),
    parallelscan_reinitialize: Some(memam_parallelscan_reinitialize),

    index_fetch_begin: Some(memam_index_fetch_begin),
    index_fetch_reset: Some(memam_index_fetch_reset),
    index_fetch_end: Some(memam_index_fetch_end),
    index_fetch_tuple: Some(memam_index_fetch_tuple),
    tuple_fetch_row_version: Some(memam_tuple_fetch_row_version),
    tuple_tid_valid: Some(memam_tuple_tid_valid),
    tuple_get_latest_tid: Some(memam_tuple_get_latest_tid),
    tuple_satisfies_snapshot: Some(memam_tuple_satisfies_snapshot),
    index_delete_tuples: Some(memam_index_delete_tuples),
    tuple_insert: Some(memam_tuple_insert),
    tuple_insert_speculative: Some(memam_tuple_insert_speculative),
    tuple_complete_speculative: Some(memam_tuple_complete_speculative),
    multi_insert: Some(memam_multi_insert),
    tuple_delete: Some(memam_tuple_delete),
    tuple_update: Some(memam_tuple_update),
    tuple_lock: Some(memam_tuple_lock),
    finish_bulk_insert: Some(memam_finish_bulk_insert),
    relation_set_new_filenode: Some(memam_relation_set_new_filenode),
    relation_nontransactional_truncate: Some(memam_relation_nontransactional_truncate),
    relation_copy_data: Some(memam_relation_copy_data),
    relation_copy_for_cluster: Some(memam_relation_copy_for_cluster),
    relation_vacuum: Some(memam_relation_vacuum),
    scan_analyze_next_block: Some(memam_scan_analyze_next_block),
    scan_analyze_next_tuple: Some(memam_scan_analyze_next_tuple),
    index_build_range_scan: Some(memam_index_build_range_scan),
    index_validate_scan: Some(memam_index_validate_scan),
    relation_size: Some(memam_relation_size),
    relation_needs_toast_table: Some(memam_relation_needs_toast_table),
    relation_toast_am: Some(memam_relation_toast_am),
    relation_fetch_toast_slice: Some(memam_relation_fetch_toast_slice),
    relation_estimate_size: Some(memam_relation_estimate_size),
    scan_bitmap_next_block: Some(memam_scan_bitmap_next_block),
    scan_bitmap_next_tuple: Some(memam_scan_bitmap_next_tuple),
    scan_sample_next_block: Some(memam_scan_sample_next_block),
    scan_sample_next_tuple: Some(memam_scan_sample_next_tuple),
};

extension_sql!(
    r#"
CREATE FUNCTION mem_tableam_handler(internal) RETURNS table_am_handler AS 'MODULE_PATHNAME', 'mem_tableam_handler' LANGUAGE C STRICT;
CREATE ACCESS METHOD mem TYPE TABLE HANDLER mem_tableam_handler;
COMMENT ON ACCESS METHOD mem IS 'mem table access method';
"#,
    name = "mem_tableam_handler"
);
#[no_mangle]
#[pg_guard]
extern "C" fn mem_tableam_handler(
    _fcinfo: pg_sys::FunctionCallInfo,
) -> *mut pg_sys::TableAmRoutine {
    unsafe { &mut MEM_TABLE_AM_ROUTINE as *mut pg_sys::TableAmRoutine }
}

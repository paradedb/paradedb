mod build;
mod funcs;
mod utils;

use pgrx::*;

use crate::table_access::build::*;
use crate::table_access::funcs::*;

extension_sql!(
    r#"
CREATE FUNCTION mem_tableam_handler(internal) RETURNS table_am_handler AS 'MODULE_PATHNAME', 'mem_tableam_handler' LANGUAGE C STRICT;
CREATE ACCESS METHOD mem TYPE TABLE HANDLER mem_tableam_handler;
COMMENT ON ACCESS METHOD mem IS 'mem table access method';
"#,
    name = "mem_tableam_handler"
);
#[no_mangle]
extern "C" fn mem_tableam_handler(
    _fcinfo: pg_sys::FunctionCallInfo,
) -> *mut pg_sys::TableAmRoutine {
    let mut amroutine =
        unsafe { PgBox::<pg_sys::TableAmRoutine>::alloc_node(pg_sys::NodeTag::T_TableAmRoutine) };

    amroutine.type_ = pg_sys::NodeTag::T_TableAmRoutine;

    amroutine.slot_callbacks = Some(memam_slot_callbacks);

    amroutine.scan_begin = Some(memam_scan_begin);
    amroutine.scan_end = Some(memam_scan_end);
    amroutine.scan_rescan = Some(memam_scan_rescan);
    amroutine.scan_getnextslot = Some(memam_scan_getnextslot);
    amroutine.scan_set_tidrange = Some(memam_scan_set_tidrange);
    amroutine.scan_getnextslot_tidrange = Some(memam_scan_getnextslot_tidrange);

    amroutine.parallelscan_estimate = Some(memam_parallelscan_estimate);
    amroutine.parallelscan_initialize = Some(memam_parallelscan_initialize);
    amroutine.parallelscan_reinitialize = Some(memam_parallelscan_reinitialize);

    amroutine.index_fetch_begin = Some(memam_index_fetch_begin);
    amroutine.index_fetch_reset = Some(memam_index_fetch_reset);
    amroutine.index_fetch_end = Some(memam_index_fetch_end);
    amroutine.index_fetch_tuple = Some(memam_index_fetch_tuple);
    amroutine.tuple_fetch_row_version = Some(memam_tuple_fetch_row_version);
    amroutine.tuple_tid_valid = Some(memam_tuple_tid_valid);
    amroutine.tuple_get_latest_tid = Some(memam_tuple_get_latest_tid);
    amroutine.tuple_satisfies_snapshot = Some(memam_tuple_satisfies_snapshot);
    amroutine.index_delete_tuples = Some(memam_index_delete_tuples);
    amroutine.tuple_insert = Some(memam_tuple_insert);
    amroutine.tuple_insert_speculative = Some(memam_tuple_insert_speculative);
    amroutine.tuple_complete_speculative = Some(memam_tuple_complete_speculative);
    amroutine.multi_insert = Some(memam_multi_insert);
    amroutine.tuple_delete = Some(memam_tuple_delete);
    amroutine.tuple_update = Some(memam_tuple_update);
    amroutine.tuple_lock = Some(memam_tuple_lock);
    amroutine.finish_bulk_insert = Some(memam_finish_bulk_insert);
    amroutine.relation_set_new_filenode = Some(memam_relation_set_new_filenode);
    amroutine.relation_nontransactional_truncate = Some(memam_relation_nontransactional_truncate);
    amroutine.relation_copy_data = Some(memam_relation_copy_data);
    amroutine.relation_copy_for_cluster = Some(memam_relation_copy_for_cluster);
    amroutine.relation_vacuum = Some(memam_relation_vacuum);
    amroutine.scan_analyze_next_block = Some(memam_scan_analyze_next_block);
    amroutine.scan_analyze_next_tuple = Some(memam_scan_analyze_next_tuple);
    amroutine.index_build_range_scan = Some(memam_index_build_range_scan);
    amroutine.index_validate_scan = Some(memam_index_validate_scan);
    amroutine.relation_size = Some(memam_relation_size);
    amroutine.relation_needs_toast_table = Some(memam_relation_needs_toast_table);
    amroutine.relation_toast_am = Some(memam_relation_toast_am);
    amroutine.relation_fetch_toast_slice = Some(memam_relation_fetch_toast_slice);
    amroutine.relation_estimate_size = Some(memam_relation_estimate_size);
    amroutine.scan_bitmap_next_block = Some(memam_scan_bitmap_next_block);
    amroutine.scan_bitmap_next_tuple = Some(memam_scan_bitmap_next_tuple);
    amroutine.scan_sample_next_block = Some(memam_scan_sample_next_block);
    amroutine.scan_sample_next_tuple = Some(memam_scan_sample_next_tuple);

    amroutine.into_pg_boxed().as_ptr()
}

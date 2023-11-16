use pgrx::prelude::*;
use shared::logs::ParadeLogsGlobal;
use shared::telemetry;

// This is a flag that can be set by the user in a session to enable logs.
// You need to initialize this in every extension that uses `plog!`.
static PARADE_LOGS_GLOBAL: ParadeLogsGlobal = ParadeLogsGlobal::new("pg_columnar");

pgrx::pg_module_magic!();

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
    amroutine.slot_callbacks = None;
    amroutine.scan_begin = None;
    amroutine.scan_end = None;
    amroutine.scan_rescan = None;
    amroutine.scan_getnextslot = None;
    amroutine.scan_set_tidrange = None;
    amroutine.scan_getnextslot_tidrange = None;
    amroutine.parallelscan_estimate = None;
    amroutine.parallelscan_initialize = None;
    amroutine.parallelscan_reinitialize = None;
    amroutine.index_fetch_begin = None;
    amroutine.index_fetch_reset = None;
    amroutine.index_fetch_end = None;
    amroutine.index_fetch_tuple = None;
    amroutine.tuple_fetch_row_version = None;
    amroutine.tuple_tid_valid = None;
    amroutine.tuple_get_latest_tid = None;
    amroutine.tuple_satisfies_snapshot = None;
    amroutine.index_delete_tuples = None;
    amroutine.tuple_insert = None;
    amroutine.tuple_insert_speculative = None;
    amroutine.tuple_complete_speculative = None;
    amroutine.multi_insert = None;
    amroutine.tuple_delete = None;
    amroutine.tuple_update = None;
    amroutine.tuple_lock = None;
    amroutine.finish_bulk_insert = None;
    amroutine.relation_set_new_filenode = None;
    amroutine.relation_nontransactional_truncate = None;
    amroutine.relation_copy_data = None;
    amroutine.relation_copy_for_cluster = None;
    amroutine.relation_vacuum = None;
    amroutine.scan_analyze_next_block = None;
    amroutine.scan_analyze_next_tuple = None;
    amroutine.index_build_range_scan = None;
    amroutine.index_validate_scan = None;
    amroutine.relation_size = None;
    amroutine.relation_needs_toast_table = None;
    amroutine.relation_toast_am = None;
    amroutine.relation_fetch_toast_slice = None;
    amroutine.relation_estimate_size = None;
    amroutine.scan_bitmap_next_block = None;
    amroutine.scan_bitmap_next_tuple = None;
    amroutine.scan_sample_next_block = None;
    amroutine.scan_sample_next_tuple = None;

    amroutine.into_pg_boxed().as_ptr()
}

#[no_mangle]
extern "C" fn pg_finfo_mem_tableam_handler() -> &'static pg_sys::Pg_finfo_record {
    const V1_API: pg_sys::Pg_finfo_record = pg_sys::Pg_finfo_record { api_version: 1 };
    &V1_API
}

// initializes telemetry
#[allow(clippy::missing_safety_doc)]
#[allow(non_snake_case)]
#[pg_guard]
pub unsafe extern "C" fn _PG_init() {
    telemetry::posthog::init("pg_columnar Deployment");
    PARADE_LOGS_GLOBAL.init();
}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    #[pgrx::pg_test]
    fn test_parade_logs() {
        shared::test_plog!("pg_columnar");
    }
}

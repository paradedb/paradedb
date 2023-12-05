use pg_sys::{
    self, planner_hook, standard_ExecutorRun, standard_planner, ExecutorRun_hook, Node, NodeTag,
    ParamListInfoData, PlannedStmt, Query, QueryDesc, SeqScan,
};
use pgrx::prelude::*;
use shared::logs::ParadeLogsGlobal;
use shared::telemetry;

use crate::to_substrait::transform_seqscan_to_substrait;

mod am_funcs;
use am_funcs::*;
use lazy_static::lazy_static;
use datafusion::prelude::SessionContext;

mod to_substrait;
use substrait::proto::RelRoot;
use substrait::proto::Rel;
use substrait::proto::rel::RelType::Read;
use substrait::proto::plan_rel::RelType::Root;
use substrait::proto::PlanRel;

use pgrx::pg_sys::get_am_name;
use core::ffi::CStr;
use pgrx::pg_sys::ScanState;
use pgrx::pg_sys::LookupFuncName;
use pgrx::pg_sys::list_make1_impl;
use pgrx::pg_sys::INTERNALOID;
use pgrx::pg_sys::makeString;
use pgrx::pg_sys::ListCell;
use std::ffi::CString;
use core::ffi::c_void;


pgrx::pg_module_magic!();

extension_sql_file!("../sql/_bootstrap.sql");

// This is a flag that can be set by the user in a session to enable logs.
// You need to initialize this in every extension that uses `plog!`.
static PARADE_LOGS_GLOBAL: ParadeLogsGlobal = ParadeLogsGlobal::new("pg_columnar");

// let's try adding the session context globally for now so we can retain info about our tables
lazy_static! {
    static ref CONTEXT: SessionContext = SessionContext::new();
}

extern "C" fn columnar_planner(
    parse: *mut Query,
    query_string: *const i8,
    cursor_options: i32,
    bound_params: *mut ParamListInfoData,
) -> *mut PlannedStmt {
    // Log the entry into the custom planner
    info!("Entering columnar_planner");

    // Log details about the query, if needed
    if !query_string.is_null() {
        let query_str = unsafe { std::ffi::CStr::from_ptr(query_string) }.to_string_lossy();
        info!("Query string: {}", query_str);
    }

    unsafe {
        let result = standard_planner(parse, query_string, cursor_options, bound_params);
        // Log the fact that standard planner was called
        info!("Standard planner called");

        // TODO: iterate through result and convert to substrait plan - first iterate through plan when UDFs are involved and determine if behavior is correct
        result
    }
}

unsafe fn describe_nodes(tree: *mut pg_sys::Plan) {
    info!("Describing plan");
    // Imitate ExplainNode for recursive plan scanning behavior
    let node_tag = (*tree).type_;
    info!("Node tag {:?}", node_tag);
    if !(*tree).lefttree.is_null() {
        info!("Left tree");
        describe_nodes((*tree).lefttree);
    }
    if !(*tree).righttree.is_null() {
        info!("Right tree");
        describe_nodes((*tree).righttree);
    }
}

unsafe extern "C" fn columnar_executor_run(
    query_desc: *mut QueryDesc,
    direction: i32,
    count: u64,
    execute_once: bool,
) {
    // Log the entry into the custom planner
    info!("Entering columnar_executor_run");

    // Imitate ExplainNode for recursive plan scanning behavior
    let ps = (*query_desc).plannedstmt;
    let plan: *mut pg_sys::Plan = (*ps).planTree;
    describe_nodes(plan);

    let node = plan as *mut Node;
    let node_tag = (*node).type_;
    let rtable = (*ps).rtable;

    // Create default Substrait plan
    let mut splan = substrait::proto::Plan::default();
    // TODO: fill out the plan
    let mut sget = substrait::proto::ReadRel::default();

    let planstate = (*query_desc).planstate;

    match node_tag {
        NodeTag::T_SeqScan => {
            info!("match T_SeqScan");
            // Check if the table is using our table AM before running our custom logic
            let scanstate = planstate as *mut ScanState;
            let rel = (*scanstate).ss_currentRelation;
            let am_oid = (*rel).rd_amhandler;

            // let amTup = SearchSysCache1(SysCacheIdentifier_AMOID.try_into().unwrap(), Datum::from(am_oid));
            // info!("here 2");
            // let amForm = heap_tuple_get_struct::<FormData_pg_am>(amTup);
            // info!("here 3");

            info!("{}", am_oid);
            let argtype = [INTERNALOID];
            info!("here 1");
            let handlername_cstr = CString::new("mem").unwrap();
            info!("here 2");
            let handlername_ptr = handlername_cstr.as_ptr() as *mut i8;
            info!("here 3");
            let mut handlername_listcell = ListCell::default();
            info!("here 4");
            handlername_listcell.ptr_value = makeString(handlername_ptr) as *mut c_void;
            info!("here 5");
            let handlername_list = list_make1_impl(NodeTag::T_List, handlername_listcell);
            info!("here 6");
            let memhandler_oid = LookupFuncName(handlername_list, 1, argtype.as_ptr(), false);
            info!("{}", memhandler_oid);
            // let handler_name = CStr::from_ptr(get_am_name(am_oid)).to_string_lossy().into_owned();
            // info!("{}", handler_name);

            // if name_data_to_str(&(*(*amForm)).amname) != "mem" {
            //     standard_ExecutorRun(query_desc, direction, count, execute_once);
            // }

            // ReleaseSysCache(amTup);
            info!("released");

            if let Err(e) = transform_seqscan_to_substrait(ps, &mut sget) {
                error!("Error transforming SeqScan to Substrait: {}", e);
            }
        }
        _ => {
            // TODO: Add missing types
        }
    }
    // splan.relations = Some(RelType::Root(RelRoot { input: Some(sget), names: $(names of output fields) }
    // splan.extensions and extension_uris should be filled in while we're transforming
    // TODO: print out the plan so we can confirm it
    // TODO: get the names
    splan.relations = vec![PlanRel{ rel_type: Some(Root(RelRoot { input: Some(Rel{ rel_type: Some(Read(Box::new(sget))) }), names: vec![]}))}];

    unsafe {
        // TODO: instead of standard_ExecutorRun, should pass substrait plan to DataFusion and process results
        standard_ExecutorRun(query_desc, direction, count, execute_once);
        // let logical_plan = from_substrait_plan(ctx, splan);
        // let results = CONTEXT.execute_logical_plan(logical_plan);

        // let sendTuples = (query_desc.operation == CMD_SELECT ||
        //           query_desc.plannedstmt.hasReturning);

        // if (sendTuples) {
        //     let dest = query_desc.dest;
        //     dest.rStartup(dest, operation, query_desc.tupDesc);
        //     // TODO: is this where we should conver the results to tuples and pass to dest?
        //     dest.receiveSlot(/* SLOT */, dest);
        // }

        // Log the fact that standard planner was called
        info!("Standard ExecutorRun called");
    }
}

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
    info!("mem_tableam_handler");
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

#[no_mangle]
extern "C" fn pg_finfo_mem_tableam_handler() -> &'static pg_sys::Pg_finfo_record {
    // TODO in the blog post he initializes the database here. Does our session context go here?
    const V1_API: pg_sys::Pg_finfo_record = pg_sys::Pg_finfo_record { api_version: 1 };
    &V1_API
}

// initializes telemetry
#[allow(clippy::missing_safety_doc)]
#[allow(non_snake_case)]
#[pg_guard]
// #[no_mangle]
pub extern "C" fn _PG_init() {
    telemetry::posthog::init("pg_columnar deployment");
    PARADE_LOGS_GLOBAL.init();
    unsafe {
        planner_hook = Some(columnar_planner as _); // Corrected cast
        ExecutorRun_hook = Some(columnar_executor_run as _);
    }
}

// We have this here in order to force the hook during CREATE EXTENSION
// This is probably avoided if we LOAD the extension instead?
#[pg_extern]
fn hello_pg_planner() -> &'static str {
    "Hello, pg_planner"
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_hello_pg_planner() {
        assert_eq!("Hello, pg_planner", crate::hello_pg_planner());
    }
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

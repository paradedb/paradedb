#![allow(unused)]
#![allow(non_snake_case)]

use pg_sys::{
    self, planner_hook, standard_ExecutorRun, standard_planner, ExecutorRun_hook, Node, NodeTag,
    ParamListInfoData, PlannedStmt, Query, QueryDesc, SeqScan,
};

use lazy_static::lazy_static;
use pgrx::once_cell::sync::Lazy;
use pgrx::prelude::*;
use shared::logs::ParadeLogsGlobal;
use shared::telemetry;

use crate::to_substrait::transform_seqscan_to_substrait;

mod to_substrait;
use substrait::proto::RelRoot;
use substrait::proto::Rel;
use substrait::proto::rel::RelType::Read;
use substrait::proto::plan_rel::RelType::Root;
use substrait::proto::PlanRel;

use std::ffi::CString;
use pgrx::pg_sys::ScanState;
use pgrx::pg_sys::get_am_name;
use pgrx::pg_sys::get_am_oid;
use pgrx::pg_sys::GetTableAmRoutine;
use pgrx::pg_sys::ReleaseSysCache;
use pgrx::pg_sys::FormData_pg_am;
use pgrx::pg_sys::heap_tuple_get_struct;
use pgrx::pg_sys::Datum;
use pgrx::pg_sys::SysCacheIdentifier_AMOID;
use pgrx::pg_sys::SearchSysCache1;

mod datafusion;
mod table_access;


pgrx::pg_module_magic!();

extension_sql_file!("../sql/_bootstrap.sql");

// This is a flag that can be set by the user in a session to enable logs.
// You need to initialize this in every extension that uses `plog!`.
static PARADE_LOGS_GLOBAL: ParadeLogsGlobal = ParadeLogsGlobal::new("pg_columnar");

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

    info!("{:?}", node_tag);

    match node_tag {
        NodeTag::T_SeqScan => {
            info!("match T_SeqScan");
            // Check if the table is using our table AM before running our custom logic
            let scanstate = planstate as *mut ScanState;
            let rel = (*scanstate).ss_currentRelation;
            let am_handler = (*rel).rd_amhandler;

            let handlername_cstr = CString::new("mem").unwrap();
            let handlername_ptr = handlername_cstr.as_ptr() as *mut i8;
            let memam_oid = get_am_oid(handlername_ptr, true);
            let amTup = SearchSysCache1(SysCacheIdentifier_AMOID.try_into().unwrap(), Datum::from(memam_oid));
            let amForm = heap_tuple_get_struct::<FormData_pg_am>(amTup);
            let memhandler_oid = (*amForm).amhandler;

            ReleaseSysCache(amTup);

            if am_handler == memhandler_oid {
                standard_ExecutorRun(query_desc, direction, count, execute_once);
                info!("Standard ExecutorRun called");
                return;
            }

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

#[no_mangle]
extern "C" fn pg_finfo_mem_tableam_handler() -> &'static pg_sys::Pg_finfo_record {
    // TODO in the blog post he initializes the database here. Does our session context go here?
    const V1_API: pg_sys::Pg_finfo_record = pg_sys::Pg_finfo_record { api_version: 1 };
    &V1_API
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
    fn test_hello_pg_planner() {
        assert_eq!("Hello, pg_planner", crate::hello_pg_planner());
    }

    #[pgrx::pg_test]
    fn test_parade_logs() {
        shared::test_plog!("pg_columnar");
    }
}

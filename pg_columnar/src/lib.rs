use pg_sys::{
    self, planner_hook, standard_ExecutorRun, standard_planner, ExecutorRun_hook, Node, NodeTag,
    ParamListInfoData, PlannedStmt, Query, QueryDesc, SeqScan,
};
use pgrx::prelude::*;
use shared::logs::ParadeLogsGlobal;
use shared::telemetry;

use crate::to_substrait::transform_seqscan_to_substrait;

mod to_substrait;

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
    let node = plan as *mut Node;
    let node_tag = (*node).type_;
    let rtable = (*ps).rtable;

    // Create default Substrait plan
    let mut sget = substrait::proto::ReadRel::default();

    match node_tag {
        NodeTag::T_SeqScan => {
            if let Err(e) = transform_seqscan_to_substrait(ps, &mut sget) {
                error!("Error transforming SeqScan to Substrait: {}", e);
            }
        }
        _ => {
            // TODO: Add missing types
        }
    }

    unsafe {
        standard_ExecutorRun(query_desc, direction, count, execute_once);
        // Log the fact that standard planner was called
        info!("Standard ExecutorRun called");
    }
}

// initializes telemetry
#[allow(clippy::missing_safety_doc)]
#[allow(non_snake_case)]
#[pg_guard]
pub unsafe extern "C" fn _PG_init() {
    telemetry::posthog::init("pg_columnar deployment");
    PARADE_LOGS_GLOBAL.init();
    planner_hook = Some(columnar_planner as _); // Corrected cast
    ExecutorRun_hook = Some(columnar_executor_run as _);
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {}

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

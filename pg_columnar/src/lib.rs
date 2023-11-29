use pgrx::prelude::*;
use pg_sys::{self, PlannedStmt, Query, standard_planner, planner_hook, ParamListInfoData, ExecutorRun_hook, QueryDesc, standard_ExecutorRun, Node, NodeTag};
use shared::logs::ParadeLogsGlobal;
use shared::telemetry;

pgrx::pg_module_magic!();

extension_sql_file!("../sql/_bootstrap.sql");

// This is a flag that can be set by the user in a session to enable logs.
// You need to initialize this in every extension that uses `plog!`.
static PARADE_LOGS_GLOBAL: ParadeLogsGlobal = ParadeLogsGlobal::new("pg_columnar");

extern "C" fn columnar_planner(
    parse: *mut Query,
    query_string: *const i8,
    cursor_options: i32,
    bound_params: *mut ParamListInfoData
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

unsafe extern "C" fn columnar_executor_run(query_desc: *mut QueryDesc, direction: i32, count: u64, execute_once: bool) {
    // Log the entry into the custom planner
    info!("Entering columnar_executor_run");

    // Imitate ExplainNode for recursive plan scanning behavior
    let ps = (*queryDesc).plannedstmt;
    let plan = (*ps).planTree;
    let node = plan as *mut Node;
    let node_tag = unsafe { (*node).type_};

    match node_tag {
        NodeTag::T_SeqScan => {
            let scan = (SeqScan*) plan;
            let rte = list_nth((*ps).rtable, (*scan).scan.scanrelid - 1);

            // match (*rte).rtekind {
            //     RTEKIND_RTE_RELATION => {
            //         let relation = RelationIdGetRelation(rte->relid);
            //     }
            // }
            let relation = RelationIdGetRelation((*rte).relid);

            let table = substrait::NamedTable {
                names: vec![(*(*relation).rd_rel).relname],
                advanced_extension: None
            };

            let type_info = substrait::Struct {
                types: vec![],
                type_variation_reference: 0,
                nullability: substrait::proto::type::Nullability::Required,
            };
            let base_schema = substrait::NamedStruct {
                names: vec![],
                struct: None
            };

            let list = (*plan).targetlist;
            if ((*plan).targetlist != NULL) {
                for (let i = 0; i < list.length; i++) {
                    let list_cell = list.elements.offset(i);
                    let list_cell_node = (*list_cell).ptr_value as mut* Node;
                    let list_cell_node_tag = unsafe { (*list_cell_node).type_ };
                    match (list_cell_node_tag) {
                        NodeTag::T_Var => {
                            let var = list_cell_node_tag as *mut Var;
                            let list_cell_rte = list_nth((*ps).rtable, (*var).varno - 1);
                            base_schema.names.push(get_attname((*list_cell_rte).relid, (*var).varattno, false));
                        }
                    }
                }
            }

            let sget = substrait::ReadRel {
                common: None,
                base_schema: base_schema,
                filter: None,
                best_effort_filter: None,
                projection: None,
                advanced_extension: None,
                read_type: None
            };
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


#[pg_extern]
fn hello_pg_columnar() -> &'static str {
    "Hello, pg_columnar"
}


#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_hello_pg_planner() {
        assert_eq!("Hello, pg_columnar", crate::hello_pg_columnar());
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

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

// use crate::to_substrait::transform_seqscan_to_substrait;
// use crate::to_substrait::transform_modify_to_substrait;

// mod to_substrait;
mod to_datafusion;
use crate::to_datafusion::transform_seqscan_to_datafusion;
use crate::to_datafusion::transform_valuesscan_to_datafusion;
use crate::to_datafusion::transform_modify_to_datafusion;
use datafusion::common::cast::as_primitive_array;
use datafusion_substrait::substrait::proto::RelRoot;
use datafusion_substrait::substrait::proto::Rel;
use datafusion_substrait::substrait::proto::rel::RelType::Read;
use datafusion_substrait::substrait::proto::plan_rel::RelType::Root;
use datafusion_substrait::substrait::proto::PlanRel;
use datafusion_substrait::logical_plan::consumer::from_substrait_plan;
use datafusion::common::arrow::array::types::{Int8Type, Int16Type, Int32Type, Int64Type, UInt32Type, Float32Type, GenericStringType, Time32SecondType, TimestampSecondType, Date32Type};
use datafusion::arrow::datatypes::{DataType, TimeUnit};
use datafusion::arrow::array::AsArray;
use datafusion::logical_expr::LogicalPlanBuilder;

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
use pgrx::PgRelation;
use pgrx::PgTupleDesc;
use pgrx::pg_sys::CmdType_CMD_SELECT;
use pgrx::pg_sys::MakeTupleTableSlot;
use pgrx::pg_sys::TTSOpsVirtual;
use pgrx::pg_sys::ModifyTable;
use pgrx::pg_sys::rt_fetch;
use pgrx::pg_sys::RelationIdGetRelation;
use datafusion::logical_expr::LogicalPlan;

use futures::executor;

mod col_datafusion;
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

unsafe fn describe_nodes(tree: *mut pg_sys::Plan, ps: *mut pg_sys::PlannedStmt) {
    info!("Describing plan");
    // Imitate ExplainNode for recursive plan scanning behavior
    let node_tag = (*tree).type_;
    info!("Node tag {:?}", node_tag);
    // let targetlist = (*tree).targetlist;
    // info!("here 1");
    // if !targetlist.is_null() {
    //     info!("targetlist {:p}", targetlist);
    //     for i in 0..(*targetlist).length {
    //         info!("targetlist {}", i);
    //         let tle =
    //             (*(*targetlist).elements.offset(i as isize)).ptr_value as *mut pg_sys::TargetEntry;
    //     }
    // }

    // let initplan = (*tree).initPlan;
    // info!("here 2");
    // if !initplan.is_null() {
    //     for i in 0..(*initplan).length {
    //         info!("initplan {}", i);
    //         let sp =
    //             (*(*initplan).elements.offset(i as isize)).ptr_value as *mut pg_sys::SubPlan;

    //         let plan = (*(*(*ps).subplans).elements.offset((*sp).plan_id as isize - 1)).ptr_value as *mut pg_sys::Plan;
    //         info!("plan {:p}", plan);
    //         describe_nodes(plan, ps);
    //     }
    // }

    if !(*tree).lefttree.is_null() {
        info!("Left tree");
        describe_nodes((*tree).lefttree, ps);
    }
    if !(*tree).righttree.is_null() {
        info!("Right tree");
        describe_nodes((*tree).righttree, ps);
    }
}

async unsafe extern "C" fn columnar_executor_run_internal(
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
    describe_nodes(plan, ps);

    let node = plan as *mut Node;
    let node_tag = (*node).type_;
    let rtable = (*ps).rtable;

    let planstate = (*query_desc).planstate;

    info!("{:?}", node_tag);

    match node_tag {
        NodeTag::T_SeqScan => {
            info!("match T_SeqScan");
            // Check if the table is using our table AM before running our custom logic
            let rte = unsafe { rt_fetch((*(plan as *mut SeqScan)).scan.scanrelid, rtable) };
            let relation = unsafe { RelationIdGetRelation((*rte).relid) };
            let am_handler = (*relation).rd_amhandler;

            let handlername_cstr = CString::new("mem").unwrap();
            let handlername_ptr = handlername_cstr.as_ptr() as *mut i8;
            let memam_oid = get_am_oid(handlername_ptr, true);
            let amTup = SearchSysCache1(SysCacheIdentifier_AMOID.try_into().unwrap(), Datum::from(memam_oid));
            let amForm = heap_tuple_get_struct::<FormData_pg_am>(amTup);
            let memhandler_oid = (*amForm).amhandler;
            ReleaseSysCache(amTup);
            if am_handler != memhandler_oid {
                standard_ExecutorRun(query_desc, direction, count, execute_once);
                info!("Standard ExecutorRun called");
                return;
            }

            if let Ok(df_plan) = transform_seqscan_to_datafusion(ps).await {
                if let Ok(dataframe) = col_datafusion::CONTEXT.execute_logical_plan(df_plan).await {
                    let recordbatchvec_result = dataframe.collect().await;
                    match recordbatchvec_result {
                        Ok(recordbatchvec) => {
                            let sendTuples = ((*query_desc).operation == CmdType_CMD_SELECT ||
                                  (*(*query_desc).plannedstmt).hasReturning);

                            if (sendTuples) {
                                let dest = (*query_desc).dest;
                                let rStartup = (*dest).rStartup;
                                match rStartup {
                                    Some(f) => f(dest, (*query_desc).operation.try_into().unwrap(), (*query_desc).tupDesc),
                                    None => panic!("no rstartup"),
                                };
                                let tuple_desc = PgTupleDesc::from_pg((*query_desc).tupDesc);

                                let receiveSlot = (*dest).receiveSlot;
                                match receiveSlot {
                                    Some(f) => for recordbatch in recordbatchvec.iter() {
                                        for row_index in 0..recordbatch.num_rows() {
                                            let tuple_table_slot = MakeTupleTableSlot((*query_desc).tupDesc, &TTSOpsVirtual);
                                            let mut col_index = 0;
                                            for attr in tuple_desc.iter() {
                                                let column = recordbatch.column(col_index);
                                                let dt = column.data_type();
                                                let tts_value = (*tuple_table_slot).tts_values.offset(col_index.try_into().unwrap());
                                                match dt {
                                                    DataType::Boolean => *tts_value = column.as_primitive::<Int8Type>().value(row_index).into_datum().unwrap(),
                                                    DataType::Int16 => *tts_value = column.as_primitive::<Int16Type>().value(row_index).into_datum().unwrap(),
                                                    DataType::Int32 => *tts_value = column.as_primitive::<Int32Type>().value(row_index).into_datum().unwrap(),
                                                    DataType::Int64 => *tts_value = column.as_primitive::<Int64Type>().value(row_index).into_datum().unwrap(),
                                                    DataType::UInt32 => *tts_value = column.as_primitive::<UInt32Type>().value(row_index).into_datum().unwrap(),
                                                    DataType::Float32 => *tts_value = column.as_primitive::<Float32Type>().value(row_index).into_datum().unwrap(),
                                                    // DataType::Utf8 => *tts_value = column.as_primitive::<GenericStringType>().value(row_index).into_datum().unwrap(),
                                                    DataType::Time32(TimeUnit::Second) => *tts_value = column.as_primitive::<Time32SecondType>().value(row_index).into_datum().unwrap(),
                                                    DataType::Timestamp(TimeUnit::Second, None) => *tts_value = column.as_primitive::<TimestampSecondType>().value(row_index).into_datum().unwrap(),
                                                    DataType::Date32 => *tts_value = column.as_primitive::<Date32Type>().value(row_index).into_datum().unwrap(),
                                                    _ => panic!("Unsupported PostgreSQL type: {:?}", dt),
                                                };
                                                col_index += 1;
                                            }
                                            f(tuple_table_slot, dest);
                                        }
                                    },
                                    None => panic!("no receiveslot"),
                                }

                                let rShutdown = (*dest).rShutdown;
                                match rShutdown {
                                    Some(f) => f(dest),
                                    None => panic!("no rshutdown"),
                                }
                            } else {
                                info!("no sendTuples");
                            }
                        },
                        Err(e) => info!("dataframe collect failed: {}", e),
                    }
                } else {
                    info!("failed executing logical plan");
                }
            }  else {
                info!("failed to transform seqscan to datafusion await");
            }

            return;
        }

        NodeTag::T_ModifyTable => {
            info!("match T_ModifyTable");
            // Check if the table is using our table AM before running our custom logic
            let rte = unsafe { rt_fetch((*((*ps).planTree as *mut ModifyTable)).nominalRelation, rtable) };
            let relation = unsafe { RelationIdGetRelation((*rte).relid) };
            let am_handler = (*relation).rd_amhandler;

            let handlername_cstr = CString::new("mem").unwrap();
            let handlername_ptr = handlername_cstr.as_ptr() as *mut i8;
            let memam_oid = get_am_oid(handlername_ptr, true);
            let amTup = SearchSysCache1(SysCacheIdentifier_AMOID.try_into().unwrap(), Datum::from(memam_oid));
            let amForm = heap_tuple_get_struct::<FormData_pg_am>(amTup);
            let memhandler_oid = (*amForm).amhandler;
            ReleaseSysCache(amTup);
            info!("{:?} ? {:?}", am_handler, memhandler_oid);
            if am_handler != memhandler_oid {
                standard_ExecutorRun(query_desc, direction, count, execute_once);
                info!("Standard ExecutorRun called");
                return;
            }

            // TODO: this is super sus but we need to do something like this to get through the tree 
            //       and pass the appropriate logical plans back up to their parent nodes
            // lefttree is the outer plan
            // let input: LogicalPlan = if !(*plan).lefttree.is_null() && (*((*plan).lefttree as *mut Node)).type_ == NodeTag::T_ValuesScan {
            //     transform_valuesscan_to_datafusion((*plan).lefttree)
            // // } else if !(*plan).righttree.is_null() && (*((*plan).righttree as *mut Node)).type_ == NodeTag::T_ValuesScan {
            // } else {
            //     // TODO: the above condition should be here, but just trying to get through a simple query for now
            //     transform_valuesscan_to_datafusion((*plan).righttree)
            // };
            // let input: LogicalPlan = transform_valuesscan_to_datafusion((*plan).lefttree);

            // else {
            //     LogicalPlanBuilder::scan(
            //         object_name_to_string(&table_name),
            //         table_source,
            //         None,
            //     )?
            //     .build()?
            // };

            info!("start if nest");
            if let Ok(logical_plan) = transform_modify_to_datafusion(ps) {
                info!("if 2");
                if let Ok(dataframe) = col_datafusion::CONTEXT.execute_logical_plan(logical_plan).await {
                    info!("if 3");
                    if let Ok(recordbatchvec) = dataframe.collect().await {
                        // TODO: implement sendTuples to support returning tuples
                    } else {
                        info!("dataframe collect failed");
                    }
                } else {
                    info!("failed to create dataframe");
                }
            } else {
                info!("failed executing logical plan");
            }

            return;
        }
        _ => {
            // TODO: Add missing types
        }
    }

    unsafe {
        info!("calling standard_ExecutorRun");
        standard_ExecutorRun(query_desc, direction, count, execute_once);

        // Log the fact that standard planner was called
        info!("Standard ExecutorRun called");
    }
}

unsafe extern "C" fn columnar_executor_run(
    query_desc: *mut QueryDesc,
    direction: i32,
    count: u64,
    execute_once: bool,
) {
    executor::block_on(columnar_executor_run_internal(query_desc, direction, count, execute_once));
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

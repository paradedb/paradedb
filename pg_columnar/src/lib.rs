#![allow(unused)]
#![allow(non_snake_case)]

use lazy_static::lazy_static;
use pg_sys::{
    self, get_am_oid, heap_tuple_get_struct, planner_hook, standard_ExecutorRun, standard_planner,
    table_slot_create, CmdType_CMD_SELECT, Datum, ExecDropSingleTupleTableSlot,
    ExecStoreVirtualTuple, ExecutorRun_hook, FormData_pg_am, Node, NodeTag, ParamListInfoData,
    PlannedStmt, Query, QueryDesc, RelationIdGetRelation, ReleaseSysCache, SearchSysCache1,
    SeqScan, SysCacheIdentifier_AMOID,
};
use pgrx::once_cell::sync::Lazy;
use pgrx::pg_sys::rt_fetch;
use pgrx::prelude::*;
use pgrx::{PgRelation, PgTupleDesc};
use shared::logs::ParadeLogsGlobal;
use shared::telemetry;

mod to_datafusion;
use crate::to_datafusion::transform_pg_plan_to_df_plan;

use datafusion::arrow::array::AsArray;
use datafusion::arrow::datatypes::{DataType, TimeUnit};
use datafusion::common::arrow::array::types::{
    Date32Type, Float32Type, GenericStringType, Int16Type, Int32Type, Int64Type, Int8Type,
    Time32SecondType, TimestampSecondType, UInt32Type, UInt64Type,
};
use datafusion::common::arrow::array::RecordBatch;
use datafusion_substrait::substrait::proto::{PlanRel, Rel, RelRoot};

use std::ffi::CString;
use std::ptr;

use async_std::task;

mod col_datafusion;
mod table_access;

pgrx::pg_module_magic!();

extension_sql_file!("../sql/_bootstrap.sql");

// This is a flag that can be set by the user in a session to enable logs.
// You need to initialize this in every extension that uses `plog!`.
static PARADE_LOGS_GLOBAL: ParadeLogsGlobal = ParadeLogsGlobal::new("pg_columnar");

unsafe fn describe_nodes(tree: *mut pg_sys::Plan, ps: *mut pg_sys::PlannedStmt) {
    info!("Describing plan");
    // Imitate ExplainNode for recursive plan scanning behavior
    let node_tag = (*tree).type_;
    info!("Node tag {:?}", node_tag);

    if !(*tree).lefttree.is_null() {
        info!("Left tree");
        describe_nodes((*tree).lefttree, ps);
    }
    if !(*tree).righttree.is_null() {
        info!("Right tree");
        describe_nodes((*tree).righttree, ps);
    }
}

#[pg_guard]
unsafe fn plannedstmt_using_columnar(ps: *mut PlannedStmt) -> bool {
    let rtable = (*ps).rtable;
    if rtable.is_null() {
        return false;
    }

    // Get mem table AM handler OID
    let handlername_cstr = CString::new("mem").unwrap();
    let handlername_ptr = handlername_cstr.as_ptr() as *mut i8;
    let memam_oid = get_am_oid(handlername_ptr, true);
    if memam_oid == pg_sys::InvalidOid {
        return false;
    }

    let amTup = SearchSysCache1(
        SysCacheIdentifier_AMOID.try_into().unwrap(),
        Datum::from(memam_oid),
    );
    let amForm = heap_tuple_get_struct::<FormData_pg_am>(amTup);
    let memhandler_oid = (*amForm).amhandler;
    ReleaseSysCache(amTup);

    let elements = (*rtable).elements;
    let mut using_noncol: bool = false;
    let mut using_col: bool = false;
    for i in 0..(*rtable).length {
        let rte = (*elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::RangeTblEntry;
        if (*rte).rtekind != pgrx::pg_sys::RTEKind_RTE_RELATION {
            continue;
        }
        let relation = RelationIdGetRelation((*rte).relid);
        let pg_relation = PgRelation::from_pg_owned(relation);
        if !pg_relation.is_table() {
            continue;
        }

        let am_handler = (*relation).rd_amhandler;

        // If any table uses the Table AM handler, then return true.
        // TODO: if we support more operations, this will be more complex.
        //       for example, if to support joins, some of the nodes will use
        //       table AM for the nodes while others won't. In this case,
        //       we'll have to process in postgres plan for part of it and
        //       datafusion for the other part. For now, we'll simply
        //       fail if we encounter an unsupported node, so this won't happen.
        if am_handler == memhandler_oid {
            using_col = true;
        } else {
            using_noncol = true;
        }
    }

    if using_col && using_noncol {
        // panic! is okay here because we are protected by pg_guard
        panic!("Mixing table types in a single query is not supported yet");
    }

    return using_col;
}

// Note: getting the relation through get_relation uses from_pg_owned, so no need to manually close later on
unsafe fn get_relation(ps: *mut PlannedStmt) -> PgRelation {
    let rtable = (*ps).rtable;
    let plan = (*ps).planTree as *mut pgrx::pg_sys::Node;
    // TODO: Replace hard-coded 1
    let rte = rt_fetch(1, rtable);
    let relation = RelationIdGetRelation((*rte).relid);
    let pg_relation = PgRelation::from_pg_owned(relation);

    return pg_relation;
}

unsafe fn send_tuples_if_necessary(
    query_desc: *mut QueryDesc,
    recordbatchvec: Vec<RecordBatch>,
) -> Result<(), String> {
    let sendTuples = ((*query_desc).operation == CmdType_CMD_SELECT
        || (*(*query_desc).plannedstmt).hasReturning);

    if (sendTuples) {
        let dest = (*query_desc).dest;
        let rStartup = (*dest).rStartup;
        match rStartup {
            Some(f) => f(
                dest,
                (*query_desc).operation.try_into().unwrap(),
                (*query_desc).tupDesc,
            ),
            None => return Err(format!("no rstartup")),
        };
        let tuple_desc = PgTupleDesc::from_pg_unchecked((*query_desc).tupDesc);

        let relation = get_relation((*query_desc).plannedstmt);

        let receiveSlot = (*dest).receiveSlot;
        match receiveSlot {
            Some(f) => {
                for recordbatch in recordbatchvec.iter() {
                    for row_index in 0..recordbatch.num_rows() {
                        let tuple_table_slot =
                            table_slot_create(relation.as_ptr(), ptr::null_mut());
                        ExecStoreVirtualTuple(tuple_table_slot);
                        let mut col_index = 0;
                        for attr in tuple_desc.iter() {
                            let column = recordbatch.column(col_index);
                            let dt = column.data_type();
                            let tts_value = (*tuple_table_slot)
                                .tts_values
                                .offset(col_index.try_into().unwrap());
                            match dt {
                                DataType::Boolean => {
                                    *tts_value = column
                                        .as_primitive::<Int8Type>()
                                        .value(row_index)
                                        .into_datum()
                                        .unwrap()
                                }
                                DataType::Int16 => {
                                    *tts_value = column
                                        .as_primitive::<Int16Type>()
                                        .value(row_index)
                                        .into_datum()
                                        .unwrap()
                                }
                                DataType::Int32 => {
                                    *tts_value = column
                                        .as_primitive::<Int32Type>()
                                        .value(row_index)
                                        .into_datum()
                                        .unwrap()
                                }
                                DataType::Int64 => {
                                    *tts_value = column
                                        .as_primitive::<Int64Type>()
                                        .value(row_index)
                                        .into_datum()
                                        .unwrap()
                                }
                                DataType::UInt32 => {
                                    *tts_value = column
                                        .as_primitive::<UInt32Type>()
                                        .value(row_index)
                                        .into_datum()
                                        .unwrap()
                                }
                                DataType::Float32 => {
                                    *tts_value = column
                                        .as_primitive::<Float32Type>()
                                        .value(row_index)
                                        .into_datum()
                                        .unwrap()
                                }
                                // DataType::Utf8 => *tts_value = column.as_primitive::<GenericStringType>().value(row_index).into_datum().unwrap(),
                                DataType::Time32(TimeUnit::Second) => {
                                    *tts_value = column
                                        .as_primitive::<Time32SecondType>()
                                        .value(row_index)
                                        .into_datum()
                                        .unwrap()
                                }
                                DataType::Timestamp(TimeUnit::Second, None) => {
                                    *tts_value = column
                                        .as_primitive::<TimestampSecondType>()
                                        .value(row_index)
                                        .into_datum()
                                        .unwrap()
                                }
                                DataType::Date32 => {
                                    *tts_value = column
                                        .as_primitive::<Date32Type>()
                                        .value(row_index)
                                        .into_datum()
                                        .unwrap()
                                }
                                _ => {
                                    return Err(format!(
                                    "send_tuples_if_necessary: Unsupported PostgreSQL type: {:?}",
                                    dt
                                ))
                                }
                            };
                            col_index += 1;
                        }
                        f(tuple_table_slot, dest);
                        ExecDropSingleTupleTableSlot(tuple_table_slot);
                    }
                }
            }
            None => return Err(format!("no receiveslot")),
        }

        let rShutdown = (*dest).rShutdown;
        match rShutdown {
            Some(f) => f(dest),
            None => return Err(format!("no rshutdown")),
        }
    }
    Ok(())
}

#[pg_guard]
// Note: only use unwrap or panic! in THIS function. This is the only function protected with
//       pg_guard and can therefore translate Rust panics into Postgres panics. All other
//       functions should bubble up errors as String.
unsafe extern "C" fn columnar_executor_run(
    query_desc: *mut QueryDesc,
    direction: i32,
    count: u64,
    execute_once: bool,
) {
    // Imitate ExplainNode for recursive plan scanning behavior
    let ps = (*query_desc).plannedstmt;
    if !plannedstmt_using_columnar(ps) {
        info!("standard_ExecutorRun");
        standard_ExecutorRun(query_desc, direction, count, execute_once);
        return;
    }

    let plan: *mut pg_sys::Plan = (*ps).planTree;

    let node = plan as *mut Node;
    let node_tag = (*node).type_;
    let rtable = (*ps).rtable;

    let logical_plan = transform_pg_plan_to_df_plan(plan.into(), rtable.into()).unwrap();
    let dataframe =
        task::block_on(col_datafusion::CONTEXT.execute_logical_plan(logical_plan)).unwrap();
    let recordbatchvec: Vec<RecordBatch> = task::block_on(dataframe.collect()).unwrap();

    // This is for any node types that need to do additional processing on estate
    match node_tag {
        NodeTag::T_ModifyTable => {
            let num_updated = recordbatchvec[0]
                .column(0)
                .as_primitive::<UInt64Type>()
                .value(0);
            (*(*query_desc).estate).es_processed = num_updated;
        }
        _ => (),
    }

    send_tuples_if_necessary(query_desc, recordbatchvec);
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

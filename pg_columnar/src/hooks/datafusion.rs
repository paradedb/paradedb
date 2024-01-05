use async_std::task;
use datafusion::arrow::array::AsArray;
use datafusion::common::arrow::array::types::UInt64Type;
use datafusion::common::arrow::array::RecordBatch;
use pgrx::hooks::PgHooks;
use pgrx::*;
use std::ffi::CStr;

use crate::datafusion::registry::CONTEXT;
use crate::hooks::utils::{
    copy_stmt_is_columnar, planned_stmt_is_columnar, send_tuples_if_necessary,
};
use crate::nodes::root::RootPlanNode;
use crate::nodes::utils::DatafusionPlanProducer;
use crate::tableam::utils::{BulkInsertState, BULK_INSERT_STATE};

pub struct DatafusionHook;

impl PgHooks for DatafusionHook {
    fn executor_run(
        &mut self,
        query_desc: PgBox<pg_sys::QueryDesc>,
        direction: pg_sys::ScanDirection,
        count: u64,
        execute_once: bool,
        prev_hook: fn(
            query_desc: PgBox<pg_sys::QueryDesc>,
            direction: pg_sys::ScanDirection,
            count: u64,
            execute_once: bool,
        ) -> HookResult<()>,
    ) -> HookResult<()> {
        unsafe {
            let ps = query_desc.plannedstmt;

            if !planned_stmt_is_columnar(ps) {
                prev_hook(query_desc, direction, count, execute_once);
                return HookResult::new(());
            }

            let plan: *mut pg_sys::Plan = (*ps).planTree;
            let node = plan as *mut pg_sys::Node;
            let node_tag = (*node).type_;
            let rtable = (*ps).rtable;

            let logical_plan = RootPlanNode::datafusion_plan(plan, rtable, None)
                .expect("Could not get logical plan");

            let context_lock = CONTEXT.read();
            let context = (*context_lock)
                .as_ref()
                .ok_or("No columnar context found. Run SELECT paradedb.init(); first.")
                .unwrap();

            let dataframe = task::block_on(context.execute_logical_plan(logical_plan))
                .expect("Could not execute logical plan");

            let recordbatchvec: Vec<RecordBatch> =
                task::block_on(dataframe.collect()).expect("Could not collect dataframe");

            // This is for any node types that need to do additional processing on estate
            if node_tag == pg_sys::NodeTag::T_ModifyTable {
                let num_updated = recordbatchvec[0]
                    .column(0)
                    .as_primitive::<UInt64Type>()
                    .value(0);
                (*(*query_desc.clone().into_pg()).estate).es_processed = num_updated;
            }

            let _ = send_tuples_if_necessary(query_desc.into_pg(), recordbatchvec);

            HookResult::new(())
        }
    }

    fn process_utility_hook(
        &mut self,
        pstmt: PgBox<pg_sys::PlannedStmt>,
        query_string: &CStr,
        read_only_tree: Option<bool>,
        context: pg_sys::ProcessUtilityContext,
        params: PgBox<pg_sys::ParamListInfoData>,
        query_env: PgBox<pg_sys::QueryEnvironment>,
        dest: PgBox<pg_sys::DestReceiver>,
        completion_tag: *mut pg_sys::QueryCompletion,
        prev_hook: fn(
            pstmt: PgBox<pg_sys::PlannedStmt>,
            query_string: &CStr,
            read_only_tree: Option<bool>,
            context: pg_sys::ProcessUtilityContext,
            params: PgBox<pg_sys::ParamListInfoData>,
            query_env: PgBox<pg_sys::QueryEnvironment>,
            dest: PgBox<pg_sys::DestReceiver>,
            completion_tag: *mut pg_sys::QueryCompletion,
        ) -> HookResult<()>,
    ) -> HookResult<()> {
        let plan = pstmt.utilityStmt;

        if unsafe { (*plan).type_ } == pg_sys::NodeTag::T_CopyStmt {
            let copy_stmt = plan as *mut pg_sys::CopyStmt;

            if unsafe { copy_stmt_is_columnar(copy_stmt) } {
                let mut bulk_insert_state = BULK_INSERT_STATE.write();
                *bulk_insert_state = BulkInsertState::new();
            }
        }

        prev_hook(
            pstmt,
            query_string,
            read_only_tree,
            context,
            params,
            query_env,
            dest,
            completion_tag,
        );

        HookResult::new(())
    }
}

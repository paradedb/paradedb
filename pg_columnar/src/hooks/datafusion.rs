use async_std::task;
use datafusion::arrow::array::AsArray;
use datafusion::common::arrow::array::types::UInt64Type;
use datafusion::common::arrow::array::RecordBatch;
use pgrx::hooks::PgHooks;
use pgrx::*;

use crate::hooks::utils::{planned_stmt_is_columnar, send_tuples_if_necessary};
use crate::nodes::root::RootPlanNode;
use crate::nodes::utils::DatafusionPlanTranslator;
use crate::tableam::utils::CONTEXT;

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

            let dataframe = task::block_on(CONTEXT.execute_logical_plan(logical_plan))
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
}

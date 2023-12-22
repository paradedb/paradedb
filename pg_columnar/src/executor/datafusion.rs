use async_std::task;
use datafusion::arrow::array::AsArray;
use datafusion::common::arrow::array::types::UInt64Type;
use datafusion::common::arrow::array::RecordBatch;
use pgrx::*;

use crate::executor::utils::send_tuples_if_necessary;
use crate::nodes::root::RootPlanNode;
use crate::nodes::utils::{using_columnar, DatafusionPlanTranslator};
use crate::tableam::utils::CONTEXT;

#[pg_guard]
pub unsafe extern "C" fn executor_hook(
    query_desc: *mut pg_sys::QueryDesc,
    direction: i32,
    count: u64,
    execute_once: bool,
) {
    let ps = (*query_desc).plannedstmt;
    if !using_columnar(ps) {
        pg_sys::standard_ExecutorRun(query_desc, direction, count, execute_once);
        return;
    }

    let plan: *mut pg_sys::Plan = (*ps).planTree;

    let node = plan as *mut pg_sys::Node;
    let node_tag = (*node).type_;
    let rtable = (*ps).rtable;

    let logical_plan = RootPlanNode::datafusion_plan(plan, rtable, None).unwrap();
    let dataframe = task::block_on(CONTEXT.execute_logical_plan(logical_plan)).unwrap();
    let recordbatchvec: Vec<RecordBatch> = task::block_on(dataframe.collect()).unwrap();

    // This is for any node types that need to do additional processing on estate
    if node_tag == pg_sys::NodeTag::T_ModifyTable {
        let num_updated = recordbatchvec[0]
            .column(0)
            .as_primitive::<UInt64Type>()
            .value(0);
        (*(*query_desc).estate).es_processed = num_updated;
    }

    let _ = send_tuples_if_necessary(query_desc, recordbatchvec);
}

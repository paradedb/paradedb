use datafusion::logical_expr::LogicalPlanBuilder;
use datafusion::logical_expr::{Expr, LogicalPlan};
use pgrx::*;

use crate::datafusion::error::datafusion_err_to_string;
use crate::datafusion::table::DatafusionTable;
use crate::nodes::producer::{DatafusionExprProducer, DatafusionPlanProducer};
use crate::nodes::t_var::VarNode;
use crate::tableam::utils::get_pg_relation;

pub struct GroupNode;
impl DatafusionPlanProducer for GroupNode {
    unsafe fn datafusion_plan(
        plan: *mut pg_sys::Plan,
        rtable: *mut pg_sys::List,
        _outer_plan: Option<LogicalPlan>,
    ) -> Result<LogicalPlan, String> {
        let group_node = plan as *mut pg_sys::Group;

        let aggr_expr: Vec<Expr> = vec![];
        let mut group_expr: Vec<Expr> = vec![];

        let list = (*plan).targetlist;
        if list.is_null() {
            return Err("Sort targetlist is null".to_string());
        }
        let elements = (*list).elements;

        // Get index of the column to group
        let col_idx_ptr = (*group_node).grpColIdx;
        if col_idx_ptr.is_null() {
            return Err("group column index is null".to_string());
        }

        // Convert the index raw pointer to a slice
        let col_idx_slice: &[i16] =
            std::slice::from_raw_parts(col_idx_ptr, (*group_node).numCols as usize);

        for &idx in col_idx_slice {
            let col_idx = idx - 1;
            let list_cell_node =
                (*elements.offset(col_idx as isize)).ptr_value as *mut pgrx::pg_sys::Node;
            assert!(is_a(list_cell_node, pg_sys::NodeTag::T_TargetEntry));
            let target_entry = list_cell_node as *mut pgrx::pg_sys::TargetEntry;
            let te_expr_node = (*target_entry).expr as *mut pgrx::pg_sys::Node;
            let expr = VarNode::datafusion_expr(te_expr_node, Some(rtable))?;
            group_expr.push(expr);
        }

        // We use a LogicalPlanBuilder to pass in group expressions
        // LogicalPlan::TableScan takes in expressions but they are pushdowns,
        // which are not supported by our existing TableProvider
        // Find the table we're supposed to be scanning by querying the range table
        let scan = plan as *mut pg_sys::SeqScan;
        let rte = pg_sys::rt_fetch((*scan).scan.scanrelid, rtable);
        let pg_relation = get_pg_relation(rte)?;
        let table = DatafusionTable::new(&pg_relation)?;

        let mut builder = LogicalPlanBuilder::scan(table.name()?, table.source()?, None)
            .map_err(datafusion_err_to_string())?;

        builder = builder
            .aggregate(group_expr.clone(), aggr_expr)
            .map_err(datafusion_err_to_string())?;

        builder.build().map_err(datafusion_err_to_string())
    }
}

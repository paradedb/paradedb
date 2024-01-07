use datafusion::logical_expr::{Expr, LogicalPlan, LogicalPlanBuilder};

use pgrx::*;

use crate::datafusion::error::datafusion_err_to_string;
use crate::datafusion::table::DatafusionTable;
use crate::nodes::producer::DatafusionExprProducer;
use crate::nodes::producer::DatafusionPlanProducer;
use crate::nodes::t_opexpr::OpExprNode;
use crate::nodes::t_var::VarNode;
use crate::tableam::utils::get_pg_relation;

pub struct SeqScanNode;
impl DatafusionPlanProducer for SeqScanNode {
    unsafe fn datafusion_plan(
        plan: *mut pg_sys::Plan,
        rtable: *mut pg_sys::List,
        _outer_plan: Option<LogicalPlan>,
    ) -> Result<LogicalPlan, String> {
        let scan = plan as *mut pg_sys::SeqScan;

        // Read projections (i.e. which columns to read)
        let mut projections: Vec<Expr> = vec![];
        let targets = (*plan).targetlist;

        if !targets.is_null() {
            let elements = (*targets).elements;
            for i in 0..(*targets).length {
                let list_cell_node = (*elements.offset(i as isize)).ptr_value as *mut pg_sys::Node;
                let target_entry = list_cell_node as *mut pg_sys::TargetEntry;
                let node = (*target_entry).expr as *mut pg_sys::Node;

                match (*node).type_ {
                    pg_sys::NodeTag::T_Var => {
                        projections.push(VarNode::datafusion_expr(node, Some(rtable))?);
                    }
                    pg_sys::NodeTag::T_OpExpr => {
                        projections.push(OpExprNode::datafusion_expr(node, Some(rtable))?);
                    }
                    _ => {
                        return Err(format!(
                            "Node {:?} not supported in SeqScanNode",
                            (*node).type_
                        ))
                    }
                }
            }
        }

        // Read filters (i.e. WHERE clause)
        let mut filters: Vec<Expr> = vec![];
        let quals = (*plan).qual;

        if !quals.is_null() {
            let elements = (*quals).elements;
            for i in 0..(*quals).length {
                let list_cell_node = (*elements.offset(i as isize)).ptr_value as *mut pg_sys::Node;
                let expr = OpExprNode::datafusion_expr(list_cell_node, Some(rtable))?;
                filters.push(expr);
            }
        }

        // We use a LogicalPlanBuilder to pass in filters and projections
        // LogicalPlan::TableScan takes in filters but they are filter pushdowns,
        // which are not supported by our existing TableProvider
        // Find the table we're supposed to be scanning by querying the range table
        let rte = pg_sys::rt_fetch((*scan).scan.scanrelid, rtable);
        let pg_relation = get_pg_relation(rte)?;
        let table = DatafusionTable::new(&pg_relation)?;

        let mut builder = LogicalPlanBuilder::scan(table.name()?, table.source()?, None)
            .map_err(datafusion_err_to_string())?;

        for filter in filters {
            builder = builder.filter(filter).map_err(datafusion_err_to_string())?;
        }

        builder = builder
            .project(projections)
            .map_err(datafusion_err_to_string())?;

        builder.build().map_err(datafusion_err_to_string())
    }
}

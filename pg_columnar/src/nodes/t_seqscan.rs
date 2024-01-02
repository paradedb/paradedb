use datafusion::logical_expr::{Expr, LogicalPlan, LogicalPlanBuilder};

use pgrx::*;

use crate::nodes::t_opexpr::OpExpr;
use crate::nodes::t_var::VarNode;
use crate::nodes::utils::DatafusionExprTranslator;
use crate::nodes::utils::DatafusionPlanTranslator;
use crate::nodes::utils::{
    datafusion_err_to_string, get_datafusion_table, get_datafusion_table_name,
};
use crate::tableam::utils::get_pg_relation;

pub struct SeqScanNode;
impl DatafusionPlanTranslator for SeqScanNode {
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
                let var = (*target_entry).expr as *mut pg_sys::Node;

                projections.push(VarNode::datafusion_expr(var, Some(rtable))?);
            }
        }

        // Read filters (i.e. WHERE clause)
        let mut filters: Vec<Expr> = vec![];
        let quals = (*plan).qual;

        if !quals.is_null() {
            let elements = (*quals).elements;
            for i in 0..(*quals).length {
                let list_cell_node = (*elements.offset(i as isize)).ptr_value as *mut pg_sys::Node;
                let expr = OpExpr::datafusion_expr(list_cell_node, Some(rtable))?;
                filters.push(expr);
            }
        }

        // We use a LogicalPlanBuilder to pass in filters and projections
        // LogicalPlan::TableScan takes in filters but they are filter pushdowns,
        // which are not supported by our existing TableProvider
        // Find the table we're supposed to be scanning by querying the range table
        let rte = pg_sys::rt_fetch((*scan).scan.scanrelid, rtable);
        let pg_relation = get_pg_relation(rte)?;
        let table_name = get_datafusion_table_name(&pg_relation)?;
        let table_source = get_datafusion_table(&table_name, &pg_relation)?;

        let mut builder = LogicalPlanBuilder::scan(table_name, table_source, None)
            .map_err(datafusion_err_to_string("Could not create TableScan"))?;

        for filter in filters {
            builder = builder
                .filter(filter)
                .map_err(datafusion_err_to_string("Could not apply filters"))?;
        }

        builder = builder
            .project(projections)
            .map_err(datafusion_err_to_string("Could not apply projections"))?;

        builder
            .build()
            .map_err(datafusion_err_to_string("Could not build TableScan plan"))
    }
}

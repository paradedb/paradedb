use datafusion::logical_expr::{Expr, LogicalPlan, LogicalPlanBuilder};

use pgrx::*;

use crate::nodes::t_opexpr::OpExpr;
use crate::nodes::utils::DatafusionExprTranslator;
use crate::nodes::utils::DatafusionPlanTranslator;
use crate::nodes::utils::{
    datafusion_err_to_string, datafusion_table_from_name, table_name_from_rte,
};

pub struct SeqScanNode;
impl DatafusionPlanTranslator for SeqScanNode {
    unsafe fn datafusion_plan(
        plan: *mut pg_sys::Plan,
        rtable: *mut pg_sys::List,
        _outer_plan: Option<LogicalPlan>,
    ) -> Result<LogicalPlan, String> {
        let scan = plan as *mut pg_sys::SeqScan;

        // Read projections (i.e. which columns to read)
        let mut projections: Vec<usize> = vec![];
        let targets = (*plan).targetlist;

        if !targets.is_null() {
            let elements = (*targets).elements;
            for i in 0..(*targets).length {
                let list_cell_node = (*elements.offset(i as isize)).ptr_value as *mut pg_sys::Node;
                let target_entry = list_cell_node as *mut pgrx::pg_sys::TargetEntry;
                let var = (*target_entry).expr as *mut pgrx::pg_sys::Var;

                let col_idx = (*var).varattno as usize;
                projections.push(col_idx - 1);
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

        // We use a LogicalPlanBuilder to pass in filters
        // LogicalPlan::TableScan takes in filters but they are filter pushdowns,
        // which are not supported by our existing TableProvider
        // Find the table we're supposed to be scanning by querying the range table
        let rte = pg_sys::rt_fetch((*scan).scan.scanrelid, rtable);
        let table_name = table_name_from_rte(rte)?;
        let table_source = datafusion_table_from_name(&table_name)?;

        let mut builder = LogicalPlanBuilder::scan(table_name, table_source, None)
            .map_err(datafusion_err_to_string("Could not create TableScan"))?;

        for filter in filters {
            builder = builder
                .filter(filter)
                .map_err(datafusion_err_to_string("Could not create TableScan"))?;
        }

        builder
            .build()
            .map_err(datafusion_err_to_string("Could not build TableScan plan"))
    }
}

use datafusion::logical_expr::{Expr, LogicalPlan, Values};
use pgrx::nodes::is_a;
use pgrx::*;

use crate::datafusion::table::DatafusionTable;
use crate::nodes::t_const::ConstNode;
use crate::nodes::utils::DatafusionExprTranslator;
use crate::nodes::utils::DatafusionPlanTranslator;
use crate::tableam::utils::get_pg_relation;

pub struct ValuesScanNode;
impl DatafusionPlanTranslator for ValuesScanNode {
    unsafe fn datafusion_plan(
        plan: *mut pg_sys::Plan,
        rtable: *mut pg_sys::List,
        _outer_plan: Option<LogicalPlan>,
    ) -> Result<LogicalPlan, String> {
        let values_scan_node = plan as *mut pg_sys::ValuesScan;
        let rte = pg_sys::rt_fetch(1, rtable);
        let pg_relation = get_pg_relation(rte)?;
        let table = DatafusionTable::new(&pg_relation)?;

        let mut values: Vec<Vec<Expr>> = vec![vec![]];
        let number_of_rows = (*(*values_scan_node).values_lists).length;

        for i in 0..number_of_rows {
            let values_lists_elements = (*(*values_scan_node).values_lists).elements;
            let row = (*values_lists_elements.offset(i as isize)).ptr_value as *mut pg_sys::List;
            let mut row_values: Vec<Expr> = vec![];

            for j in 0..(*row).length {
                let list_cell_node =
                    (*(*row).elements.offset(j as isize)).ptr_value as *mut pg_sys::Node;

                assert!(is_a(list_cell_node, pg_sys::NodeTag::T_Const));

                let value = ConstNode::datafusion_expr(list_cell_node, None)?;
                row_values.push(value);
            }

            values.push(row_values);
        }

        Ok(LogicalPlan::Values(Values {
            schema: table.schema()?.into(),
            values,
        }))
    }
}

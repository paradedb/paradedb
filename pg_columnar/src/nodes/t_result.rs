use datafusion::logical_expr::{Expr, LogicalPlan, Values};
use pgrx::nodes::is_a;
use pgrx::*;

use crate::nodes::t_const::ConstNode;
use crate::nodes::utils::DatafusionExprTranslator;
use crate::nodes::utils::DatafusionPlanTranslator;
use crate::nodes::utils::{
    datafusion_schema_from_table, datafusion_table_from_name, table_name_from_rte,
};

pub struct ResultNode;
impl DatafusionPlanTranslator for ResultNode {
    unsafe fn datafusion_plan(
        plan: *mut pg_sys::Plan,
        rtable: *mut pg_sys::List,
        _outer_plan: Option<LogicalPlan>,
    ) -> Result<LogicalPlan, String> {
        let rte = pg_sys::rt_fetch(1, rtable);
        let table_name = table_name_from_rte(rte)?;
        let table_source = datafusion_table_from_name(&table_name)?;
        let schema = datafusion_schema_from_table(table_source)?;

        let mut values: Vec<Vec<Expr>> = vec![vec![]];
        let row = (*plan).targetlist;

        for j in 0..(*row).length {
            let list_cell_node =
                (*(*row).elements.offset(j as isize)).ptr_value as *mut pg_sys::Node;

            assert!(is_a(list_cell_node, pg_sys::NodeTag::T_TargetEntry));

            let target_node = list_cell_node as *mut pg_sys::TargetEntry;
            let const_node = (*target_node).expr as *mut pg_sys::Node;

            assert!(is_a(const_node, pg_sys::NodeTag::T_Const));

            let value = ConstNode::datafusion_expr(const_node, None)?;
            values[0].push(value);
        }

        Ok(LogicalPlan::Values(Values {
            schema: schema.into(),
            values,
        }))
    }
}

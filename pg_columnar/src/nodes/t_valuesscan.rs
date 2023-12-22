use datafusion::arrow::datatypes::Schema;
use datafusion::common::arrow::datatypes::Field;
use datafusion::common::DFSchema;
use datafusion::logical_expr::{Expr, LogicalPlan, Values};
use pgrx::nodes::is_a;
use pgrx::*;

use crate::nodes::t_const::ConstNode;
use crate::nodes::utils::DatafusionExprTranslator;
use crate::nodes::utils::DatafusionPlanTranslator;
use crate::nodes::utils::{datafusion_err_to_string, datafusion_table_from_rte};

pub struct ValuesScanNode;
impl DatafusionPlanTranslator for ValuesScanNode {
    unsafe fn datafusion_plan(
        plan: *mut pg_sys::Plan,
        rtable: *mut pg_sys::List,
        _outer_plan: Option<LogicalPlan>,
    ) -> Result<LogicalPlan, String> {
        let values_scan_node = plan as *mut pg_sys::ValuesScan;
        let rte = pg_sys::rt_fetch(1, rtable);
        let table = datafusion_table_from_rte(rte)?;
        let schema = DFSchema::try_from(Schema::new(
            table
                .schema()
                .fields()
                .iter()
                .map(|f| Field::new(f.name(), f.data_type().clone(), f.is_nullable()))
                .collect::<Vec<_>>(),
        ))
        .map_err(datafusion_err_to_string("Result DFSchema failed"))?;

        let _fields: Vec<Field> = vec![];
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
            schema: schema.into(),
            values,
        }))
    }
}

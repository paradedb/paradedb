use datafusion::logical_expr::{BinaryExpr, Expr, LogicalPlan, LogicalPlanBuilder, Operator};

use pgrx::*;
use std::ffi::CStr;

use crate::nodes::t_const::ConstNode;
use crate::nodes::t_var::VarNode;
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
                let operator_expr = list_cell_node as *mut pg_sys::OpExpr;
                let operator_tuple = pg_sys::SearchSysCache1(
                    pg_sys::SysCacheIdentifier_OPEROID as i32,
                    pg_sys::Datum::from((*operator_expr).opno),
                );
                let operator_form =
                    pg_sys::GETSTRUCT(operator_tuple) as *mut pg_sys::FormData_pg_operator;
                let operator_name = CStr::from_ptr((*operator_form).oprname.data.as_ptr())
                    .to_string_lossy()
                    .into_owned();

                pg_sys::ReleaseSysCache(operator_tuple);

                // TODO: This logic won't work for statements like
                // SELECT * FROM t WHERE (a + b) > 0;
                let lhs = pg_sys::pgrx_list_nth((*operator_expr).args, 0) as *mut pg_sys::Node;
                let rhs = pg_sys::pgrx_list_nth((*operator_expr).args, 1) as *mut pg_sys::Node;

                let lhs_is_const =
                    is_a(lhs, pg_sys::NodeTag::T_Const) && is_a(rhs, pg_sys::NodeTag::T_Var);
                let rhs_is_const =
                    is_a(rhs, pg_sys::NodeTag::T_Const) && is_a(lhs, pg_sys::NodeTag::T_Var);

                if !(lhs_is_const || rhs_is_const) {
                    return Err(format!(
                        "WHERE clause requires Var {} Const or Const {} Var",
                        operator_name, operator_name
                    ));
                }

                let (left_expr, right_expr) = match lhs_is_const {
                    true => (
                        ConstNode::datafusion_expr(lhs, None)?,
                        VarNode::datafusion_expr(rhs, Some(rtable))?,
                    ),
                    false => (
                        VarNode::datafusion_expr(lhs, Some(rtable))?,
                        ConstNode::datafusion_expr(rhs, None)?,
                    ),
                };

                filters.push(Expr::BinaryExpr(BinaryExpr {
                    left: Box::new(left_expr),
                    right: Box::new(right_expr),
                    op: match operator_name.as_str() {
                        "=" => Operator::Eq,
                        "<>" => Operator::NotEq,
                        "<" => Operator::Lt,
                        ">" => Operator::Gt,
                        "<=" => Operator::LtEq,
                        ">=" => Operator::GtEq,
                        _ => return Err(format!("operator {} not supported yet", operator_name)),
                    },
                }));
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

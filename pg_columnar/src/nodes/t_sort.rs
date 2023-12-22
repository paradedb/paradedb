use datafusion::logical_expr::{Expr, LogicalPlan, Sort};
use pgrx::*;
use std::ffi::CStr;

use crate::nodes::t_var::VarNode;
use crate::nodes::utils::DatafusionExprTranslator;
use crate::nodes::utils::DatafusionPlanTranslator;

pub struct SortNode;
impl DatafusionPlanTranslator for SortNode {
    unsafe fn datafusion_plan(
        plan: *mut pg_sys::Plan,
        rtable: *mut pg_sys::List,
        outer_plan: Option<LogicalPlan>,
    ) -> Result<LogicalPlan, String> {
        let outer_plan = outer_plan.ok_or("Sort does not have an outer plan")?;
        let sort_node = plan as *mut pg_sys::Sort;

        // Get sort by operator
        let sort_operators_ptr = (*sort_node).sortOperators;
        let sort_operators = unsafe {
            if sort_operators_ptr.is_null() {
                None
            } else {
                Some(*sort_operators_ptr)
            }
        };
        let sort_operators_oid = sort_operators.ok_or("Failed to get Sort operator oid")?;

        let operator_tuple = pg_sys::SearchSysCache1(
            pg_sys::SysCacheIdentifier_OPEROID as i32,
            pg_sys::Datum::from(sort_operators_oid),
        );
        let operator_form = pg_sys::GETSTRUCT(operator_tuple) as *mut pg_sys::FormData_pg_operator;
        let operator_name = CStr::from_ptr((*operator_form).oprname.data.as_ptr())
            .to_string_lossy()
            .into_owned();

        let asc = operator_name.as_str() == "<";

        // Release to avoid cache reference leaks
        pg_sys::ReleaseSysCache(operator_tuple);

        // Get nulls first
        let nulls_first_ptr = (*sort_node).nullsFirst;
        let nulls_first = unsafe {
            if nulls_first_ptr.is_null() {
                None
            } else {
                Some(*nulls_first_ptr)
            }
        };
        let nulls_first = nulls_first.ok_or("Sort does not have nulls first")?;

        let list = (*sort_node).plan.targetlist;
        if list.is_null() {
            return Err("Sort targetlist is null".to_string());
        }

        let elements = (*list).elements;
        let mut sort_expr_vec: Vec<Expr> = vec![];

        // Get index of the column to sort
        let col_idx_ptr = (*sort_node).sortColIdx;
        if col_idx_ptr.is_null() {
            return Err("Sort column index is null".to_string());
        }
        let col_idx = (*col_idx_ptr) - 1;
        let list_cell_node = (*elements.offset(col_idx as isize)).ptr_value as *mut pg_sys::Node;

        assert!(is_a(list_cell_node, pg_sys::NodeTag::T_TargetEntry));

        let target_entry = list_cell_node as *mut pg_sys::TargetEntry;
        let te_expr_node = (*target_entry).expr as *mut pg_sys::Node;
        let expr = VarNode::datafusion_expr(te_expr_node, Some(rtable))?;
        sort_expr_vec.push(expr.sort(asc, nulls_first));

        Ok(LogicalPlan::Sort(Sort {
            expr: sort_expr_vec,
            input: Box::new(outer_plan).into(),
            fetch: None,
        }))
    }
}

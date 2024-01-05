use datafusion::logical_expr::expr::AggregateFunction;
use datafusion::logical_expr::{Aggregate, AggregateFunction as BuiltInAgg, Expr, LogicalPlan};
use pgrx::nodes::is_a;
use pgrx::*;
use std::ffi::CStr;

use crate::datafusion::error::datafusion_err_to_string;
use crate::nodes::t_var::VarNode;
use crate::nodes::utils::DatafusionExprProducer;
use crate::nodes::utils::DatafusionPlanProducer;

pub struct AggRefNode;
impl DatafusionPlanProducer for AggRefNode {
    unsafe fn datafusion_plan(
        plan: *mut pg_sys::Plan,
        rtable: *mut pg_sys::List,
        outer_plan: Option<LogicalPlan>,
    ) -> Result<LogicalPlan, String> {
        let list = (*plan).targetlist;

        if list.is_null() {
            return Err("Agg targetlist is null".to_string());
        }

        let elements = (*list).elements;
        let mut agg_expr: Vec<Expr> = vec![];

        // Iterate through the list of aggregates
        for i in 0..(*list).length {
            let list_cell_node = (*elements.offset(i as isize)).ptr_value as *mut pg_sys::Node;

            assert!(is_a(list_cell_node, pg_sys::NodeTag::T_TargetEntry));

            let target_entry = list_cell_node as *mut pgrx::pg_sys::TargetEntry;
            let expr = (*target_entry).expr;

            assert!(is_a(expr as *mut pg_sys::Node, pg_sys::NodeTag::T_Aggref));

            // Map the Postgres aggregate function to a DataFusion aggregate function
            let agg_ref = expr as *mut pg_sys::Aggref;
            let df_agg = transform_pg_agg_to_df_agg((*agg_ref).aggfnoid);

            // Read function arguments
            let args = (*agg_ref).args;
            let mut args_expr: Vec<Expr> = vec![];

            if !args.is_null() {
                let elements = (*args).elements;
                for i in 0..(*args).length {
                    let arg_node = (*elements.offset(i as isize)).ptr_value as *mut pg_sys::Node;

                    assert!(is_a(arg_node, pg_sys::NodeTag::T_TargetEntry));

                    let target_entry = arg_node as *mut pg_sys::TargetEntry;
                    let var = (*target_entry).expr as *mut pg_sys::Node;
                    let column = VarNode::datafusion_expr(var, Some(rtable))?;
                    args_expr.push(column);
                }
            }

            // Check if the aggregate is distinct
            let distinct = !(*agg_ref).aggdistinct.is_null();

            // Check if * is used, ie COUNT(*)
            if (*agg_ref).aggstar {
                args_expr = vec![Expr::Wildcard { qualifier: None }];
            }

            // TODO: For now we're ignoring filters and order bys
            // These are only relevant for more complex aggregates which we don't support
            // Don't get this confused with the outer plan's filters and order bys
            agg_expr.push(Expr::AggregateFunction(AggregateFunction::new(
                df_agg, args_expr, distinct, None, None,
            )));
        }

        let outer_plan =
            outer_plan.ok_or_else(|| "Aggregate does not have an outer plan".to_string())?;

        Ok(LogicalPlan::Aggregate(
            Aggregate::try_new(Box::new(outer_plan).into(), vec![], agg_expr)
                .map_err(datafusion_err_to_string())?,
        ))
    }
}

#[inline]
unsafe fn transform_pg_agg_to_df_agg(func_oid: pg_sys::Oid) -> BuiltInAgg {
    let func_name = pg_sys::get_func_name(func_oid);
    let func_name_str = CStr::from_ptr(func_name).to_string_lossy().into_owned();

    match func_name_str.as_str() {
        "sum" => BuiltInAgg::Sum,
        "avg" => BuiltInAgg::Avg,
        "count" => BuiltInAgg::Count,
        "max" => BuiltInAgg::Max,
        "min" => BuiltInAgg::Min,
        _ => todo!(),
    }
}

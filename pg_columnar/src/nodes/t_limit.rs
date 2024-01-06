use datafusion::common::ScalarValue;
use datafusion::logical_expr::{Expr, Limit, LogicalPlan};
use pgrx::*;

use crate::nodes::producer::DatafusionExprProducer;
use crate::nodes::producer::DatafusionPlanProducer;
use crate::nodes::t_const::ConstNode;

pub struct LimitNode;
impl DatafusionPlanProducer for LimitNode {
    unsafe fn datafusion_plan(
        plan: *mut pg_sys::Plan,
        rtable: *mut pg_sys::List,
        outer_plan: Option<LogicalPlan>,
    ) -> Result<LogicalPlan, String> {
        let outer_plan = outer_plan.ok_or("Limit does not have an outer plan")?;

        let limit_node = plan as *mut pg_sys::Limit;
        let skip_node = (*limit_node).limitOffset;
        let fetch_node = (*limit_node).limitCount;

        let skip = match skip_node.is_null() {
            true => 0,
            false => match ConstNode::datafusion_expr(skip_node, Some(rtable))? {
                Expr::Literal(ScalarValue::Int64(Some(s))) => s as usize,
                Expr::Literal(ScalarValue::Int32(Some(s))) => s as usize,
                Expr::Literal(ScalarValue::Int16(Some(s))) => s as usize,
                _ => {
                    return Err("Could not unwrap OFFSET".to_string());
                }
            },
        };

        let fetch = match fetch_node.is_null() {
            true => None,
            false => match ConstNode::datafusion_expr(fetch_node, Some(rtable))? {
                Expr::Literal(ScalarValue::Int64(Some(f))) => Some(f as usize),
                Expr::Literal(ScalarValue::Int32(Some(f))) => Some(f as usize),
                Expr::Literal(ScalarValue::Int16(Some(f))) => Some(f as usize),
                _ => {
                    return Err("Could not unwrap LIMIT".to_string());
                }
            },
        };

        Ok(LogicalPlan::Limit(Limit {
            skip,
            fetch,
            input: Box::new(outer_plan).into(),
        }))
    }
}

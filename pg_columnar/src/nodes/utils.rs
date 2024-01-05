use datafusion::logical_expr::{Expr, LogicalPlan};
use pgrx::*;

pub trait DatafusionPlanProducer {
    unsafe fn datafusion_plan(
        plan: *mut pg_sys::Plan,
        rtable: *mut pg_sys::List,
        outer_plan: Option<LogicalPlan>,
    ) -> Result<LogicalPlan, String>;
}

pub trait DatafusionExprProducer {
    unsafe fn datafusion_expr(
        node: *mut pg_sys::Node,
        rtable: Option<*mut pg_sys::List>,
    ) -> Result<Expr, String>;
}

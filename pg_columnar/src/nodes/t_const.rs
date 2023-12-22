use datafusion::common::ScalarValue;
use datafusion::logical_expr::Expr;
use pgrx::*;

use crate::nodes::utils::DatafusionExprTranslator;

pub struct ConstNode;
impl DatafusionExprTranslator for ConstNode {
    unsafe fn datafusion_expr(
        node: *mut pg_sys::Node,
        _rtable: Option<*mut pg_sys::List>,
    ) -> Result<Expr, String> {
        let constnode = node as *mut pg_sys::Const;

        let constval = (*constnode).constvalue;
        let _consttype = (*constnode).consttype;
        let _constisnull = (*constnode).constisnull;

        Ok(Expr::Literal(ScalarValue::Int32(Some(
            constval.value() as i32
        ))))
    }
}

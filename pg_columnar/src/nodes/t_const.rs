use datafusion::logical_expr::Expr;
use pgrx::*;

use crate::nodes::utils::DatafusionExprTranslator;
use crate::tableam::utils::datum_to_expr;

pub struct ConstNode;
impl DatafusionExprTranslator for ConstNode {
    unsafe fn datafusion_expr(
        node: *mut pg_sys::Node,
        _rtable: Option<*mut pg_sys::List>,
    ) -> Result<Expr, String> {
        let constnode = node as *mut pg_sys::Const;

        let mut constval = (*constnode).constvalue;
        let consttype = (*constnode).consttype;
        let constisnull = (*constnode).constisnull;

        datum_to_expr(
            &mut constval as *mut pg_sys::Datum,
            PgOid::from(consttype),
            constisnull,
        )
    }
}

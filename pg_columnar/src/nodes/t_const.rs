use datafusion::logical_expr::Expr;
use pgrx::*;

use crate::datafusion::substrait::{DatafusionMap, DatafusionMapProducer, SubstraitTranslator};
use crate::nodes::producer::DatafusionExprProducer;

pub struct ConstNode;
impl DatafusionExprProducer for ConstNode {
    unsafe fn datafusion_expr(
        node: *mut pg_sys::Node,
        _rtable: Option<*mut pg_sys::List>,
    ) -> Result<Expr, String> {
        let constnode = node as *mut pg_sys::Const;

        let mut constval = (*constnode).constvalue;
        let consttype = (*constnode).consttype;
        let constisnull = (*constnode).constisnull;

        DatafusionMapProducer::map(
            PgOid::from(consttype).to_substrait()?,
            |df_map: DatafusionMap| (df_map.literal)(&mut constval, constisnull),
        )
    }
}

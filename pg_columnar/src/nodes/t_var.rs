use datafusion::logical_expr::Expr;
use pgrx::*;
use std::cmp::max;
use std::ffi::CStr;

use crate::nodes::utils::DatafusionExprTranslator;

pub struct VarNode;
impl DatafusionExprTranslator for VarNode {
    unsafe fn datafusion_expr(
        node: *mut pg_sys::Node,
        rtable: Option<*mut pg_sys::List>,
    ) -> Result<Expr, String> {
        let var = node as *mut pg_sys::Var;
        if let Some(r) = rtable {
            let varno = max(1, (*var).varno);
            let rte = pg_sys::pgrx_list_nth(r, varno - 1) as *mut pg_sys::RangeTblEntry;
            let var_relid = (*rte).relid;
            let att_name = pg_sys::get_attname(var_relid, (*var).varattno, false);
            let att_name_str = CStr::from_ptr(att_name).to_string_lossy().into_owned();

            Ok(Expr::Column(att_name_str.into()))
        } else {
            Err("No range table provided".into())
        }
    }
}

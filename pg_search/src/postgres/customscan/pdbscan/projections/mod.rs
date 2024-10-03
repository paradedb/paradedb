pub mod score;
pub mod snippet;

use pgrx::callconv::{Arg, ArgAbi};
use pgrx::pgrx_sql_entity_graph::metadata::{
    ArgumentError, Returns, ReturnsError, SqlMapping, SqlTranslatable,
};

pub struct OpaqueRecordArg;

unsafe impl<'fcx> ArgAbi<'fcx> for OpaqueRecordArg {
    unsafe fn unbox_arg_unchecked(arg: Arg<'_, 'fcx>) -> Self {
        OpaqueRecordArg
    }
}

unsafe impl SqlTranslatable for OpaqueRecordArg {
    fn argument_sql() -> Result<SqlMapping, ArgumentError> {
        Ok(SqlMapping::As("record".into()))
    }

    fn return_sql() -> Result<Returns, ReturnsError> {
        Ok(Returns::One(SqlMapping::As("record".into())))
    }
}

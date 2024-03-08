use pgrx::*;

use std::ffi::CStr;

use crate::datafusion::session::Session;
use crate::errors::ParadeError;

pub unsafe fn createfunction(
    createfunction_stmt: *mut pg_sys::CreateFunctionStmt,
) -> Result<(), ParadeError> {
    // Drop any functions with the same name from the context and allow the next call to the UDF load them back

    let funcname = pg_sys::NameListToString((*createfunction_stmt).funcname);
    let _funcname_cstr = CStr::from_ptr(funcname);

    Session::with_session_context(|_context| {
        // TODO: need to deregister all UDFs of the same name from context
        //       this is necessary because the function signature might have changed
        //       and it will only be updated in the context if we deregister it first.
        // https://github.com/apache/arrow-datafusion/pull/9239

        Box::pin(async move { Ok(()) })
    })?;

    Ok(())
}

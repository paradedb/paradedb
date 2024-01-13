use pgrx::pg_sys::NodeTag;
use pgrx::*;
use std::ffi::CStr;

use crate::errors::ParadeError;
use crate::hooks::vacuum::vacuum_columnar;

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub fn process_utility(
    pstmt: PgBox<pg_sys::PlannedStmt>,
    query_string: &CStr,
    read_only_tree: Option<bool>,
    context: pg_sys::ProcessUtilityContext,
    params: PgBox<pg_sys::ParamListInfoData>,
    query_env: PgBox<pg_sys::QueryEnvironment>,
    dest: PgBox<pg_sys::DestReceiver>,
    completion_tag: *mut pg_sys::QueryCompletion,
    prev_hook: fn(
        pstmt: PgBox<pg_sys::PlannedStmt>,
        query_string: &CStr,
        read_only_tree: Option<bool>,
        context: pg_sys::ProcessUtilityContext,
        params: PgBox<pg_sys::ParamListInfoData>,
        query_env: PgBox<pg_sys::QueryEnvironment>,
        dest: PgBox<pg_sys::DestReceiver>,
        completion_tag: *mut pg_sys::QueryCompletion,
    ) -> HookResult<()>,
) -> Result<(), ParadeError> {
    let plan = pstmt.utilityStmt;

    #[allow(clippy::single_match)]
    match unsafe { (*plan).type_ } {
        NodeTag::T_VacuumStmt => unsafe {
            let vacuum_stmt = plan as *mut pg_sys::VacuumStmt;
            vacuum_columnar(vacuum_stmt)?;
        },
        _ => {}
    };

    let _ = prev_hook(
        pstmt.clone(),
        query_string,
        read_only_tree,
        context,
        params,
        query_env,
        dest,
        completion_tag,
    );

    Ok(())
}

use async_std::task;
use pgrx::*;
use std::ffi::CStr;
use thiserror::Error;

use super::explain::*;

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
) -> Result<(), ProcessHookError> {
    let plan = pstmt.utilityStmt;
    let pg_plan = pstmt.clone().into_pg();

    if pg_sys::NodeTag::T_ExplainStmt == unsafe { (*plan).type_ } {
        if let Ok(true) = unsafe { task::block_on(explain(pg_plan, query_string, &dest)) } {
            return Ok(());
        }
    }

    prev_hook(
        pstmt,
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

#[derive(Error, Debug)]
pub enum ProcessHookError {
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),
}

use async_std::task;
use pgrx::pg_sys::NodeTag;
use pgrx::*;
use std::ffi::CStr;

use crate::datafusion::commit::{commit_writer, needs_commit};
use crate::datafusion::query::{ASTVec, QueryString};
use crate::errors::ParadeError;
use crate::hooks::alter::alter;
use crate::hooks::drop::drop;
use crate::hooks::query::Query;
use crate::hooks::rename::rename;
use crate::hooks::truncate::truncate;
use crate::hooks::vacuum::vacuum;

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
    if needs_commit()? {
        task::block_on(commit_writer())?;
    }

    unsafe {
        let plan = pstmt.utilityStmt;

        // Parse the query into an AST
        let pg_plan = pstmt.clone().into_pg();
        let query = pg_plan.get_query_string(query_string)?;

        let ast = ASTVec::try_from(QueryString(&query));

        match (*plan).type_ {
            NodeTag::T_AlterTableStmt => {
                if let Ok(ASTVec(ast)) = ast {
                    task::block_on(alter(plan as *mut pg_sys::AlterTableStmt, &ast[0]))?;
                }
            }
            NodeTag::T_DropStmt => {
                drop(plan as *mut pg_sys::DropStmt)?;
            }
            NodeTag::T_RenameStmt => {
                if let Ok(ASTVec(ast)) = ast {
                    rename(plan as *mut pg_sys::RenameStmt, &ast[0])?;
                }
            }
            NodeTag::T_TruncateStmt => {
                truncate(plan as *mut pg_sys::TruncateStmt)?;
            }
            NodeTag::T_VacuumStmt => {
                vacuum(plan as *mut pg_sys::VacuumStmt)?;
            }
            _ => {}
        };

        let _ = prev_hook(
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
}

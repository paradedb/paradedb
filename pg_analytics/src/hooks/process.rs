use crate::datafusion::query::{ASTVec, QueryString};
use async_std::task;
use deltalake::datafusion::sql::parser::Statement;
use pgrx::pg_sys::{AsPgCStr, NodeTag};
use pgrx::*;
use std::ffi::CStr;
use thiserror::Error;

use super::alter::{alter, AlterHookError};
use super::drop::{drop, DropHookError};
use super::executor::{ExecutableDatafusion, ExecutorHookError};
use super::query::{Query, QueryStringError};
use super::rename::{rename, RenameHookError};
use super::truncate::{truncate, TruncateHookError};
use crate::datafusion::catalog::CatalogError;
use crate::datafusion::udf::{createfunction, UDFError};

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
            NodeTag::T_CreateFunctionStmt => {
                createfunction(plan as *mut pg_sys::CreateFunctionStmt)?;
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
                task::block_on(truncate(plan as *mut pg_sys::TruncateStmt))?;
            }
            NodeTag::T_ExplainStmt => {
                if let Ok(ASTVec(ast)) = ast {
                    if let Statement::Explain(estmt) = &ast[0] {
                        let inner_query_string = estmt.statement.to_string();

                        // Get the query desc for the explained query
                        let stmt = plan as *mut pg_sys::ExplainStmt;
                        let explained_query_tree = (*stmt).query as *mut pg_sys::Query;
                        let es = pg_sys::NewExplainState();
                        (*es).format = pg_sys::ExplainFormat_EXPLAIN_FORMAT_TEXT;
                        let internal_pstmt: *mut pg_sys::PlannedStmt;
                        #[cfg(feature = "pg12")]
                        {
                            internal_pstmt = pg_sys::pg_plan_query(
                                explained_query_tree,
                                pg_sys::CURSOR_OPT_PARALLEL_OK.try_into().unwrap(),
                                params.as_ptr(),
                            );
                        }
                        #[cfg(any(
                            feature = "pg13",
                            feature = "pg14",
                            feature = "pg15",
                            feature = "pg16"
                        ))]
                        {
                            internal_pstmt = pg_sys::pg_plan_query(
                                explained_query_tree,
                                query.clone().as_pg_cstr(),
                                pg_sys::CURSOR_OPT_PARALLEL_OK.try_into().unwrap(),
                                params.as_ptr(),
                            );
                        }
                        let query_desc = pg_sys::CreateQueryDesc(
                            internal_pstmt,
                            query.clone().as_pg_cstr(),
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            dest.as_ptr(),
                            params.as_ptr(),
                            query_env.as_ptr(),
                            0,
                        );

                        // If successfully get logical plan details, then that means we use the datafusion plan
                        if let Some(logical_plan_details) = pgbox::PgBox::from_pg(query_desc)
                            .try_get_logical_plan_details(&inner_query_string)?
                        {
                            let es = pg_sys::NewExplainState();
                            pg_sys::appendStringInfoString(
                                (*es).str_,
                                format!("{:?}", logical_plan_details.logical_plan()).as_pg_cstr(),
                            );
                            let tupdesc = pg_sys::CreateTemplateTupleDesc(1);
                            pg_sys::TupleDescInitEntry(
                                tupdesc,
                                1,
                                "QUERY PLAN".as_pg_cstr(),
                                pg_sys::TEXTOID,
                                -1,
                                0,
                            );
                            let tstate = pg_sys::begin_tup_output_tupdesc(
                                dest.as_ptr(),
                                tupdesc,
                                &pg_sys::TTSOpsVirtual,
                            );
                            pg_sys::do_text_output_multiline(tstate, (*(*es).str_).data);
                            pg_sys::end_tup_output(tstate);
                            pg_sys::pfree((*(*es).str_).data as *mut std::ffi::c_void);

                            // Don't execute prev_hook for EXPLAIN
                            return Ok(());
                        }
                    }
                }
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

#[derive(Error, Debug)]
pub enum ProcessHookError {
    #[error(transparent)]
    Catalog(#[from] CatalogError),

    #[error(transparent)]
    AlterHook(#[from] AlterHookError),

    #[error(transparent)]
    DropHook(#[from] DropHookError),

    #[error(transparent)]
    ExecutorHook(#[from] ExecutorHookError),

    #[error(transparent)]
    QueryString(#[from] QueryStringError),

    #[error(transparent)]
    RenameHook(#[from] RenameHookError),

    #[error(transparent)]
    TruncateHook(#[from] TruncateHookError),

    #[error(transparent)]
    Udf(#[from] UDFError),
}

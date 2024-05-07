use deltalake::datafusion::sql::parser::Statement;
use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use std::num::TryFromIntError;
use thiserror::Error;

use crate::hooks::executor::ExecutableDataFrame;

pub unsafe fn explain(
    explain_stmt: *mut pg_sys::ExplainStmt,
    statement: &Statement,
    query: &str,
    params: &PgBox<pg_sys::ParamListInfoData>,
    query_env: &PgBox<pg_sys::QueryEnvironment>,
    dest: &PgBox<pg_sys::DestReceiver>,
) -> Result<bool, ExplainHookError> {
    if let Statement::Explain(estmt) = statement {
        let inner_query_string = estmt.statement.to_string();

        // Get the query desc for the explained query
        let explained_query_tree = (*explain_stmt).query as *mut pg_sys::Query;
        let es = pg_sys::NewExplainState();
        (*es).format = pg_sys::ExplainFormat_EXPLAIN_FORMAT_TEXT;
        let internal_pstmt: *mut pg_sys::PlannedStmt;

        #[cfg(feature = "pg12")]
        {
            internal_pstmt = pg_sys::pg_plan_query(
                explained_query_tree,
                pg_sys::CURSOR_OPT_PARALLEL_OK.try_into()?,
                params.as_ptr(),
            );
        }
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        {
            internal_pstmt = pg_sys::pg_plan_query(
                explained_query_tree,
                query.as_pg_cstr(),
                pg_sys::CURSOR_OPT_PARALLEL_OK.try_into()?,
                params.as_ptr(),
            );
        }

        let query_desc = pg_sys::CreateQueryDesc(
            internal_pstmt,
            inner_query_string.clone().as_pg_cstr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            dest.as_ptr(),
            params.as_ptr(),
            query_env.as_ptr(),
            0,
        );

        // If successfully get logical plan details, then that means we use the datafusion plan
        if let Ok(Some(df)) = pgbox::PgBox::from_pg(query_desc).try_get_dataframe() {
            let logical_plan = df.logical_plan();
            let es = pg_sys::NewExplainState();
            pg_sys::appendStringInfoString((*es).str_, format!("{:?}", logical_plan).as_pg_cstr());
            let tupdesc = pg_sys::CreateTemplateTupleDesc(1);
            pg_sys::TupleDescInitEntry(
                tupdesc,
                1,
                "QUERY PLAN".as_pg_cstr(),
                pg_sys::TEXTOID,
                -1,
                0,
            );
            let tstate =
                pg_sys::begin_tup_output_tupdesc(dest.as_ptr(), tupdesc, &pg_sys::TTSOpsVirtual);
            pg_sys::do_text_output_multiline(tstate, (*(*es).str_).data);
            pg_sys::end_tup_output(tstate);
            pg_sys::pfree((*(*es).str_).data as *mut std::ffi::c_void);

            // Explained datafusion logical plan
            return Ok(true);
        }
    }

    Ok(false)
}

#[derive(Error, Debug)]
pub enum ExplainHookError {
    #[error(transparent)]
    TryFromIntError(#[from] TryFromIntError),
}

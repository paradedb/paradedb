use async_std::task;
use deltalake::datafusion::logical_expr::{DdlStatement, LogicalPlan};
use pgrx::*;
use std::ffi::CStr;

use crate::datafusion::commit::{commit_writer, needs_commit};
use crate::errors::{NotSupported, ParadeError};
use crate::hooks::handler::IsColumn;
use crate::hooks::insert::insert;
use crate::hooks::query::Query;
use crate::hooks::select::select;

pub fn executor_run(
    query_desc: PgBox<pg_sys::QueryDesc>,
    direction: pg_sys::ScanDirection,
    count: u64,
    execute_once: bool,
    prev_hook: fn(
        query_desc: PgBox<pg_sys::QueryDesc>,
        direction: pg_sys::ScanDirection,
        count: u64,
        execute_once: bool,
    ) -> HookResult<()>,
) -> Result<(), ParadeError> {
    if needs_commit()? {
        task::block_on(commit_writer())?;
    }

    unsafe {
        let ps = query_desc.plannedstmt;
        let rtable = (*ps).rtable;
        let pg_plan = query_desc.plannedstmt;
        let query = pg_plan.get_query_string(CStr::from_ptr(query_desc.sourceText))?;

        if query_desc.operation == pg_sys::CmdType_CMD_INSERT {
            insert(rtable, query_desc.clone())?;
        }

        // Only use this hook for deltalake tables
        // Allow INSERTs to go through
        if rtable.is_null()
            || query_desc.operation == pg_sys::CmdType_CMD_INSERT
            || !rtable.is_column()?
            // Tech Debt: Find a less hacky way to let COPY go through
            || query.to_lowercase().starts_with("copy")
        {
            prev_hook(query_desc, direction, count, execute_once);
            return Ok(());
        }

        // Parse the query into a LogicalPlan
        let logical_plan = pg_plan.get_logical_plan(&query);

        // CREATE TABLE queries can reach the executor for CREATE TABLE AS SELECT
        // We should let these queries go through to the table access method
        if let Ok(LogicalPlan::Ddl(DdlStatement::CreateMemoryTable(_))) = logical_plan {
            prev_hook(query_desc, direction, count, execute_once);
            return Ok(());
        }

        // Execute SELECT, DELETE, UPDATE
        match query_desc.operation {
            pg_sys::CmdType_CMD_SELECT => {
                if let Ok(logical_plan) = logical_plan {
                    select(query_desc, logical_plan)?;
                }
            }
            pg_sys::CmdType_CMD_UPDATE => return Err(NotSupported::Update.into()),
            _ => {
                prev_hook(query_desc, direction, count, execute_once);
            }
        };

        Ok(())
    }
}

use async_std::task;
use deltalake::datafusion::logical_expr::{DdlStatement, LogicalPlan};
use pgrx::*;
use std::ffi::CStr;
use thiserror::Error;

use crate::datafusion::catalog::CatalogError;
use crate::datafusion::plan::LogicalPlanDetails;
use crate::datafusion::query::QueryString;
use crate::federation::handler::{get_federated_batches, FederatedHandlerError};
use crate::federation::{COLUMN_FEDERATION_KEY, ROW_FEDERATION_KEY};

use super::handler::{HandlerError, TableClassifier};
use super::query::{Query, QueryStringError};
use super::select::{get_datafusion_batches, write_batches_to_slots, SelectHookError};

macro_rules! fallback_warning {
    ($msg:expr) => {
        warning!("This query was not pushed down to DataFusion because DataFusion returned an error: {}. Query times may be impacted.", $msg);
    };
}

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
) -> Result<(), ExecutorHookError> {
    unsafe {
        let ps = query_desc.plannedstmt;
        let rtable = (*ps).rtable;
        let pg_plan = query_desc.plannedstmt;
        let query = pg_plan.get_query_string(CStr::from_ptr(query_desc.sourceText))?;

        let classified_tables = rtable.table_lists()?;
        let col_tables = classified_tables
            .get(COLUMN_FEDERATION_KEY)
            .ok_or(ExecutorHookError::ColumnListNotFound)?;
        let row_tables = classified_tables
            .get(ROW_FEDERATION_KEY)
            .ok_or(ExecutorHookError::ColumnListNotFound)?;

        // Only use this hook for deltalake tables
        // Allow INSERTs to go through
        if rtable.is_null()
            || query_desc.operation == pg_sys::CmdType_CMD_INSERT
            || col_tables.is_empty()
            // Tech Debt: Find a less hacky way to let COPY go through
            || query.to_lowercase().starts_with("copy")
        {
            prev_hook(query_desc, direction, count, execute_once);
            return Ok(());
        }

        // If tables of different types are both present in the query, federate the query.
        if !row_tables.is_empty() && !col_tables.is_empty() {
            if query_desc.operation != pg_sys::CmdType_CMD_SELECT {
                prev_hook(query_desc, direction, count, execute_once);
                return Ok(());
            }

            match task::block_on(get_federated_batches(query, classified_tables)) {
                Ok(batches) => write_batches_to_slots(query_desc, batches)?,
                Err(err) => {
                    fallback_warning!(err.to_string());
                    prev_hook(query_desc, direction, count, execute_once);
                    return Ok(());
                }
            };
        } else {
            // Parse the query into a LogicalPlan
            match LogicalPlanDetails::try_from(QueryString(&query)) {
                Ok(logical_plan_details) => {
                    let logical_plan = logical_plan_details.logical_plan();

                    // CREATE TABLE queries can reach the executor for CREATE TABLE AS SELECT
                    // We should let these queries go through to the table access method
                    match logical_plan {
                        LogicalPlan::Ddl(DdlStatement::CreateMemoryTable(_))
                        | LogicalPlan::Ddl(DdlStatement::CreateView(_)) => {
                            prev_hook(query_desc, direction, count, execute_once);
                            return Ok(());
                        }
                        _ => {}
                    };

                    // Execute SELECT, DELETE, UPDATE
                    match query_desc.operation {
                        pg_sys::CmdType_CMD_SELECT => {
                            let single_thread = logical_plan_details.includes_udf();
                            match get_datafusion_batches(logical_plan, single_thread) {
                                Ok(batches) => write_batches_to_slots(query_desc, batches)?,
                                Err(err) => {
                                    fallback_warning!(err.to_string());
                                    prev_hook(query_desc, direction, count, execute_once);
                                    return Ok(());
                                }
                            };
                        }
                        pg_sys::CmdType_CMD_UPDATE => {
                            return Err(ExecutorHookError::UpdateNotSupported);
                        }
                        _ => {
                            prev_hook(query_desc, direction, count, execute_once);
                        }
                    }
                }
                Err(err) => {
                    fallback_warning!(err.to_string());
                    prev_hook(query_desc, direction, count, execute_once);
                }
            };
        }

        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum ExecutorHookError {
    #[error(transparent)]
    CatalogError(#[from] CatalogError),

    #[error(transparent)]
    FederatedHandlerError(#[from] FederatedHandlerError),

    #[error(transparent)]
    HandlerError(#[from] HandlerError),

    #[error(transparent)]
    SelectHookError(#[from] SelectHookError),

    #[error(transparent)]
    QueryStringError(#[from] QueryStringError),

    #[error("Table classifier did not return a column list")]
    ColumnListNotFound,

    #[error("UPDATE is not currently supported because Parquet tables are append only.")]
    UpdateNotSupported,
}

use async_std::task;
use deltalake::datafusion::common::DataFusionError;
use deltalake::datafusion::dataframe::DataFrame;
use deltalake::datafusion::logical_expr::{DdlStatement, LogicalPlan};
use pgrx::*;
use std::ffi::CStr;
use thiserror::Error;

use crate::datafusion::catalog::CatalogError;
use crate::datafusion::plan::LogicalPlanDetails;
use crate::datafusion::query::{QueryParserError, QueryString};
use crate::federation::handler::{get_federated_dataframe, FederatedHandlerError};
use crate::federation::{COLUMN_FEDERATION_KEY, ROW_FEDERATION_KEY};

use super::handler::{HandlerError, TableClassifier};
use super::query::{Query, QueryStringError};
use super::select::{get_datafusion_dataframe, write_batches_to_slots, SelectHookError};

macro_rules! fallback_warning {
    ($msg:expr) => {
        warning!("This query was not pushed down to DataFusion because DataFusion returned an error: {}. Query times may be impacted.", $msg);
    };
}

pub trait ExecutableDataFrame {
    fn try_get_dataframe(&self) -> Result<Option<DataFrame>, ExecutorHookError>;
}

impl ExecutableDataFrame for PgBox<pg_sys::QueryDesc> {
    fn try_get_dataframe(&self) -> Result<Option<DataFrame>, ExecutorHookError> {
        unsafe {
            let ps = self.plannedstmt;
            let rtable = (*ps).rtable;
            let pg_plan = self.plannedstmt;
            let query = pg_plan.get_query_string(CStr::from_ptr(self.sourceText))?;

            let classified_tables = rtable.table_lists()?;
            let col_tables = classified_tables
                .get(COLUMN_FEDERATION_KEY)
                .ok_or(ExecutorHookError::ColumnListNotFound)?;
            let row_tables = classified_tables
                .get(ROW_FEDERATION_KEY)
                .ok_or(ExecutorHookError::RowListNotFound)?;

            // Only use this hook for deltalake tables
            // Allow INSERTs to go through
            if rtable.is_null()
                || self.operation == pg_sys::CmdType_CMD_INSERT
                || col_tables.is_empty()
                // Tech Debt: Find a less hacky way to let COPY go through
                || query.to_lowercase().starts_with("copy")
            {
                return Ok(None);
            }

            // If tables of different types are both present in the query, federate the query.
            if !row_tables.is_empty() && !col_tables.is_empty() {
                Ok(if self.operation != pg_sys::CmdType_CMD_SELECT {
                    None
                } else {
                    Some(task::block_on(get_federated_dataframe(
                        query,
                        classified_tables,
                    ))?)
                })
            } else {
                // Parse the query into a LogicalPlan
                let logical_plan_details = LogicalPlanDetails::try_from(QueryString(&query))?;
                let logical_plan = logical_plan_details.logical_plan();

                // CREATE TABLE queries can reach the executor for CREATE TABLE AS SELECT
                // We should let these queries go through to the table access method
                match logical_plan {
                    LogicalPlan::Ddl(DdlStatement::CreateMemoryTable(_))
                    | LogicalPlan::Ddl(DdlStatement::CreateView(_)) => {
                        return Ok(None);
                    }
                    _ => {}
                };

                // Execute SELECT, DELETE, UPDATE
                match self.operation {
                    pg_sys::CmdType_CMD_SELECT => {
                        let single_thread = logical_plan_details.includes_udf();
                        Ok(Some(get_datafusion_dataframe(logical_plan, single_thread)?))
                    }
                    pg_sys::CmdType_CMD_UPDATE => Err(ExecutorHookError::UpdateNotSupported),
                    _ => Ok(None),
                }
            }
        }
    }
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
    match query_desc.try_get_dataframe() {
        Ok(Some(df)) => {
            let batches = task::block_on(df.collect())?;
            write_batches_to_slots(query_desc, batches)?;
        }
        other => {
            if let Err(err) = other {
                fallback_warning!(err.to_string());
            }
            prev_hook(query_desc, direction, count, execute_once);
        }
    }

    Ok(())
}

#[derive(Error, Debug)]
pub enum ExecutorHookError {
    #[error(transparent)]
    CatalogError(#[from] CatalogError),

    #[error(transparent)]
    DataFusionError(#[from] DataFusionError),

    #[error(transparent)]
    FederatedHandlerError(#[from] FederatedHandlerError),

    #[error(transparent)]
    HandlerError(#[from] HandlerError),

    #[error(transparent)]
    SelectHookError(#[from] SelectHookError),

    #[error(transparent)]
    QueryParserError(#[from] QueryParserError),

    #[error(transparent)]
    QueryStringError(#[from] QueryStringError),

    #[error("Table classifier did not return a column list")]
    ColumnListNotFound,

    #[error("Table classifier did not return a row list")]
    RowListNotFound,

    #[error("UPDATE is not currently supported because Parquet tables are append only.")]
    UpdateNotSupported,
}

use datafusion::common::arrow::array::RecordBatch;
use datafusion::common::DataFusionError;
use datafusion::logical_expr::LogicalPlan;
use pgrx::*;
use std::ffi::CStr;
use thiserror::Error;

use crate::datafusion::context::ContextError;
use crate::datafusion::plan::QueryString;
use crate::datafusion::session::Session;

use super::query::QueryType;

// use crate::datafusion::catalog::CatalogError;
// use crate::datafusion::plan::LogicalPlanDetails;
// use crate::datafusion::query::QueryString;
// use crate::federation::handler::{get_federated_batches, FederatedHandlerError};

// use super::handler::{HandlerError, TableClassifier};
// use super::query::{Query, QueryStringError};
// use super::select::{get_datafusion_batches, write_batches_to_slots, SelectHookError};

macro_rules! fallback_warning {
    ($msg:expr) => {
        warning!("This query was not pushed down to DataFusion because DataFusion returned an error: {}. Query times may be impacted.", $msg);
    };
}

pub unsafe fn executor_run(
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
    prev_hook(query_desc, direction, count, execute_once);
    return Ok(());
    // let ps = query_desc.plannedstmt;
    // let rtable = (*ps).rtable;
    // let pg_plan = query_desc.plannedstmt;
    // let QueryString(query) = QueryString::try_from(query_desc.clone())?;
    // let query_type = QueryType::from(query_desc.clone());

    // // Only use this hook for deltalake tables
    // // Allow INSERTs to go through
    // if rtable.is_null()
    //     || query_desc.operation != pg_sys::CmdType_CMD_SELECT
    //     || query_type != QueryType::DataFusion
    //     // Tech Debt: Find a less hacky way to let COPY go through
    //     || query.to_lowercase().starts_with("copy")
    // {
    //     prev_hook(query_desc, direction, count, execute_once);
    //     return Ok(());
    // }

    // // Parse the query into a LogicalPlan
    // match LogicalPlan::try_from(QueryString(&query)) {
    //     Ok(logical_plan) => {
    //         // Don't intercept any DDL or DML statements
    //         match logical_plan {
    //             LogicalPlan::Ddl(_) | LogicalPlan::Dml(_) => {
    //                 prev_hook(query_desc, direction, count, execute_once);
    //                 return Ok(());
    //             }
    //             _ => {}
    //         };

    //         // Execute SELECT
    //         // match get_datafusion_batches(logical_plan) {
    //         //     Ok(batches) => write_batches_to_slots(query_desc, batches)?,
    //         //     Err(err) => {
    //         //         fallback_warning!(err.to_string());
    //         //         prev_hook(query_desc, direction, count, execute_once);
    //         //         return Ok(());
    //         //     }
    //         // };
    //         // fallback_warning!(err.to_string());
    //         prev_hook(query_desc, direction, count, execute_once);
    //         return Ok(());
    //     }
    //     Err(_) => {
    //         prev_hook(query_desc, direction, count, execute_once);
    //     }
    // };

    // Ok(())
}

#[inline]
fn get_datafusion_batches(logical_plan: LogicalPlan) -> Result<Vec<RecordBatch>, ContextError> {
    // Execute the logical plan and collect the resulting batches
    Ok(Session::with_session_context(|context| {
        Box::pin(async move {
            let dataframe = context.execute_logical_plan(logical_plan).await?;
            Ok(dataframe.collect().await?)
        })
    })?)
}

#[derive(Error, Debug)]
pub enum ExecutorHookError {
    #[error(transparent)]
    ContextError(#[from] ContextError),

    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
}

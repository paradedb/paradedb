use async_std::task;
use datafusion::arrow::error::ArrowError;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::DataFusionError;
use datafusion::datasource::{provider_as_source, TableProvider};
use datafusion::execution::context::SessionState;
use datafusion::logical_expr::{LogicalPlan, LogicalPlanBuilder};
use datafusion::physical_plan::SendableRecordBatchStream;
use datafusion::prelude::DataFrame;
use pgrx::*;
use std::collections::HashMap;
use std::sync::Arc;
use supabase_wrappers::prelude::*;
use thiserror::Error;

use super::cell::*;
use super::format::*;
use super::object_store::*;
use super::table::*;

// Because the SessionContext is recreated on each scan, we don't need to worry about
// assigning a unique name to the DataFusion table
const DEFAULT_TABLE_NAME: &str = "listing_table";

pub trait BaseFdw {
    // Getter methods
    fn get_current_batch(&self) -> Option<RecordBatch>;
    fn get_current_batch_index(&self) -> usize;
    fn get_target_columns(&self) -> Vec<Column>;
    fn get_session_state(&self) -> SessionState;

    // Setter methods
    fn set_current_batch(&mut self, batch: Option<RecordBatch>);
    fn set_current_batch_index(&mut self, index: usize);
    fn set_stream(&mut self, stream: Option<SendableRecordBatchStream>);
    fn set_target_columns(&mut self, columns: Vec<Column>);

    // DataFusion methods
    async fn execute_logical_plan(&self, plan: LogicalPlan) -> Result<DataFrame, BaseFdwError>;
    async fn get_next_batch(&mut self) -> Result<Option<RecordBatch>, BaseFdwError>;
    fn register_table(
        &mut self,
        name: &str,
        provider: Arc<dyn TableProvider>,
    ) -> Result<Option<Arc<dyn TableProvider>>, BaseFdwError>;

    // Default trait methods
    fn begin_scan_impl(
        &mut self,
        _quals: &[Qual],
        columns: &[Column],
        _sorts: &[Sort],
        limit: &Option<Limit>,
        options: &HashMap<String, String>,
    ) -> Result<(), BaseFdwError> {
        self.set_target_columns(columns.to_vec());

        let oid_map: HashMap<usize, pg_sys::Oid> = columns
            .iter()
            .cloned()
            .map(|col| (col.num - 1, col.type_oid))
            .collect();

        let provider = create_table_provider(options.clone(), oid_map, &self.get_session_state())?;

        self.register_table(DEFAULT_TABLE_NAME, provider.clone())?;

        // Construct LogicalPlan
        let mut logical_plan = LogicalPlanBuilder::scan(
            DEFAULT_TABLE_NAME,
            provider_as_source(provider),
            Some(columns.iter().map(|c| c.num - 1).collect::<Vec<usize>>()),
        )?;

        if let Some(limit) = limit {
            logical_plan = logical_plan.limit(limit.offset as usize, Some(limit.count as usize))?;
        }

        let dataframe = task::block_on(self.execute_logical_plan(logical_plan.build()?))?;
        self.set_stream(Some(task::block_on(dataframe.execute_stream())?));

        Ok(())
    }

    fn iter_scan_impl(&mut self, row: &mut Row) -> Result<Option<()>, BaseFdwError> {
        if self.get_current_batch().is_none()
            || self.get_current_batch_index()
                >= self
                    .get_current_batch()
                    .as_ref()
                    .ok_or(BaseFdwError::BatchNotFound)?
                    .num_rows()
        {
            self.set_current_batch_index(0);
            let next_batch = task::block_on(self.get_next_batch())?;

            if next_batch.is_none() {
                return Ok(None);
            }

            self.set_current_batch(next_batch);
        }

        let current_batch_binding = self.get_current_batch();
        let current_batch = current_batch_binding
            .as_ref()
            .ok_or(BaseFdwError::BatchNotFound)?;
        let current_batch_index = self.get_current_batch_index();

        for (column_index, target_column) in
            self.get_target_columns().clone().into_iter().enumerate()
        {
            let batch_column = current_batch.column(column_index);
            let cell = batch_column.get_cell(current_batch_index, target_column.type_oid)?;
            row.push(target_column.name.as_str(), cell);
        }

        self.set_current_batch_index(current_batch_index + 1);

        Ok(Some(()))
    }

    fn end_scan_impl(&mut self) -> Result<(), BaseFdwError> {
        self.set_stream(None);
        Ok(())
    }
}

impl From<BaseFdwError> for pg_sys::panic::ErrorReport {
    fn from(value: BaseFdwError) -> Self {
        pg_sys::panic::ErrorReport::new(PgSqlErrorCode::ERRCODE_FDW_ERROR, format!("{}", value), "")
    }
}

#[derive(Error, Debug)]
pub enum BaseFdwError {
    #[error(transparent)]
    ArrowError(#[from] ArrowError),

    #[error(transparent)]
    DataFusionError(#[from] DataFusionError),

    #[error(transparent)]
    DataTypeError(#[from] DataTypeError),

    #[error(transparent)]
    FileFormatError(#[from] FileFormatError),

    #[error(transparent)]
    ObjectStoreError(#[from] ObjectStoreError),

    #[error(transparent)]
    OptionsError(#[from] super::options::OptionsError),

    #[error(transparent)]
    SupabaseOptionsError(#[from] supabase_wrappers::options::OptionsError),

    #[error(transparent)]
    TableError(#[from] TableError),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error("Unexpected error: Expected RecordBatch but found None")]
    BatchNotFound,

    #[error("Unexpected error: Expected SendableRecordBatchStream but found None")]
    StreamNotFound,

    #[error("Received unsupported FDW oid {0:?}")]
    UnsupportedFdwOid(pg_sys::Oid),
}

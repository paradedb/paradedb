use async_std::task;
use datafusion::arrow::error::ArrowError;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::DataFusionError;
use datafusion::datasource::{provider_as_source, TableProvider};
use datafusion::execution::context::SessionState;
use datafusion::logical_expr::{LogicalPlan, LogicalPlanBuilder};
use datafusion::physical_plan::SendableRecordBatchStream;
use datafusion::prelude::DataFrame;
use deltalake::DeltaTableError;
use pgrx::*;
use std::collections::HashMap;
use std::sync::Arc;
use supabase_wrappers::prelude::*;
use thiserror::Error;

use crate::datafusion::provider::*;
use crate::types::schema::*;

use super::cell::*;
use super::format::*;
use super::object_store::*;
use super::options::TableOption;

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

        let attribute_map: HashMap<usize, PgAttribute> = columns
            .iter()
            .cloned()
            .map(|col| {
                (
                    col.num - 1,
                    PgAttribute::new(&col.name, col.type_oid, col.type_mod),
                )
            })
            .collect();

        let format = require_option_or(TableOption::Format.as_str(), options, "");
        let provider = match TableFormat::from(format) {
            TableFormat::None => task::block_on(create_listing_provider(
                options.clone(),
                attribute_map,
                &self.get_session_state(),
            ))?,
            TableFormat::Delta => {
                task::block_on(create_delta_provider(options.clone(), attribute_map))?
            }
        };

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
            let cell = batch_column.get_cell(
                current_batch_index,
                target_column.type_oid,
                target_column.type_mod,
            )?;
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
    DeltaTableError(#[from] DeltaTableError),

    #[error(transparent)]
    FormatError(#[from] FormatError),

    #[error(transparent)]
    ObjectStoreError(#[from] ObjectStoreError),

    #[error(transparent)]
    OptionsError(#[from] supabase_wrappers::options::OptionsError),

    #[error(transparent)]
    SchemaError(#[from] SchemaError),

    #[error(transparent)]
    TableProviderError(#[from] TableProviderError),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error("Unexpected error: Expected RecordBatch but found None")]
    BatchNotFound,

    #[error("Unexpected error: Expected SendableRecordBatchStream but found None")]
    StreamNotFound,

    #[error("Received unsupported FDW oid {0:?}")]
    UnsupportedFdwOid(PgOid),
}

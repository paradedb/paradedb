use async_std::stream::StreamExt;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::datasource::TableProvider;
use datafusion::execution::context::SessionState;
use datafusion::logical_expr::LogicalPlan;
use datafusion::physical_plan::SendableRecordBatchStream;
use datafusion::prelude::{DataFrame, SessionContext};
use fdw::options::*;
use object_store::local::LocalFileSystem;
use pgrx::*;
use std::collections::HashMap;
use std::sync::Arc;
use supabase_wrappers::prelude::*;
use url::Url;

use super::fdw::*;

#[wrappers_fdw(
    author = "ParadeDB",
    website = "https://github.com/paradedb/paradedb",
    error_type = "BaseFdwError"
)]
pub(crate) struct LocalFileFdw {
    stream: Option<SendableRecordBatchStream>,
    current_batch: Option<RecordBatch>,
    current_batch_index: usize,
    context: SessionContext,
    target_columns: Vec<Column>,
}

impl BaseFdw for LocalFileFdw {
    fn get_current_batch(&self) -> Option<RecordBatch> {
        self.current_batch.clone()
    }

    fn get_current_batch_index(&self) -> usize {
        self.current_batch_index
    }

    fn get_session_state(&self) -> SessionState {
        self.context.state()
    }

    fn get_target_columns(&self) -> Vec<Column> {
        self.target_columns.clone()
    }

    fn set_current_batch(&mut self, batch: Option<RecordBatch>) {
        self.current_batch = batch;
    }

    fn set_current_batch_index(&mut self, index: usize) {
        self.current_batch_index = index;
    }

    fn set_stream(&mut self, stream: Option<SendableRecordBatchStream>) {
        self.stream = stream;
    }

    fn set_target_columns(&mut self, columns: Vec<Column>) {
        self.target_columns = columns;
    }

    async fn get_next_batch(&mut self) -> Result<Option<RecordBatch>, BaseFdwError> {
        match self
            .stream
            .as_mut()
            .ok_or(BaseFdwError::StreamNotFound)?
            .next()
            .await
        {
            Some(Ok(batch)) => Ok(Some(batch)),
            None => Ok(None),
            Some(Err(err)) => Err(BaseFdwError::DataFusionError(err)),
        }
    }

    async fn execute_logical_plan(&self, plan: LogicalPlan) -> Result<DataFrame, BaseFdwError> {
        Ok(self.context.execute_logical_plan(plan).await?)
    }

    fn register_table(
        &mut self,
        name: &str,
        provider: Arc<dyn TableProvider>,
    ) -> Result<Option<Arc<dyn TableProvider>>, BaseFdwError> {
        Ok(self.context.register_table(name, provider)?)
    }
}

impl ForeignDataWrapper<BaseFdwError> for LocalFileFdw {
    fn new(_options: &HashMap<String, String>) -> Result<Self, BaseFdwError> {
        // Create S3 ObjectStore
        let object_store = LocalFileSystem::new();

        // Create SessionContext with ObjectStore
        let context = SessionContext::new();
        context
            .runtime_env()
            .register_object_store(&Url::parse("file://")?, Arc::new(object_store));

        Ok(Self {
            current_batch: None,
            current_batch_index: 0,
            stream: None,
            target_columns: Vec::new(),
            context,
        })
    }

    fn validator(
        opt_list: Vec<Option<String>>,
        catalog: Option<pg_sys::Oid>,
    ) -> Result<(), BaseFdwError> {
        if let Some(oid) = catalog {
            match oid {
                FOREIGN_DATA_WRAPPER_RELATION_ID => {}
                FOREIGN_SERVER_RELATION_ID => {}
                FOREIGN_TABLE_RELATION_ID => {
                    for opt in TableOption::iter() {
                        if opt.is_required() {
                            check_options_contain(&opt_list, opt.as_str())?;
                        }
                    }
                }
                unsupported => return Err(BaseFdwError::UnsupportedFdwOid(unsupported)),
            }
        }

        Ok(())
    }

    fn begin_scan(
        &mut self,
        _quals: &[Qual],
        columns: &[Column],
        _sorts: &[Sort],
        limit: &Option<Limit>,
        options: &HashMap<String, String>,
    ) -> Result<(), BaseFdwError> {
        self.begin_scan_impl(_quals, columns, _sorts, limit, options)
    }

    fn iter_scan(&mut self, row: &mut Row) -> Result<Option<()>, BaseFdwError> {
        self.iter_scan_impl(row)
    }

    fn end_scan(&mut self) -> Result<(), BaseFdwError> {
        self.end_scan_impl()
    }
}

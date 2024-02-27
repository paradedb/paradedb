use async_std::stream::StreamExt;
use async_std::task;
use async_trait::async_trait;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::datasource::TableProvider;
use deltalake::datafusion::error::Result;
use deltalake::datafusion::execution::context::SessionState;
use deltalake::datafusion::execution::TaskContext;
use deltalake::datafusion::physical_plan::SendableRecordBatchStream;
use deltalake::operations::update::UpdateBuilder;
use deltalake::table::state::DeltaTableState;
use parking_lot::Mutex;
use pgrx::*;
use std::collections::HashMap;
use std::future::IntoFuture;
use std::{
    any::{type_name, Any},
    ffi::CString,
    path::PathBuf,
    sync::Arc,
};

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::directory::ParadeDirectory;
use crate::datafusion::stream::Streams;
use crate::datafusion::table::Tables;
use crate::datafusion::writer::Writers;
use crate::errors::{NotFound, ParadeError};

const BYTES_IN_MB: i64 = 1_048_576;

pub struct ParadeSchemaProvider {
    schema_name: String,
    tables: Arc<Mutex<Tables>>,
    writers: Arc<Mutex<Writers>>,
    streams: Arc<Mutex<Streams>>,
}

impl ParadeSchemaProvider {
    pub async fn try_new(schema_name: &str) -> Result<Self, ParadeError> {
        Ok(Self {
            schema_name: schema_name.to_string(),
            tables: Arc::new(Mutex::new(Tables::new()?)),
            writers: Arc::new(Mutex::new(Writers::new()?)),
            streams: Arc::new(Mutex::new(Streams::new()?)),
        })
    }

    pub fn tables(&self) -> Result<Arc<Mutex<Tables>>, ParadeError> {
        Ok(self.tables.clone())
    }

    pub fn writers(&self) -> Result<Arc<Mutex<Writers>>, ParadeError> {
        Ok(self.writers.clone())
    }

    pub fn streams(&self) -> Result<Arc<Mutex<Streams>>, ParadeError> {
        Ok(self.streams.clone())
    }

    fn table_path(&self, table_name: &str) -> Result<PathBuf, ParadeError> {
        let schema_oid = unsafe {
            pg_sys::get_namespace_oid(CString::new(self.schema_name.clone())?.as_ptr(), true)
        };

        let table_oid =
            unsafe { pg_sys::get_relname_relid(CString::new(table_name)?.as_ptr(), schema_oid) };

        ParadeDirectory::table_path(DatafusionContext::catalog_oid()?, schema_oid, table_oid)
    }
}

#[async_trait]
impl SchemaProvider for ParadeSchemaProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn table_names(&self) -> Vec<String> {
        vec![]
    }

    async fn table(&self, table_name: &str) -> Option<Arc<dyn TableProvider>> {
        let table_path = Self::table_path(self, table_name).unwrap();

        let delta_table = DatafusionContext::with_tables(&self.schema_name, |mut tables| {
            let table_ref = task::block_on(tables.get_ref(table_path))?;
            Ok(task::block_on(
                UpdateBuilder::new(
                    table_ref.log_store(),
                    table_ref
                        .state
                        .clone()
                        .ok_or(NotFound::Value(type_name::<DeltaTableState>().to_string()))?,
                )
                .into_future(),
            )?
            .0)
        })
        .unwrap();

        Some(Arc::new(delta_table.clone()) as Arc<dyn TableProvider>)
    }

    fn table_exist(&self, table_name: &str) -> bool {
        let table_path = Self::table_path(self, table_name).unwrap();

        DatafusionContext::with_tables(&self.schema_name, |tables| tables.contains(&table_path))
            .unwrap()
    }
}

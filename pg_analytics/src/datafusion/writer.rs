use async_std::sync::Mutex;
use deltalake::datafusion::arrow::datatypes::Schema as ArrowSchema;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::kernel::{Action, Add};
use deltalake::operations::transaction::commit as commit_delta;
use deltalake::operations::writer::{DeltaWriter, WriterConfig};
use deltalake::protocol::{DeltaOperation, SaveMode};
use deltalake::DeltaTable;
use once_cell::sync::Lazy;
use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::errors::ParadeError;
use crate::guc::PARADE_GUC;

use super::session::Session;

pub static TRANSACTION_CALLBACK_CACHE_ID: &str = "parade_parquet_table";

const BYTES_IN_MB: i64 = 1_048_576;
const WRITER_ID: &str = "delta_writer";

struct WriterCache {
    writer: DeltaWriter,
    table: DeltaTable,
    schema_name: String,
    table_path: PathBuf,
}

impl WriterCache {
    pub fn new(
        writer: DeltaWriter,
        table: DeltaTable,
        schema_name: &str,
        table_path: &Path,
    ) -> Result<Self, ParadeError> {
        Ok(Self {
            writer,
            table,
            schema_name: schema_name.to_string(),
            table_path: table_path.to_path_buf(),
        })
    }

    pub async fn flush(self) -> Result<Vec<Add>, ParadeError> {
        Ok(self.writer.close().await?)
    }

    pub fn table(&self) -> Result<DeltaTable, ParadeError> {
        Ok(self.table.clone())
    }

    pub fn table_path(&self) -> Result<PathBuf, ParadeError> {
        Ok(self.table_path.clone())
    }

    pub async fn write(&mut self, batch: &RecordBatch) -> Result<(), ParadeError> {
        self.writer.write(batch).await?;
        Ok(())
    }
}

struct ActionCache {
    table: DeltaTable,
    actions: Vec<Add>,
}

impl ActionCache {
    pub fn new(table: DeltaTable, actions: Vec<Add>) -> Result<Self, ParadeError> {
        Ok(Self { table, actions })
    }

    pub async fn commit(self) -> Result<(), ParadeError> {
        commit_delta(
            self.table.log_store().as_ref(),
            &self
                .actions
                .iter()
                .map(|a| Action::Add(a.clone()))
                .collect(),
            DeltaOperation::Write {
                mode: SaveMode::Append,
                partition_by: None,
                predicate: None,
            },
            self.table.state.as_ref(),
            None,
        )
        .await?;

        Ok(())
    }
}

static WRITER_CACHE: Lazy<Arc<Mutex<HashMap<String, WriterCache>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

static ACTIONS_CACHE: Lazy<Arc<Mutex<HashMap<PathBuf, ActionCache>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub struct Writer;

impl Writer {
    pub async fn write(
        schema_name: &str,
        table_path: &Path,
        arrow_schema: Arc<ArrowSchema>,
        batch: &RecordBatch,
    ) -> Result<(), ParadeError> {
        let mut cache = WRITER_CACHE.lock().await;

        let writer_cache = match cache.entry(WRITER_ID.to_string()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => {
                let writer = Self::create(schema_name, table_path, arrow_schema).await?;
                let table_path_cloned = table_path.to_path_buf();
                let provider = Session::with_tables(schema_name, |mut tables| {
                    Box::pin(async move { tables.get_owned(&table_path_cloned).await })
                })?;
                entry.insert(WriterCache::new(
                    writer,
                    provider.table(),
                    schema_name,
                    table_path,
                )?)
            }
        };

        writer_cache.write(batch).await
    }

    pub async fn commit() -> Result<(), ParadeError> {
        let mut actions_cache = ACTIONS_CACHE.lock().await;

        for (_, action_cache) in actions_cache.drain() {
            action_cache.commit().await?;
        }

        Ok(())
    }

    pub async fn flush() -> Result<(), ParadeError> {
        let mut writer_cache = WRITER_CACHE.lock().await;
        let mut actions_cache = ACTIONS_CACHE.lock().await;

        if let Some(writer_cache) = writer_cache.remove(WRITER_ID) {
            let table = writer_cache.table()?;
            let table_path = writer_cache.table_path()?;
            let actions = writer_cache.flush().await?;
            actions_cache.insert(table_path, ActionCache::new(table, actions)?);
        }

        Ok(())
    }

    async fn create(
        schema_name: &str,
        table_path: &Path,
        arrow_schema: Arc<ArrowSchema>,
    ) -> Result<DeltaWriter, ParadeError> {
        let target_file_size = PARADE_GUC.optimize_file_size_mb.get() as i64 * BYTES_IN_MB;

        let writer_config = WriterConfig::new(
            arrow_schema,
            vec![],
            None,
            Some(target_file_size as usize),
            None,
        );

        let table_path = table_path.to_path_buf();
        let provider = Session::with_tables(schema_name, |mut tables| {
            Box::pin(async move { tables.get_owned(&table_path).await })
        })?;

        Ok(DeltaWriter::new(
            provider.table().object_store(),
            writer_config,
        ))
    }
}

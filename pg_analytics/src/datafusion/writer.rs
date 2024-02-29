use async_std::sync::Mutex;
use async_std::task;
use deltalake::datafusion::arrow::datatypes::Schema as ArrowSchema;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::kernel::Action;
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

use crate::datafusion::context::DatafusionContext;
use crate::errors::ParadeError;
use crate::guc::PARADE_GUC;

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

    pub async fn commit(self) -> Result<(String, PathBuf, DeltaTable), ParadeError> {
        let actions = self.writer.close().await?;

        commit_delta(
            self.table.log_store().as_ref(),
            &actions.iter().map(|a| Action::Add(a.clone())).collect(),
            DeltaOperation::Write {
                mode: SaveMode::Append,
                partition_by: None,
                predicate: None,
            },
            self.table.state.as_ref(),
            None,
        )
        .await?;

        Ok((self.schema_name, self.table_path, self.table))
    }

    pub async fn write(&mut self, batch: &RecordBatch) -> Result<(), ParadeError> {
        self.writer.write(batch).await?;
        Ok(())
    }
}

static WRITER_CACHE: Lazy<Arc<Mutex<HashMap<String, WriterCache>>>> =
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
                let table = DatafusionContext::with_tables(schema_name, |mut tables| {
                    task::block_on(tables.get_owned(table_path))
                })?;
                entry.insert(WriterCache::new(writer, table, schema_name, table_path)?)
            }
        };

        writer_cache.write(batch).await
    }

    pub async fn commit() -> Result<Option<(String, PathBuf, DeltaTable)>, ParadeError> {
        let mut cache = WRITER_CACHE.lock().await;

        match cache.remove(WRITER_ID) {
            Some(writer_cache) => Ok(Some(writer_cache.commit().await?)),
            None => Ok(None),
        }
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

        let delta_table = DatafusionContext::with_tables(schema_name, |mut tables| {
            task::block_on(tables.get_owned(table_path))
        })?;

        Ok(DeltaWriter::new(delta_table.object_store(), writer_config))
    }
}

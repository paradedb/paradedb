use async_std::task;
use deltalake::datafusion::arrow::datatypes::Schema as ArrowSchema;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::kernel::Action;
use deltalake::operations::transaction::commit as commit_delta;
use deltalake::operations::writer::{DeltaWriter, WriterConfig};
use deltalake::protocol::{DeltaOperation, SaveMode};
use deltalake::writer::{DeltaWriter as DeltaWriterTrait, RecordBatchWriter, WriteMode};
use deltalake::DeltaTable;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::{
    hash_map::Entry::{self, Occupied, Vacant},
    HashMap,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::datafusion::context::DatafusionContext;
use crate::errors::{NotFound, ParadeError};
use crate::guc::PARADE_GUC;

const BYTES_IN_MB: i64 = 1_048_576;
const WRITER_ID: &str = "delta_writer";

struct WriterCache {
    writer: DeltaWriter,
    table: DeltaTable,
    schema_name: String,
}

impl WriterCache {
    pub fn new(
        writer: DeltaWriter,
        table: DeltaTable,
        schema_name: String,
    ) -> Result<Self, ParadeError> {
        Ok(Self {
            writer,
            table,
            schema_name,
        })
    }

    pub async fn commit(self) -> Result<(String, DeltaTable), ParadeError> {
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

        Ok((self.schema_name, self.table))
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
        batch: RecordBatch,
    ) -> Result<(), ParadeError> {
        let mut cache = WRITER_CACHE.lock();

        if !cache.contains_key(WRITER_ID) {
            let writer = Self::create(schema_name, table_path, arrow_schema).await?;
            let table = DatafusionContext::with_tables(schema_name, |mut tables| {
                task::block_on(tables.get_owned(table_path))
            })?;

            cache.insert(
                WRITER_ID.to_string(),
                WriterCache::new(writer, table, schema_name.to_string())?,
            );
        }

        match cache.get_mut(WRITER_ID) {
            Some(writer_cache) => {
                writer_cache.write(&batch).await?;
                Ok(())
            }
            None => return Err(NotFound::Writer().into()),
        }
    }

    pub async fn commit() -> Result<(String, DeltaTable), ParadeError> {
        let mut cache = WRITER_CACHE.lock();

        match cache.entry(WRITER_ID.to_string()) {
            Occupied(entry) => {
                let writer_cache = entry.remove();
                writer_cache.commit().await
            }
            Vacant(_) => return Err(NotFound::Writer().into()),
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

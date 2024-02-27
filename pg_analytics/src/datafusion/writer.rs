use async_std::task;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::kernel::Action;
use deltalake::operations::transaction::commit;
use deltalake::operations::writer::{DeltaWriter, WriterConfig};
use deltalake::protocol::{DeltaOperation, SaveMode};
use deltalake::writer::{DeltaWriter as DeltaWriterTrait, RecordBatchWriter, WriteMode};
use deltalake::DeltaTable;
use pgrx::*;
use std::collections::{
    hash_map::Entry::{self, Occupied, Vacant},
    HashMap,
};
use std::path::PathBuf;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::table::DatafusionTable;
use crate::errors::{NotFound, ParadeError};
use crate::guc::PARADE_GUC;

const BYTES_IN_MB: i64 = 1_048_576;

pub struct Writers {
    delta_writers: HashMap<PathBuf, DeltaWriter>,
}

impl Writers {
    pub fn new() -> Result<Self, ParadeError> {
        Ok(Self {
            delta_writers: HashMap::new(),
        })
    }

    pub async fn write(
        &mut self,
        pg_relation: &PgRelation,
        batch: RecordBatch,
    ) -> Result<(), ParadeError> {
        let table_path = pg_relation.table_path()?;

        let writer = match Self::get_entry(self, &table_path)? {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(Self::create(pg_relation).await?),
        };

        writer.write(&batch).await?;

        Ok(())
    }

    pub async fn flush_and_commit(
        &mut self,
        table_name: &str,
        schema_name: &str,
        table_path: &PathBuf,
    ) -> Result<DeltaTable, ParadeError> {
        let writer = match Self::get_entry(self, table_path)? {
            Occupied(entry) => entry.remove(),
            Vacant(_) => return Err(NotFound::Writer(table_name.to_string()).into()),
        };

        let actions = writer.close().await?;
        let delta_table = DatafusionContext::with_tables(schema_name, |mut tables| {
            task::block_on(tables.get_owned(table_path))
        })?;

        commit(
            delta_table.log_store().as_ref(),
            &actions.iter().map(|a| Action::Add(a.clone())).collect(),
            DeltaOperation::Write {
                mode: SaveMode::Append,
                partition_by: None,
                predicate: None,
            },
            delta_table.state.as_ref(),
            None,
        )
        .await?;

        Ok(delta_table)
    }

    pub async fn merge_schema(
        &mut self,
        pg_relation: &PgRelation,
        batch: RecordBatch,
    ) -> Result<DeltaTable, ParadeError> {
        let schema_name = pg_relation.namespace();
        let table_path = pg_relation.table_path()?;

        let mut delta_table = DatafusionContext::with_tables(schema_name, |mut tables| {
            task::block_on(tables.get_owned(&table_path))
        })?;

        // Write the RecordBatch to the DeltaTable
        let mut writer = RecordBatchWriter::for_table(&delta_table)?;
        writer
            .write_with_mode(batch, WriteMode::MergeSchema)
            .await?;
        writer.flush_and_commit(&mut delta_table).await?;

        // Remove the old writer
        self.delta_writers.remove(&table_path);

        Ok(delta_table)
    }

    async fn create(pg_relation: &PgRelation) -> Result<DeltaWriter, ParadeError> {
        let target_file_size = PARADE_GUC.optimize_file_size_mb.get() as i64 * BYTES_IN_MB;

        let schema_name = pg_relation.namespace();
        let table_path = pg_relation.table_path()?;
        let arrow_schema = pg_relation.arrow_schema()?;
        let writer_config = WriterConfig::new(
            arrow_schema,
            vec![],
            None,
            Some(target_file_size as usize),
            None,
        );

        let delta_table = DatafusionContext::with_tables(schema_name, |mut tables| {
            task::block_on(tables.get_owned(&table_path))
        })?;

        Ok(DeltaWriter::new(delta_table.object_store(), writer_config))
    }

    fn get_entry(
        &mut self,
        table_path: &PathBuf,
    ) -> Result<Entry<PathBuf, DeltaWriter>, ParadeError> {
        Ok(self.delta_writers.entry(table_path.to_path_buf()))
    }
}

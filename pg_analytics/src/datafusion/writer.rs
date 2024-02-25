use async_std::task;
use deltalake::datafusion::arrow::datatypes::Schema as ArrowSchema;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::kernel::Action;
use deltalake::operations::transaction::commit;
use deltalake::operations::writer::{DeltaWriter, WriterConfig};
use deltalake::protocol::{DeltaOperation, SaveMode};
use deltalake::writer::{DeltaWriter as DeltaWriterTrait, RecordBatchWriter, WriteMode};
use pgrx::*;
use std::collections::{
    hash_map::Entry::{self, Occupied, Vacant},
    HashMap,
};
use std::path::PathBuf;
use std::sync::Arc;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::directory::ParadeDirectory;
use crate::datafusion::table::DeltaTableProvider;
use crate::errors::{NotFound, ParadeError};
use crate::guc::PARADE_GUC;

const BYTES_IN_MB: i64 = 1_048_576;

pub struct Writer {
    schema_name: String,
    delta_writers: HashMap<PathBuf, DeltaWriter>,
}

impl Writer {
    pub fn new(schema_name: &str) -> Self {
        Self {
            schema_name: schema_name.to_string(),
            delta_writers: HashMap::new(),
        }
    }

    pub async fn write(
        &mut self,
        pg_relation: &PgRelation,
        batch: RecordBatch,
    ) -> Result<(), ParadeError> {
        let table_name = pg_relation.name();
        let table_oid = pg_relation.oid();
        let schema_oid = pg_relation.namespace_oid();
        let table_path =
            ParadeDirectory::table_path(DatafusionContext::catalog_oid()?, schema_oid, table_oid)?;

        info!("Writing to table: {:?}", table_path);

        let writer = match Self::get_writer(self, table_path)? {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(Self::create_writer(pg_relation)?),
        };

        info!("writer created");

        writer.write(&batch).await?;

        info!("written");

        Ok(())
    }

    pub async fn flush_and_commit(
        &mut self,
        table_name: &str,
        table_path: PathBuf,
    ) -> Result<(), ParadeError> {
        let writer = match Self::get_writer(self, table_path.clone())? {
            Occupied(entry) => entry.remove(),
            Vacant(_) => return Err(NotFound::Writer(table_name.to_string()).into()),
        };

        info!("Flushing and committing table: {:?}", table_path);

        let actions = writer.close().await?;
        let mut delta_table =
            DatafusionContext::with_schema_provider(&self.schema_name, |provider| {
                task::block_on(provider.get_delta_table(table_name))
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

        delta_table.update().await?;

        DatafusionContext::with_schema_provider(&self.schema_name, |provider| {
            Ok(provider.register_table(table_name.to_string(), Arc::new(delta_table)))
        })?;

        Ok(())
    }

    pub async fn merge_schema(
        &mut self,
        table_name: &str,
        table_path: PathBuf,
        batch: RecordBatch,
    ) -> Result<(), ParadeError> {
        let mut delta_table =
            DatafusionContext::with_schema_provider(&self.schema_name, |provider| {
                task::block_on(provider.get_delta_table(table_name))
            })?;

        // Write the RecordBatch to the DeltaTable
        let mut writer = RecordBatchWriter::for_table(&delta_table)?;
        writer
            .write_with_mode(batch, WriteMode::MergeSchema)
            .await?;
        writer.flush_and_commit(&mut delta_table).await?;

        // Update the DeltaTable
        delta_table.update().await?;

        // Commit the table
        DatafusionContext::with_schema_provider(&self.schema_name, |provider| {
            Ok(provider.register_table(table_name.to_string(), Arc::new(delta_table)))
        })?;

        // Remove the old writer
        self.delta_writers.remove(&table_path);

        Ok(())
    }

    fn get_writer(
        &mut self,
        table_path: PathBuf,
    ) -> Result<Entry<PathBuf, DeltaWriter>, ParadeError> {
        Ok(self.delta_writers.entry(table_path))
    }

    fn create_writer(pg_relation: &PgRelation) -> Result<DeltaWriter, ParadeError> {
        let table_name = pg_relation.name();
        let schema_name = pg_relation.namespace();
        let target_file_size = PARADE_GUC.optimize_file_size_mb.get() as i64 * BYTES_IN_MB;
        let fields = pg_relation.fields()?;
        let arrow_schema = Arc::new(ArrowSchema::new(fields));
        let writer_config = WriterConfig::new(
            arrow_schema,
            vec![],
            None,
            Some(target_file_size as usize),
            None,
        );

        info!("got writer config");

        let delta_table = DatafusionContext::with_schema_provider(schema_name, |provider| {
            task::block_on(provider.get_delta_table(table_name))
        })?;

        info!("got delta table");

        Ok(DeltaWriter::new(delta_table.object_store(), writer_config))
    }
}

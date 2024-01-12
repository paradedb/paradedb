use async_std::task;
use async_trait::async_trait;
use deltalake::datafusion::arrow::datatypes::{DataType, Field, Schema as ArrowSchema};
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::datasource::TableProvider;
use deltalake::datafusion::error::Result;
use deltalake::operations::optimize::OptimizeBuilder;
use deltalake::operations::update::UpdateBuilder;
use deltalake::operations::vacuum::VacuumBuilder;
use deltalake::parquet::file::properties::{WriterProperties, WriterVersion};
use deltalake::schema::Schema as DeltaSchema;
use deltalake::writer::{DeltaWriter, RecordBatchWriter};
use deltalake::DeltaOps;
use deltalake::DeltaTable;

use parking_lot::{Mutex, RwLock};
use pgrx::*;
use std::{
    any::Any, collections::HashMap, ffi::CStr, fs::remove_dir_all, path::PathBuf, sync::Arc,
};

use crate::datafusion::directory::ParquetDirectory;
use crate::datafusion::error::{datafusion_err_to_string, delta_err_to_string};
use crate::datafusion::substrait::SubstraitTranslator;
use crate::guc::PARADE_GUC;

pub static PARADE_SCHEMA: &str = "public";
const BYTES_IN_MB: i64 = 1_048_576;

pub struct ParadeSchemaProvider {
    tables: RwLock<HashMap<String, Arc<dyn TableProvider>>>,
    writers: Mutex<HashMap<String, RecordBatchWriter>>,
    dir: PathBuf,
}

impl ParadeSchemaProvider {
    // Creates an empty ParadeSchemaProvider
    pub async fn try_new(dir: PathBuf) -> Result<Self> {
        Ok(Self {
            tables: RwLock::new(HashMap::new()),
            writers: Mutex::new(HashMap::new()),
            dir,
        })
    }

    // Loads tables and writers into ParadeSchemaProvider
    pub async fn init(&self) -> Result<()> {
        let mut tables = HashMap::new();
        let mut writers = HashMap::new();

        let listdir = std::fs::read_dir(self.dir.clone())?;

        for res in listdir {
            let table_oid = res?.file_name().to_str().unwrap().to_string();

            if let Ok(oid) = table_oid.parse() {
                let pg_oid = unsafe { pg_sys::Oid::from_u32_unchecked(oid) };
                let relation = unsafe { pg_sys::RelationIdGetRelation(pg_oid) };

                if relation.is_null() {
                    continue;
                }

                let table_name = unsafe {
                    CStr::from_ptr((*((*relation).rd_rel)).relname.data.as_ptr())
                        .to_str()
                        .unwrap()
                };

                let delta_table = match Self::table_exist(self, table_name) {
                    true => Self::get_delta_table(self, table_name).await.unwrap(),
                    false => {
                        deltalake::open_table(&ParquetDirectory::table_path(&table_oid).unwrap())
                            .await?
                    }
                };

                let writer = RecordBatchWriter::for_table(&delta_table)?;

                writers.insert(table_name.to_string(), writer);
                tables.insert(
                    table_name.to_string(),
                    Arc::new(delta_table) as Arc<dyn TableProvider>,
                );

                unsafe { pg_sys::RelationClose(relation) };
            }
        }

        let mut table_lock = self.tables.write();
        *table_lock = tables;

        let mut writer_lock = self.writers.lock();
        *writer_lock = writers;

        Ok(())
    }

    // Creates and registers an empty DeltaTable
    pub async fn create_table(&self, pg_relation: &PgRelation) -> Result<(), String> {
        // Create a RecordBatch with schema from pg_relation
        let table_oid = pg_relation.oid().as_u32().to_string();
        let table_name = pg_relation.name();
        let fields = Self::fields(pg_relation)?;
        let arrow_schema = ArrowSchema::new(fields);
        let delta_schema =
            DeltaSchema::try_from(&arrow_schema).expect("Could not convert Arrow to Delta schema");
        let batch = RecordBatch::new_empty(Arc::new(arrow_schema));

        // Create a DeltaTable
        let mut table = DeltaOps::try_from_uri(&ParquetDirectory::table_path(&table_oid)?)
            .await
            .map_err(delta_err_to_string())?
            .create()
            .with_columns(delta_schema.get_fields().to_vec())
            .await
            .map_err(delta_err_to_string())?;

        // Create a table writer
        let writer_properties = WriterProperties::builder()
            .set_writer_version(WriterVersion::PARQUET_2_0)
            .build();

        let mut writer = RecordBatchWriter::for_table(&table)
            .map_err(delta_err_to_string())?
            .with_writer_properties(writer_properties);

        // Write the RecordBatch to the DeltaTable
        writer.write(batch).await.map_err(delta_err_to_string())?;
        writer
            .flush_and_commit(&mut table)
            .await
            .map_err(delta_err_to_string())?;

        // Register the table writer
        Self::register_writer(self, table_name, writer).map_err(datafusion_err_to_string())?;

        // Register the DeltaTable
        Self::register_table(
            self,
            table_name.to_string(),
            Arc::new(table) as Arc<dyn TableProvider>,
        )
        .map_err(datafusion_err_to_string())?;

        Ok(())
    }

    // Calls DeltaOps vacuum on a DeltaTable
    pub async fn vacuum(&self, table_name: &str, optimize: bool) -> Result<(), String> {
        // Open the DeltaTable
        let mut old_table = Self::get_delta_table(self, table_name).await?;

        // Optimize the table
        if optimize {
            let optimized_table =
                OptimizeBuilder::new(old_table.object_store(), old_table.state.clone())
                    .with_target_size(PARADE_GUC.optimize_file_size_mb.get() as i64 * BYTES_IN_MB)
                    .await
                    .map_err(delta_err_to_string())?
                    .0;

            old_table = optimized_table;
        }

        // Vacuum the table
        let vacuumed_table = VacuumBuilder::new(old_table.object_store(), old_table.state)
            .with_retention_period(chrono::Duration::days(
                PARADE_GUC.vacuum_retention_days.get() as i64,
            ))
            .with_enforce_retention_duration(PARADE_GUC.vacuum_enforce_retention.get())
            .await
            .map_err(delta_err_to_string())?
            .0;

        // Commit the vacuumed table
        Self::register_table(
            self,
            table_name.to_string(),
            Arc::new(vacuumed_table) as Arc<dyn TableProvider>,
        )
        .map_err(datafusion_err_to_string())?;

        Ok(())
    }

    // Vacuum all tables in the schema directory and delete directories for dropped tables
    pub async fn vacuum_all(&self, optimize: bool) -> Result<(), String> {
        let listdir = std::fs::read_dir(self.dir.clone()).unwrap();

        // Iterate over all tables in the directory
        for res in listdir {
            let table_oid = res.unwrap().file_name().to_str().unwrap().to_string();

            if let Ok(oid) = table_oid.parse() {
                let pg_oid = unsafe { pg_sys::Oid::from_u32_unchecked(oid) };
                let relation = unsafe { pg_sys::RelationIdGetRelation(pg_oid) };

                // If the relation is null, delete the directory
                if relation.is_null() {
                    let path = self.dir.join(&table_oid);
                    remove_dir_all(path.clone()).unwrap();
                // Otherwise, vacuum the table
                } else {
                    let table_name = unsafe {
                        CStr::from_ptr((*((*relation).rd_rel)).relname.data.as_ptr())
                            .to_str()
                            .unwrap()
                    };

                    Self::vacuum(self, table_name, optimize).await?;

                    unsafe { pg_sys::RelationClose(relation) }
                }
            }
        }

        Ok(())
    }

    // Write a RecordBatch to a table's writer
    pub fn write(&self, table_name: &str, batch: RecordBatch) -> Result<(), String> {
        let FILE_SIZE_MB = (PARADE_GUC.optimize_file_size_mb.get() as i64) * BYTES_IN_MB;

        // Write batch to buffer
        let mut writer_lock = self.writers.lock();
        let writer = writer_lock
            .get_mut(table_name)
            .expect("Failed to get writer");

        task::block_on(writer.write(batch)).map_err(delta_err_to_string())?;

        // If the buffer is too large, flush it to disk
        if writer.buffer_len() > FILE_SIZE_MB as usize {
            task::block_on(writer.flush()).map_err(delta_err_to_string())?;
        }

        Ok(())
    }

    // Flush and commit a table's writer buffer to disk
    pub async fn flush_and_commit(&self, table_name: &str) -> Result<(), String> {
        // Get the DeltaTable
        let mut delta_table = Self::get_delta_table(self, table_name).await?;

        // Get the writer
        let mut writer_lock = self.writers.lock();
        let writer = writer_lock
            .get_mut(table_name)
            .expect("Failed to get writer");

        // Flush and commit buffer to delta logs
        task::block_on(writer.flush_and_commit(&mut delta_table)).map_err(delta_err_to_string())?;

        // Commiting creates a new version of the DeltaTable
        // Update the provider with the new version
        Self::register_table(
            self,
            table_name.to_string(),
            Arc::new(delta_table) as Arc<dyn TableProvider>,
        )
        .map_err(datafusion_err_to_string())?;

        Ok(())
    }

    // SchemaProvider stores immutable TableProviders, whereas many DeltaOps methods
    // require a mutable DeltaTable. This function gets a mutable DeltaTable from
    // a TableProvider using the DeltaOps UpdateBuilder.
    async fn get_delta_table(&self, table_name: &str) -> Result<DeltaTable, String> {
        let provider = Self::table(self, table_name).await.unwrap();
        let old_table = provider.as_any().downcast_ref::<DeltaTable>().unwrap();

        Ok(
            UpdateBuilder::new(old_table.object_store(), old_table.state.clone())
                .await
                .map_err(delta_err_to_string())?
                .0,
        )
    }

    // Helper function to convert pg_relation attributes to a list of Datafusion Fields
    fn fields(pg_relation: &PgRelation) -> Result<Vec<Field>, String> {
        let tupdesc = pg_relation.tuple_desc();
        let mut fields = Vec::with_capacity(tupdesc.len());

        for attribute in tupdesc.iter() {
            if attribute.is_dropped() {
                continue;
            }

            let attname = attribute.name();
            let attribute_type_oid = attribute.type_oid();
            // Setting it to true because of a likely bug in Datafusion where inserts
            // fail on nullability = false fields
            let nullability = true;

            let array_type = unsafe { pg_sys::get_element_type(attribute_type_oid.value()) };
            let (base_oid, is_array) = if array_type != pg_sys::InvalidOid {
                (PgOid::from(array_type), true)
            } else {
                (attribute_type_oid, false)
            };

            if is_array {
                return Err("Array data types are not supported".to_string());
            }

            let field = Field::new(
                attname,
                DataType::from_substrait(base_oid.to_substrait()?)?,
                nullability,
            );

            fields.push(field);
        }

        Ok(fields)
    }

    // Helper function to register a table writer
    fn register_writer(&self, name: &str, writer: RecordBatchWriter) -> Result<()> {
        let mut writers = self.writers.lock();
        writers.insert(name.to_string(), writer);

        Ok(())
    }
}

#[async_trait]
impl SchemaProvider for ParadeSchemaProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn table_names(&self) -> Vec<String> {
        let tables = self.tables.read();
        tables.keys().cloned().collect::<Vec<_>>()
    }

    async fn table(&self, name: &str) -> Option<Arc<dyn TableProvider>> {
        let tables = self.tables.read();
        tables.get(name).cloned()
    }

    fn table_exist(&self, name: &str) -> bool {
        let tables = self.tables.read();
        tables.contains_key(name)
    }

    fn register_table(
        &self,
        name: String,
        table: Arc<dyn TableProvider>,
    ) -> Result<Option<Arc<dyn TableProvider>>> {
        let mut tables = self.tables.write();
        tables.insert(name, table.clone());
        Ok(Some(table))
    }

    fn deregister_table(&self, name: &str) -> Result<Option<Arc<dyn TableProvider>>> {
        let mut tables = self.tables.write();
        Ok(tables.remove(name))
    }
}

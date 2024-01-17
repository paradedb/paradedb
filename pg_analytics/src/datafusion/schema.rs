use async_std::task;
use async_trait::async_trait;
use deltalake::datafusion::arrow::datatypes::{DataType, Field, Schema as ArrowSchema};
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::datasource::TableProvider;
use deltalake::datafusion::error::Result;
use deltalake::operations::delete::DeleteBuilder;
use deltalake::operations::delete::DeleteBuilder;
use deltalake::operations::optimize::OptimizeBuilder;
use deltalake::operations::transaction::commit;
use deltalake::operations::update::UpdateBuilder;
use deltalake::operations::vacuum::VacuumBuilder;
use deltalake::operations::writer::{DeltaWriter, WriterConfig};
use deltalake::protocol::{Action, DeltaOperation, SaveMode};
use deltalake::schema::Schema as DeltaSchema;
use deltalake::storage::DeltaObjectStore;
use deltalake::{DeltaOps, DeltaTable};
use parking_lot::{Mutex, RwLock};
use pgrx::*;
use std::{
    any::Any, collections::HashMap, ffi::CStr, ffi::CString, fs::remove_dir_all, path::PathBuf,
    sync::Arc,
};

use crate::datafusion::datatype::{DatafusionTypeTranslator, PostgresTypeTranslator};
use crate::datafusion::directory::ParadeDirectory;
use crate::errors::ParadeError;
use crate::guc::PARADE_GUC;

const BYTES_IN_MB: i64 = 1_048_576;

pub struct ParadeSchemaProvider {
    schema_name: String,
    tables: RwLock<HashMap<String, Arc<dyn TableProvider>>>,
    writers: Mutex<HashMap<String, DeltaWriter>>,
    dir: PathBuf,
}

impl ParadeSchemaProvider {
    // Creates an empty ParadeSchemaProvider
    pub async fn try_new(schema_name: &str, dir: PathBuf) -> Result<Self, ParadeError> {
        Ok(Self {
            schema_name: schema_name.to_string(),
            tables: RwLock::new(HashMap::new()),
            writers: Mutex::new(HashMap::new()),
            dir,
        })
    }

    // Loads tables and writers into ParadeSchemaProvider
    pub async fn init(&self) -> Result<(), ParadeError> {
        let mut tables = HashMap::new();
        let mut writers = HashMap::new();

        let listdir = std::fs::read_dir(self.dir.clone())?;

        for res in listdir {
            // Get the table OID from the file name
            let table_oid = res?
                .file_name()
                .to_str()
                .ok_or_else(|| ParadeError::NotFound)?
                .to_string();

            if let Ok(oid) = table_oid.parse() {
                let pg_oid = unsafe { pg_sys::Oid::from_u32_unchecked(oid) };
                let relation = unsafe { pg_sys::RelationIdGetRelation(pg_oid) };

                if relation.is_null() {
                    continue;
                }

                // Get the table name from the OID
                let table_name = unsafe {
                    CStr::from_ptr((*((*relation).rd_rel)).relname.data.as_ptr()).to_str()?
                };

                let schema_oid = unsafe {
                    pg_sys::get_namespace_oid(
                        CString::new(self.schema_name.clone())?.as_ptr(),
                        true,
                    )
                };

                // Create a DeltaTable
                // This is the only place where deltalake::open_table should be called
                // Calling deltalake::open_table multiple times on the same directory results in an error
                let delta_table = match Self::table_exist(self, table_name) {
                    true => Self::get_delta_table(self, table_name).await?,
                    false => {
                        deltalake::open_table(
                            ParadeDirectory::table_path(schema_oid, pg_oid)?
                                .to_str()
                                .ok_or_else(|| ParadeError::NotFound)?,
                        )
                        .await?
                    }
                };

                // Create a writer
                let pg_relation = unsafe { PgRelation::from_pg(relation) };
                let fields = Self::fields(&pg_relation)?;
                let writer = Self::create_writer(
                    self,
                    delta_table.object_store(),
                    Arc::new(ArrowSchema::new(fields)),
                )?;

                writers.insert(table_name.to_string(), writer);
                tables.insert(
                    table_name.to_string(),
                    Arc::new(delta_table) as Arc<dyn TableProvider>,
                );

                unsafe { pg_sys::RelationClose(relation) };
            }
        }

        // Register all tables and writers
        let mut table_lock = self.tables.write();
        *table_lock = tables;

        let mut writer_lock = self.writers.lock();
        *writer_lock = writers;

        Ok(())
    }

    // Creates and registers an empty DeltaTable
    pub async fn create_table(&self, pg_relation: &PgRelation) -> Result<(), ParadeError> {
        // Create a RecordBatch with schema from pg_relation
        let table_oid = pg_relation.oid();
        let schema_oid = pg_relation.namespace_oid();
        let table_name = pg_relation.name();
        let fields = Self::fields(pg_relation)?;
        let arrow_schema = ArrowSchema::new(fields);
        let delta_schema = DeltaSchema::try_from(&arrow_schema)?;
        let batch = RecordBatch::new_empty(Arc::new(arrow_schema.clone()));

        // Create a DeltaTable
        ParadeDirectory::create_schema_path(schema_oid)?;

        let mut delta_table = DeltaOps::try_from_uri(
            &ParadeDirectory::table_path(schema_oid, table_oid)?
                .to_str()
                .ok_or_else(|| ParadeError::NotFound)?,
        )
        .await?
        .create()
        .with_columns(delta_schema.get_fields().to_vec())
        .await?;

        let mut writer = Self::create_writer(
            self,
            delta_table.object_store(),
            Arc::new(arrow_schema.clone()),
        )?;

        // Write the RecordBatch to the DeltaTable
        writer.write(&batch).await?;
        writer.close().await?;

        // Update the DeltaTable
        delta_table.update().await?;

        // Register the table writer
        Self::register_writer(
            self,
            table_name,
            Self::create_writer(self, delta_table.object_store(), Arc::new(arrow_schema))?,
        )?;

        // Register the DeltaTable
        Self::register_table(
            self,
            table_name.to_string(),
            Arc::new(delta_table) as Arc<dyn TableProvider>,
        )?;

        Ok(())
    }

    // Calls DeltaOps vacuum on a DeltaTable
    pub async fn vacuum(&self, table_name: &str, optimize: bool) -> Result<(), ParadeError> {
        // Open the DeltaTable
        let mut old_table = Self::get_delta_table(self, table_name).await?;

        // Optimize the table
        if optimize {
            let optimized_table =
                OptimizeBuilder::new(old_table.object_store(), old_table.state.clone())
                    .with_target_size(PARADE_GUC.optimize_file_size_mb.get() as i64 * BYTES_IN_MB)
                    .await?
                    .0;

            old_table = optimized_table;
        }

        // Vacuum the table
        let vacuumed_table = VacuumBuilder::new(old_table.object_store(), old_table.state)
            .with_retention_period(chrono::Duration::days(
                PARADE_GUC.vacuum_retention_days.get() as i64,
            ))
            .with_enforce_retention_duration(PARADE_GUC.vacuum_enforce_retention.get())
            .await?
            .0;

        // Commit the vacuumed table
        Self::register_table(
            self,
            table_name.to_string(),
            Arc::new(vacuumed_table) as Arc<dyn TableProvider>,
        )?;

        Ok(())
    }

    // Vacuum all tables in the schema directory and delete directories for dropped tables
    pub async fn vacuum_all(&self, optimize: bool) -> Result<(), ParadeError> {
        let listdir = std::fs::read_dir(self.dir.clone())?;

        // Iterate over all tables in the directory
        for res in listdir {
            let table_oid = res?
                .file_name()
                .to_str()
                .ok_or_else(|| ParadeError::NotFound)?
                .to_string();

            if let Ok(oid) = table_oid.parse() {
                let pg_oid = unsafe { pg_sys::Oid::from_u32_unchecked(oid) };
                let relation = unsafe { pg_sys::RelationIdGetRelation(pg_oid) };

                // If the relation is null, delete the directory
                if relation.is_null() {
                    let path = self.dir.join(&table_oid);
                    remove_dir_all(path.clone())?;
                // Otherwise, vacuum the table
                } else {
                    let table_name = unsafe {
                        CStr::from_ptr((*((*relation).rd_rel)).relname.data.as_ptr()).to_str()?
                    };

                    Self::vacuum(self, table_name, optimize).await?;

                    unsafe { pg_sys::RelationClose(relation) }
                }
            }
        }

        Ok(())
    }

    // Write a RecordBatch to a table's writer
    pub async fn write(&self, table_name: &str, batch: RecordBatch) -> Result<(), ParadeError> {
        // Write batch to buffer
        let mut writer_lock = self.writers.lock();
        let writer = writer_lock
            .get_mut(table_name)
            .ok_or_else(|| ParadeError::NotFound)?;

        task::block_on(writer.write(&batch))?;
        Ok(())
    }

    // Flush and commit a table's writer buffer to disk
    pub async fn flush_and_commit(
        &self,
        table_name: &str,
        arrow_schema: Arc<ArrowSchema>,
    ) -> Result<(), ParadeError> {
        // Get the DeltaTable
        let mut delta_table = Self::get_delta_table(self, table_name).await?;

        // Get the writer
        let mut writer_lock = self.writers.lock();
        let writer = writer_lock
            .remove(table_name)
            .ok_or_else(|| ParadeError::NotFound)?;

        // Generate commit actions by closing the writer and commit to delta logs
        let actions = task::block_on(writer.close())?;
        drop(writer_lock);

        task::block_on(commit(
            delta_table.object_store().as_ref(),
            &actions.iter().map(|a| Action::add(a.clone())).collect(),
            DeltaOperation::Write {
                mode: SaveMode::Append,
                partition_by: None,
                predicate: None,
            },
            &delta_table.state,
            None,
        ))?;

        // Update the DeltaTable
        task::block_on(delta_table.update())?;

        // Create and register a new writer
        Self::register_writer(
            self,
            table_name,
            Self::create_writer(self, delta_table.object_store(), arrow_schema)?,
        )?;

        // Commiting creates a new version of the DeltaTable
        // Update the provider with the new version
        Self::register_table(
            self,
            table_name.to_string(),
            Arc::new(delta_table) as Arc<dyn TableProvider>,
        )?;

        Ok(())
    }

    // modeled after vacuum
    pub async fn delete(&self, table_name: &str, predicate: &str) -> Result<(), ParadeError> {
        // Open the DeltaTable
        let old_table = Self::get_delta_table(self, table_name).await?;

        // Delete (deletebuilder can take a string predicate as long as it can be turned into a datafusion expr)
        let deleted_table = DeleteBuilder::new(old_table.object_store(), old_table.state)
            .with_predicate(predicate)
            .await?
            .0;

        // Commit the vacuumed table
        Self::register_table(
            self,
            table_name.to_string(),
            Arc::new(deleted_table) as Arc<dyn TableProvider>,
        )?;

        Ok(())
    }

    pub async fn rename(&self, old_name: &str, new_name: &str) -> Result<(), ParadeError> {
        let mut tables = self.tables.write();
        let mut writers = self.writers.lock();

        if let Some(table) = tables.remove(old_name) {
            tables.insert(new_name.to_string(), table);
        }

        if let Some(writer) = writers.remove(old_name) {
            writers.insert(new_name.to_string(), writer);
        }

        Ok(())
    }

    pub async fn truncate(&self, table_name: &str) -> Result<(), ParadeError> {
        // Open the DeltaTable
        let delta_table = Self::get_delta_table(self, table_name).await?;

        // Truncate the table
        let truncated_table = DeleteBuilder::new(delta_table.object_store(), delta_table.state)
            .await?
            .0;

        // Commit the vacuumed table
        Self::register_table(
            self,
            table_name.to_string(),
            Arc::new(truncated_table) as Arc<dyn TableProvider>,
        )?;

        Ok(())
    }

    // SchemaProvider stores immutable TableProviders, whereas many DeltaOps methods
    // require a mutable DeltaTable. This function gets a mutable DeltaTable from
    // a TableProvider using the DeltaOps UpdateBuilder.
    async fn get_delta_table(&self, table_name: &str) -> Result<DeltaTable, ParadeError> {
        let provider = Self::table(self, table_name)
            .await
            .ok_or_else(|| ParadeError::NotFound)?;
        let old_table = provider
            .as_any()
            .downcast_ref::<DeltaTable>()
            .ok_or_else(|| ParadeError::NotFound)?;

        Ok(
            UpdateBuilder::new(old_table.object_store(), old_table.state.clone())
                .await?
                .0,
        )
    }

    // Helper function to convert pg_relation attributes to a list of Datafusion Fields
    fn fields(pg_relation: &PgRelation) -> Result<Vec<Field>, ParadeError> {
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
                return Err(ParadeError::Generic(
                    "Array types not yet supported".to_string(),
                ));
            }

            let field = Field::new(
                attname,
                DataType::from_sql_data_type(base_oid.to_sql_data_type(attribute.type_mod())?)?,
                nullability,
            );

            fields.push(field);
        }

        Ok(fields)
    }

    // Helper function to register a table writer
    fn register_writer(&self, name: &str, writer: DeltaWriter) -> Result<(), ParadeError> {
        let mut writers = self.writers.lock();
        writers.insert(name.to_string(), writer);

        Ok(())
    }

    // Helper function to create a table writer
    fn create_writer(
        &self,
        object_store: Arc<DeltaObjectStore>,
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
        Ok(DeltaWriter::new(object_store, writer_config))
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

use async_std::task;
use async_trait::async_trait;
use deltalake::datafusion::arrow::datatypes::Schema as ArrowSchema;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::datasource::TableProvider;
use deltalake::datafusion::error::Result;
use deltalake::datafusion::logical_expr::Expr;
use deltalake::kernel::Action;
use deltalake::kernel::Schema as DeltaSchema;
use deltalake::operations::create::CreateBuilder;
use deltalake::operations::delete::DeleteBuilder;
use deltalake::operations::delete::DeleteBuilder;
use deltalake::operations::optimize::OptimizeBuilder;
use deltalake::operations::transaction::commit;
use deltalake::operations::update::UpdateBuilder;
use deltalake::operations::vacuum::VacuumBuilder;
use deltalake::operations::writer::{DeltaWriter, WriterConfig};
use deltalake::protocol::{DeltaOperation, SaveMode};
use deltalake::storage::ObjectStoreRef;
use deltalake::table::state::DeltaTableState;
use deltalake::DeltaTable;
use parking_lot::{Mutex, RwLock};
use pgrx::pg_sys::print;
use pgrx::*;
use std::future::IntoFuture;
use std::{
    any::type_name, any::Any, collections::HashMap, ffi::CStr, ffi::CString, fs::remove_dir_all,
    path::PathBuf, sync::Arc,
};

use crate::datafusion::directory::ParadeDirectory;
use crate::datafusion::table::DeltaTableProvider;
use crate::errors::{NotFound, ParadeError};
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
            let table_oid = res?.file_name().into_string()?;

            if let Ok(oid) = table_oid.parse::<u32>() {
                let pg_oid = pg_sys::Oid::from(oid);
                let relation = unsafe { pg_sys::RelationIdGetRelation(pg_oid) };

                if relation.is_null() {
                    continue;
                }

                // Get the table name from the OID
                let table_name = unsafe {
                    CStr::from_ptr((*((*relation).rd_rel)).relname.data.as_ptr()).to_str()?
                };

                // Create a DeltaTable
                // This is the only place where deltalake::open_table should be called
                // Calling deltalake::open_table multiple times on the same directory results in an error
                let delta_table = Self::get_delta_table(self, table_name).await?;

                // Create a writer
                let pg_relation = unsafe { PgRelation::from_pg(relation) };
                let fields = pg_relation.fields()?;
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
        let fields = pg_relation.fields()?;
        let arrow_schema = ArrowSchema::new(fields);
        let delta_schema = DeltaSchema::try_from(&arrow_schema)?;
        let batch = RecordBatch::new_empty(Arc::new(arrow_schema.clone()));

        // Create a DeltaTable
        ParadeDirectory::create_schema_path(schema_oid)?;

        let mut delta_table = CreateBuilder::new()
            .with_location(ParadeDirectory::table_path(schema_oid, table_oid)?.to_string_lossy())
            .with_columns(delta_schema.fields().to_vec())
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
            let optimized_table = OptimizeBuilder::new(
                old_table.log_store(),
                old_table
                    .state
                    .ok_or(NotFound::Value(type_name::<DeltaTableState>().to_string()))?,
            )
            .with_target_size(PARADE_GUC.optimize_file_size_mb.get() as i64 * BYTES_IN_MB)
            .await?
            .0;

            old_table = optimized_table;
        }

        // Vacuum the table
        let vacuumed_table = VacuumBuilder::new(
            old_table.log_store(),
            old_table
                .state
                .ok_or(NotFound::Value(type_name::<DeltaTableState>().to_string()))?,
        )
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
            let table_oid = res?.file_name().into_string()?;

            if let Ok(oid) = table_oid.parse::<u32>() {
                let pg_oid = pg_sys::Oid::from(oid);
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
            .ok_or(NotFound::Table(table_name.to_string()))?;

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
            .ok_or(NotFound::Table(table_name.to_string()))?;

        // Generate commit actions by closing the writer and commit to delta logs
        let actions = task::block_on(writer.close())?;
        drop(writer_lock);

        task::block_on(commit(
            delta_table.log_store().as_ref(),
            &actions.iter().map(|a| Action::Add(a.clone())).collect(),
            DeltaOperation::Write {
                mode: SaveMode::Append,
                partition_by: None,
                predicate: None,
            },
            delta_table.state.as_ref(),
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
    pub async fn delete(
        &self,
        table_name: &str,
        predicate: Option<Expr>,
    ) -> Result<(), ParadeError> {
        // Open the DeltaTable
        let old_table = Self::get_delta_table(self, table_name).await?;

        // Delete (deletebuilder can take a string predicate as long as it can be turned into a datafusion expr)
        let delete_builder = DeleteBuilder::new(old_table.object_store(), old_table.state);
        let deleted_table = match predicate {
            Some(expr) => delete_builder.with_predicate(expr),
            None => delete_builder,
        }
        .await?
        .0;

        // Commit the edited table
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
        let truncated_table = DeleteBuilder::new(
            delta_table.log_store(),
            delta_table
                .state
                .ok_or(NotFound::Value(type_name::<DeltaTableState>().to_string()))?,
        )
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
    pub async fn get_delta_table(&self, name: &str) -> Result<DeltaTable, ParadeError> {
        let mut delta_table = match Self::table_exist(self, name) {
            true => {
                let tables = self.tables.read();
                let provider = tables.get(name).ok_or(NotFound::Table(name.to_string()))?;

                let old_table = provider
                    .as_any()
                    .downcast_ref::<DeltaTable>()
                    .ok_or(NotFound::Value(type_name::<DeltaTable>().to_string()))?;

                task::block_on(
                    UpdateBuilder::new(
                        old_table.log_store(),
                        old_table
                            .state
                            .clone()
                            .ok_or(NotFound::Value(type_name::<DeltaTableState>().to_string()))?,
                    )
                    .into_future(),
                )?
                .0
            }
            false => {
                let schema_oid = unsafe {
                    pg_sys::get_namespace_oid(
                        CString::new(self.schema_name.clone())?.as_ptr(),
                        true,
                    )
                };

                let table_oid =
                    unsafe { pg_sys::get_relname_relid(CString::new(name)?.as_ptr(), schema_oid) };

                deltalake::open_table(
                    ParadeDirectory::table_path(schema_oid, table_oid)?.to_string_lossy(),
                )
                .await?
            }
        };

        task::block_on(delta_table.load())?;

        Ok(delta_table)
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
        object_store: ObjectStoreRef,
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
        let delta_table = task::block_on(Self::get_delta_table(self, name)).ok()?;
        let provider = Arc::new(delta_table) as Arc<dyn TableProvider>;

        Self::register_table(self, name.to_string(), provider.clone()).ok()?
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

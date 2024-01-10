use async_std::task;
use async_trait::async_trait;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::datasource::TableProvider;
use deltalake::datafusion::error::Result;
use deltalake::datafusion::execution::context::SessionState;
use deltalake::writer::{DeltaWriter, RecordBatchWriter};
use deltalake::DeltaTable;
use parking_lot::{Mutex, RwLock};
use pgrx::*;
use std::{
    any::Any, collections::HashMap, ffi::CStr, fs::remove_dir_all, path::PathBuf, sync::Arc,
};

use crate::datafusion::directory::ParquetDirectory;
use crate::datafusion::error::delta_err_to_string;

pub struct ParadeSchemaOpts {
    pub dir: PathBuf,
}

pub struct ParadeSchemaProvider {
    tables: RwLock<HashMap<String, Arc<dyn TableProvider>>>,
    writers: Mutex<HashMap<String, RecordBatchWriter>>,
    opts: ParadeSchemaOpts,
}

impl ParadeSchemaProvider {
    pub async fn try_new(state: &SessionState, opts: ParadeSchemaOpts) -> Result<Self> {
        let (tables, writers) = ParadeSchemaProvider::load(state, &opts).await?;
        Ok(Self {
            tables: RwLock::new(tables),
            writers: Mutex::new(writers),
            opts,
        })
    }

    pub async fn refresh(&self, state: &SessionState) -> Result<()> {
        let (tables, writers) = ParadeSchemaProvider::load(state, &self.opts).await?;
        let mut table_lock = self.tables.write();
        *table_lock = tables;

        let mut writer_lock = self.writers.lock();
        *writer_lock = writers;

        Ok(())
    }

    pub fn vacuum_tables(&self, _state: &SessionState) -> Result<()> {
        let listdir = std::fs::read_dir(self.opts.dir.clone())?;

        for res in listdir {
            let entry = res?;
            let file_name = entry.file_name();
            let table_oid = file_name.to_str().unwrap().to_string();
            if let Ok(oid) = table_oid.parse() {
                let pg_oid = unsafe { pg_sys::Oid::from_u32_unchecked(oid) };
                let relation = unsafe { pg_sys::RelationIdGetRelation(pg_oid) };

                if relation.is_null() {
                    let path = self.opts.dir.join(&table_oid);
                    remove_dir_all(path.clone())?;
                } else {
                    unsafe { pg_sys::RelationClose(relation) }
                }
            }
        }

        Ok(())
    }

    pub fn write(&self, table_name: &str, batch: RecordBatch) -> Result<(), String> {
        let MAX_BUFFER_LEN = 100_000_000;

        // Write batch to buffer
        let mut writer_lock = self.writers.lock();
        let writer = writer_lock
            .get_mut(table_name)
            .expect("Failed to get writer");

        task::block_on(writer.write(batch)).map_err(delta_err_to_string())?;

        // If the buffer is too large, flush it to disk
        if writer.buffer_len() > MAX_BUFFER_LEN {
            task::block_on(writer.flush()).map_err(delta_err_to_string())?;
        }

        Ok(())
    }

    pub fn flush_and_commit(
        &self,
        table_name: &str,
        mut delta_table: DeltaTable,
    ) -> Result<(), String> {
        let mut writer_lock = self.writers.lock();
        let writer = writer_lock
            .get_mut(table_name)
            .expect("Failed to get writer");

        // Flush and commit buffer to delta logs
        task::block_on(writer.flush_and_commit(&mut delta_table)).map_err(delta_err_to_string())?;

        // Commiting creates a new version of the DeltaTable
        // Update the provider with the new version
        let mut table_lock = self.tables.write();
        table_lock.insert(
            table_name.to_string(),
            Arc::new(delta_table) as Arc<dyn TableProvider>,
        );

        Ok(())
    }

    async fn load(
        _state: &SessionState,
        opts: &ParadeSchemaOpts,
    ) -> Result<(
        HashMap<String, Arc<dyn TableProvider>>,
        HashMap<String, RecordBatchWriter>,
    )> {
        let mut tables = HashMap::new();
        let mut writers = HashMap::new();

        let listdir = std::fs::read_dir(opts.dir.clone())?;

        for res in listdir {
            let entry = res?;
            let file_name = entry.file_name();
            let table_oid = file_name.to_str().unwrap().to_string();
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

                let table_path = ParquetDirectory::table_path(&table_oid).unwrap();

                if let Ok(delta_table) = deltalake::open_table(table_path).await {
                    let writer = RecordBatchWriter::for_table(&delta_table)?;

                    writers.insert(table_name.to_string(), writer);
                    tables.insert(
                        table_name.to_string(),
                        Arc::new(delta_table) as Arc<dyn TableProvider>,
                    );
                }

                unsafe { pg_sys::RelationClose(relation) };
            }
        }

        Ok((tables, writers))
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

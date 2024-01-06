use async_trait::async_trait;
use datafusion::catalog::schema::SchemaProvider;
use datafusion::datasource::file_format::FileFormat;
use datafusion::datasource::listing::{
    ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl,
};
use datafusion::datasource::TableProvider;
use datafusion::error::Result;
use datafusion::execution::context::SessionState;
use parking_lot::RwLock;
use pgrx::*;
use std::{any::Any, collections::HashMap, fs::remove_dir_all, path::PathBuf, sync::Arc};

pub struct ParadeSchemaOpts {
    pub dir: PathBuf,
    pub format: Arc<dyn FileFormat>,
}

pub struct ParadeSchemaProvider {
    tables: RwLock<HashMap<String, Arc<dyn TableProvider>>>,
    opts: ParadeSchemaOpts,
}

impl ParadeSchemaProvider {
    pub async fn create(state: &SessionState, opts: ParadeSchemaOpts) -> Result<Self> {
        let tables = ParadeSchemaProvider::load_tables(state, &opts).await?;
        Ok(Self {
            tables: RwLock::new(tables),
            opts,
        })
    }

    pub async fn refresh(&self, state: &SessionState) -> Result<()> {
        let tables = ParadeSchemaProvider::load_tables(state, &self.opts).await?;
        let mut table_lock = self.tables.write();
        *table_lock = tables;
        Ok(())
    }

    pub async fn vacuum_tables(&self, state: &SessionState) -> Result<()> {
        let tables = ParadeSchemaProvider::load_tables(state, &self.opts).await?;

        for (table_oid, _) in tables.iter() {
            if let Ok(oid) = table_oid.parse() {
                let pg_oid = unsafe { pg_sys::Oid::from_u32_unchecked(oid) };
                let relation = unsafe { pg_sys::RelationIdGetRelation(pg_oid) };
                if relation.is_null() {
                    let path = self.opts.dir.join(table_oid);
                    remove_dir_all(path.clone())?;
                }
            }
        }

        Ok(())
    }

    async fn load_tables(
        state: &SessionState,
        opts: &ParadeSchemaOpts,
    ) -> Result<HashMap<String, Arc<dyn TableProvider>>> {
        let mut tables = HashMap::new();
        let listdir = std::fs::read_dir(opts.dir.clone())?;
        for res in listdir {
            let entry = res?;
            let file_name = entry.file_name();
            let table_name = file_name.to_str().unwrap().to_string();
            let table_path = ListingTableUrl::parse(entry.path().to_str().unwrap())?;
            let listing_options = ListingOptions::new(opts.format.clone());
            let config = match ListingTableConfig::new(table_path)
                .with_listing_options(listing_options)
                .infer_schema(state)
                .await
            {
                Ok(conf) => conf,
                Err(_) => {
                    continue;
                }
            };
            let table = ListingTable::try_new(config)?;
            tables.insert(table_name, Arc::new(table) as Arc<dyn TableProvider>);
        }

        Ok(tables)
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

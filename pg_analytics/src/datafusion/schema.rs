use async_std::task;
use async_trait::async_trait;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::datasource::TableProvider;
use deltalake::datafusion::error::Result;
use deltalake::operations::update::UpdateBuilder;
use deltalake::table::state::DeltaTableState;
use parking_lot::Mutex;
use pgrx::*;
use std::future::IntoFuture;
use std::{
    any::{type_name, Any},
    path::PathBuf,
    sync::Arc,
};

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::directory::ParadeDirectory;
use crate::datafusion::table::Tables;
use crate::errors::{NotFound, ParadeError};

pub struct ParadeSchemaProvider {
    schema_name: String,
    tables: Arc<Mutex<Tables>>,
}

impl ParadeSchemaProvider {
    pub async fn try_new(schema_name: &str) -> Result<Self, ParadeError> {
        Ok(Self {
            schema_name: schema_name.to_string(),
            tables: Arc::new(Mutex::new(Tables::new()?)),
        })
    }

    pub fn tables(&self) -> Result<Arc<Mutex<Tables>>, ParadeError> {
        Ok(self.tables.clone())
    }

    fn table_path(&self, table_name: &str) -> Result<Option<PathBuf>, ParadeError> {
        let pg_relation = match unsafe {
            PgRelation::open_with_name(&format!("{}.{}", self.schema_name, table_name))
        } {
            Ok(relation) => relation,
            Err(_) => {
                return Ok(None);
            }
        };

        Ok(Some(ParadeDirectory::table_path(
            DatafusionContext::catalog_oid()?,
            pg_relation.namespace_oid(),
            pg_relation.oid(),
        )?))
    }
}

#[async_trait]
impl SchemaProvider for ParadeSchemaProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn table_names(&self) -> Vec<String> {
        vec![]
    }

    async fn table(&self, table_name: &str) -> Option<Arc<dyn TableProvider>> {
        match Self::table_path(self, table_name) {
            Ok(Some(table_path)) => {
                let delta_table =
                    DatafusionContext::with_tables(&self.schema_name, |mut tables| {
                        let table_ref = task::block_on(tables.get_ref(&table_path))?;
                        Ok(task::block_on(
                            UpdateBuilder::new(
                                table_ref.log_store(),
                                table_ref.state.clone().ok_or(NotFound::Value(
                                    type_name::<DeltaTableState>().to_string(),
                                ))?,
                            )
                            .into_future(),
                        )?
                        .0)
                    })
                    .unwrap();

                Some(Arc::new(delta_table.clone()) as Arc<dyn TableProvider>)
            }
            _ => None,
        }
    }

    fn table_exist(&self, table_name: &str) -> bool {
        matches!(Self::table_path(self, table_name), Ok(Some(_)))
    }
}

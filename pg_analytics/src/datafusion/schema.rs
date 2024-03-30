use async_std::sync::Mutex;
use async_trait::async_trait;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::datasource::TableProvider;
use deltalake::datafusion::error::Result;
use deltalake::operations::update::UpdateBuilder;
use deltalake::table::state::DeltaTableState;
use pgrx::*;
use std::any::{type_name, Any};
use std::ffi::{CStr, CString};
use std::fs::read_dir;
use std::future::IntoFuture;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::errors::{NotFound, ParadeError};

use super::directory::ParadeDirectory;
use super::plan::xmin_filter_plan;
use super::session::Session;
use super::table::{PgTableProvider, Tables};

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
}

#[async_trait]
impl SchemaProvider for ParadeSchemaProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn table_names(&self) -> Vec<String> {
        table_names_impl(&self.schema_name).expect("Failed to get table names")
    }

    async fn table(&self, table_name: &str) -> Option<Arc<dyn TableProvider>> {
        let tables = Self::tables(self).expect("Failed to get tables");
        let table_path = table_path(&self.schema_name, table_name).unwrap_or_else(|_| {
            panic!(
                "Failed to get table path for {}.{}",
                self.schema_name, table_name
            )
        });

        match table_path {
            Some(table_path) => Some(
                table_impl(table_name, &self.schema_name, tables, &table_path)
                    .await
                    .unwrap_or_else(|_| {
                        panic!("Failed to get {}.{}", self.schema_name, table_name)
                    }),
            ),
            None => None,
        }
    }

    fn table_exist(&self, table_name: &str) -> bool {
        matches!(table_path(&self.schema_name, table_name), Ok(Some(_)))
    }
}

#[inline]
fn table_path(schema_name: &str, table_name: &str) -> Result<Option<PathBuf>, ParadeError> {
    let pg_relation =
        match unsafe { PgRelation::open_with_name(&format!("{}.{}", schema_name, table_name)) } {
            Ok(relation) => relation,
            Err(_) => {
                return Ok(None);
            }
        };

    Ok(Some(ParadeDirectory::table_path(
        Session::catalog_oid()?,
        pg_relation.namespace_oid(),
        pg_relation.oid(),
    )?))
}

#[inline]
fn table_names_impl(schema_name: &str) -> Result<Vec<String>, ParadeError> {
    let mut names = vec![];

    let schema_oid =
        unsafe { pg_sys::get_namespace_oid(CString::new(schema_name)?.as_ptr(), true) };
    let schema_path = ParadeDirectory::schema_path(Session::catalog_oid()?, schema_oid)?;

    for file in read_dir(schema_path)? {
        if let Ok(oid) = file?.file_name().into_string()?.parse::<u32>() {
            let pg_oid = pg_sys::Oid::from(oid);
            let relation = unsafe { pg_sys::RelationIdGetRelation(pg_oid) };

            if relation.is_null() {
                continue;
            }

            let table_name =
                unsafe { CStr::from_ptr((*((*relation).rd_rel)).relname.data.as_ptr()).to_str()? };

            names.push(table_name.to_string());
        }
    }

    Ok(names)
}

#[inline]
async fn table_impl(
    table_name: &str,
    schema_name: &str,
    tables: Arc<Mutex<Tables>>,
    table_path: &Path,
) -> Result<Arc<dyn TableProvider>, ParadeError> {
    let mut tables = tables.lock().await;
    let provider = tables.get_ref(table_path).await?;
    let delta_table = provider.table();

    let updated_table = UpdateBuilder::new(
        delta_table.log_store(),
        delta_table
            .state
            .clone()
            .ok_or(NotFound::Value(type_name::<DeltaTableState>().to_string()))?,
    )
    .into_future()
    .await?
    .0;

    Ok(Arc::new(
        PgTableProvider::new(updated_table.clone()).with_logical_plan(
            xmin_filter_plan(table_name, schema_name, updated_table, unsafe {
                pg_sys::GetCurrentTransactionId()
            } as i64)
            .unwrap(),
        ),
    ) as Arc<dyn TableProvider>)
}

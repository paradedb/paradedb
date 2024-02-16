use async_std::task;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::catalog::CatalogProvider;
use deltalake::datafusion::common::arrow::datatypes::DataType;
use deltalake::datafusion::common::config::ConfigOptions;
use deltalake::datafusion::common::DataFusionError;
use deltalake::datafusion::datasource::provider_as_source;
use deltalake::datafusion::execution::runtime_env::{RuntimeConfig, RuntimeEnv};
use deltalake::datafusion::logical_expr::{AggregateUDF, ScalarUDF, TableSource, WindowUDF};
use deltalake::datafusion::prelude::{SessionConfig, SessionContext};
use deltalake::datafusion::sql::planner::ContextProvider;
use deltalake::datafusion::sql::TableReference;
use lazy_static::lazy_static;
use parking_lot::{RwLock, RwLockWriteGuard};
use pgrx::*;
use std::any::type_name;
use std::ffi::{CStr, CString};
use std::sync::Arc;

use crate::datafusion::catalog::{ObjectStoreCatalog, ParadeCatalogList, PostgresCatalog};
use crate::datafusion::directory::ParadeDirectory;
use crate::datafusion::schema::{PermanentSchemaProvider, TempSchemaProvider};
use crate::errors::{NotFound, ParadeError};

lazy_static! {
    pub static ref CONTEXT: RwLock<Option<SessionContext>> = RwLock::new(None);
}

pub struct DatafusionContext;

impl<'a> DatafusionContext {
    pub fn with_session_context<F, R>(f: F) -> Result<R, ParadeError>
    where
        F: FnOnce(&SessionContext) -> Result<R, ParadeError>,
    {
        let context_lock = CONTEXT.read();
        let context = match context_lock.as_ref() {
            Some(context) => context.clone(),
            None => {
                drop(context_lock);
                Self::init(Self::postgres_catalog_oid()?)?
            }
        };

        f(&context)
    }

    pub fn with_permanent_schema_provider<F, R>(schema_name: &str, f: F) -> Result<R, ParadeError>
    where
        F: FnOnce(&PermanentSchemaProvider) -> Result<R, ParadeError>,
    {
        let context_lock = CONTEXT.read();
        let context = match context_lock.as_ref() {
            Some(context) => context.clone(),
            None => {
                drop(context_lock);
                Self::init(Self::postgres_catalog_oid()?)?
            }
        };

        let _catalog_provider =
            context
                .catalog(&Self::postgres_catalog_name()?)
                .ok_or(NotFound::Catalog(
                    Self::postgres_catalog_name()?.to_string(),
                ))?;

        let schema_provider = context
            .catalog(&Self::postgres_catalog_name()?)
            .ok_or(NotFound::Catalog(
                Self::postgres_catalog_name()?.to_string(),
            ))?
            .schema(schema_name)
            .ok_or(NotFound::Schema(schema_name.to_string()))?;

        let parade_provider = schema_provider
            .as_any()
            .downcast_ref::<PermanentSchemaProvider>()
            .ok_or(NotFound::Value(
                type_name::<PermanentSchemaProvider>().to_string(),
            ))?;

        f(parade_provider)
    }

    pub fn with_postgres_catalog<F, R>(f: F) -> Result<R, ParadeError>
    where
        F: FnOnce(&PostgresCatalog) -> Result<R, ParadeError>,
    {
        let context_lock = CONTEXT.read();
        let context = match context_lock.as_ref() {
            Some(context) => context.clone(),
            None => {
                drop(context_lock);
                Self::init(Self::postgres_catalog_oid()?)?
            }
        };

        let catalog_provider =
            context
                .catalog(&Self::postgres_catalog_name()?)
                .ok_or(NotFound::Catalog(
                    Self::postgres_catalog_name()?.to_string(),
                ))?;

        let parade_catalog = catalog_provider
            .as_any()
            .downcast_ref::<PostgresCatalog>()
            .ok_or(NotFound::Value(type_name::<PostgresCatalog>().to_string()))?;

        f(parade_catalog)
    }

    pub fn with_object_store_catalog<F, R>(f: F) -> Result<R, ParadeError>
    where
        F: FnOnce(&ObjectStoreCatalog) -> Result<R, ParadeError>,
    {
        let context_lock = CONTEXT.read();
        let context = match context_lock.as_ref() {
            Some(context) => context.clone(),
            None => {
                drop(context_lock);
                Self::init(Self::postgres_catalog_oid()?)?
            }
        };

        let catalog_provider =
            context
                .catalog(&Self::object_store_catalog_name()?)
                .ok_or(NotFound::Catalog(
                    Self::object_store_catalog_name()?.to_string(),
                ))?;

        let parade_catalog = catalog_provider
            .as_any()
            .downcast_ref::<ObjectStoreCatalog>()
            .ok_or(NotFound::Value(
                type_name::<ObjectStoreCatalog>().to_string(),
            ))?;

        f(parade_catalog)
    }

    pub fn with_temp_schema_provider<F, R>(schema_name: &str, f: F) -> Result<R, ParadeError>
    where
        F: FnOnce(&TempSchemaProvider) -> Result<R, ParadeError>,
    {
        let context_lock = CONTEXT.read();
        let context = match context_lock.as_ref() {
            Some(context) => context.clone(),
            None => {
                drop(context_lock);
                DatafusionContext::init(Self::postgres_catalog_oid()?)?
            }
        };

        let schema_provider = context
            .catalog(&Self::postgres_catalog_name()?)
            .ok_or(NotFound::Catalog(
                Self::postgres_catalog_name()?.to_string(),
            ))?
            .schema(schema_name)
            .ok_or(NotFound::Schema(schema_name.to_string()))?;

        let parade_provider = schema_provider
            .as_any()
            .downcast_ref::<TempSchemaProvider>()
            .ok_or(NotFound::Value(
                type_name::<TempSchemaProvider>().to_string(),
            ))?;

        f(parade_provider)
    }

    pub fn with_write_lock<F, R>(f: F) -> Result<R, ParadeError>
    where
        F: FnOnce(RwLockWriteGuard<'a, Option<SessionContext>>) -> Result<R, ParadeError>,
    {
        let context_lock = CONTEXT.write();
        f(context_lock)
    }

    pub fn init(catalog_oid: pg_sys::Oid) -> Result<SessionContext, ParadeError> {
        let preload_libraries = unsafe {
            CStr::from_ptr(pg_sys::GetConfigOptionByName(
                CString::new("shared_preload_libraries")?.as_ptr(),
                std::ptr::null_mut(),
                true,
            ))
            .to_str()?
        };

        if !preload_libraries.contains("pg_analytics") {
            return Err(ParadeError::SharedPreload);
        }

        let session_config = SessionConfig::from_env()?.with_information_schema(true);

        let rn_config = RuntimeConfig::new();
        let runtime_env = RuntimeEnv::new(rn_config)?;

        Self::with_write_lock(|mut context_lock| {
            let mut context =
                SessionContext::new_with_config_rt(session_config, Arc::new(runtime_env));

            // Create schema directory if it doesn't exist
            ParadeDirectory::create_catalog_path(catalog_oid)?;

            // Register catalog list
            context.register_catalog_list(Arc::new(ParadeCatalogList::try_new()?));

            // Create and register delta catalog
            let postgres_catalog = PostgresCatalog::try_new()?;
            task::block_on(postgres_catalog.init())?;
            context.register_catalog(&Self::postgres_catalog_name()?, Arc::new(postgres_catalog));

            // Create and register object store catalog
            let object_store_catalog = ObjectStoreCatalog::try_new()?;
            context.register_catalog(
                &Self::object_store_catalog_name()?,
                Arc::new(object_store_catalog),
            );

            // Set context
            *context_lock = Some(context.clone());

            Ok(context)
        })
    }

    pub fn postgres_catalog_name() -> Result<String, ParadeError> {
        let database_name = unsafe { pg_sys::get_database_name(Self::postgres_catalog_oid()?) };
        if database_name.is_null() {
            return Err(
                NotFound::Database(Self::postgres_catalog_oid()?.as_u32().to_string()).into(),
            );
        }

        Ok(unsafe { CStr::from_ptr(database_name).to_str()?.to_owned() })
    }

    pub fn object_store_catalog_name() -> Result<String, ParadeError> {
        Ok(String::from("paradedb_object_store_catalog"))
    }

    pub fn postgres_catalog_oid() -> Result<pg_sys::Oid, ParadeError> {
        Ok(unsafe { pg_sys::MyDatabaseId })
    }
}

trait PostgresSchema {
    fn is_temp_schema(&self) -> Result<bool, ParadeError>;
}

impl PostgresSchema for str {
    fn is_temp_schema(&self) -> Result<bool, ParadeError> {
        let c_schema_name = CString::new(self)?;
        let schema_oid = unsafe { pg_sys::get_namespace_oid(c_schema_name.as_ptr(), false) };
        Ok(unsafe { pg_sys::isTempNamespace(schema_oid) })
    }
}

pub struct ParadeContextProvider {
    options: ConfigOptions,
}

impl ParadeContextProvider {
    pub fn new() -> Result<Self, ParadeError> {
        Ok(Self {
            options: ConfigOptions::new(),
        })
    }

    fn get_table_source_impl(
        reference: TableReference,
    ) -> Result<Arc<dyn TableSource>, ParadeError> {
        let table_name = reference.table();

        match reference.schema() {
            Some(name) => match name.is_temp_schema()? {
                true => DatafusionContext::with_temp_schema_provider(name, |provider| {
                    let table = task::block_on(provider.table(table_name))
                        .ok_or(NotFound::Table(table_name.to_string()))?;

                    Ok(provider_as_source(table))
                }),
                false => DatafusionContext::with_permanent_schema_provider(name, |provider| {
                    let table = task::block_on(provider.table(table_name))
                        .ok_or(NotFound::Table(table_name.to_string()))?;

                    Ok(provider_as_source(table))
                }),
            },
            None => {
                let current_schemas = unsafe {
                    direct_function_call::<Array<pg_sys::Datum>>(
                        pg_sys::current_schemas,
                        &[Some(pg_sys::Datum::from(true))],
                    )
                };

                if let Some(current_schemas) = current_schemas {
                    for datum in current_schemas.iter().flatten() {
                        let schema_name =
                            unsafe { CStr::from_ptr(datum.cast_mut_ptr::<i8>()).to_str()? };
                        let schema_registered =
                            DatafusionContext::with_postgres_catalog(|catalog| {
                                Ok(catalog.schema(schema_name).is_some())
                            })?;

                        if !schema_registered {
                            continue;
                        }

                        let table_registered = match schema_name.is_temp_schema()? {
                            true => DatafusionContext::with_temp_schema_provider(
                                schema_name,
                                |provider| Ok(task::block_on(provider.table(table_name)).is_some()),
                            ),
                            false => DatafusionContext::with_permanent_schema_provider(
                                schema_name,
                                |provider| Ok(task::block_on(provider.table(table_name)).is_some()),
                            ),
                        }?;

                        if !table_registered {
                            continue;
                        }

                        return match schema_name.is_temp_schema()? {
                            true => DatafusionContext::with_temp_schema_provider(
                                schema_name,
                                |provider| {
                                    let table = task::block_on(provider.table(table_name))
                                        .ok_or(NotFound::Table(table_name.to_string()))?;

                                    Ok(provider_as_source(table))
                                },
                            ),
                            false => DatafusionContext::with_permanent_schema_provider(
                                schema_name,
                                |provider| {
                                    let table = task::block_on(provider.table(table_name))
                                        .ok_or(NotFound::Table(table_name.to_string()))?;

                                    Ok(provider_as_source(table))
                                },
                            ),
                        };
                    }
                }

                Err(NotFound::Table(table_name.to_string()).into())
            }
        }
    }
}

impl ContextProvider for ParadeContextProvider {
    fn get_table_source(
        &self,
        reference: TableReference,
    ) -> Result<Arc<dyn TableSource>, DataFusionError> {
        Self::get_table_source_impl(reference)
            .map_err(|err| DataFusionError::Execution(err.to_string()))
    }

    fn get_function_meta(&self, _name: &str) -> Option<Arc<ScalarUDF>> {
        None
    }

    fn get_aggregate_meta(&self, _name: &str) -> Option<Arc<AggregateUDF>> {
        None
    }

    fn get_variable_type(&self, _variable_names: &[String]) -> Option<DataType> {
        None
    }

    fn get_window_meta(&self, _name: &str) -> Option<Arc<WindowUDF>> {
        None
    }

    fn options(&self) -> &ConfigOptions {
        &self.options
    }
}

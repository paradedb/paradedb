use async_std::sync::Mutex;
use async_std::task;
use datafusion::execution::runtime_env::{RuntimeConfig, RuntimeEnv};
use datafusion::prelude::{SessionConfig, SessionContext};
use once_cell::sync::Lazy;
use pgrx::*;
use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};
use std::ffi::CStr;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use super::catalog::*;
use super::context::ContextError;

const SESSION_ID: &str = "lakehouse_session_context";

static SESSION_CACHE: Lazy<Arc<Mutex<HashMap<String, SessionContext>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub struct Session;

impl Session {
    pub fn with_session_context<F, R>(f: F) -> Result<R, ContextError>
    where
        F: for<'a> FnOnce(
            &'a SessionContext,
        ) -> Pin<Box<dyn Future<Output = Result<R, ContextError>> + 'a>>,
    {
        let mut lock = task::block_on(SESSION_CACHE.lock());

        let context = match lock.entry(SESSION_ID.to_string()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => {
                // Set current timezone
                let mut session_config = SessionConfig::from_env()?.with_information_schema(true);
                let session_timezone = unsafe {
                    CStr::from_ptr(pg_sys::pg_get_timezone_name(pg_sys::session_timezone))
                        .to_str()
                        .unwrap_or_else(|err| panic!("{:?}", err))
                };
                session_config.options_mut().execution.time_zone =
                    Some(session_timezone.to_string());

                // Create a new context
                let rn_config = RuntimeConfig::new();
                let runtime_env = RuntimeEnv::new(rn_config)?;
                let mut context =
                    SessionContext::new_with_config_rt(session_config, Arc::new(runtime_env));

                // Register catalog
                context.register_catalog_list(Arc::new(LakehouseCatalogList::new()));
                context.register_catalog(&Self::catalog_name()?, Arc::new(LakehouseCatalog::new()));

                entry.insert(context)
            }
        };

        task::block_on(f(context))
    }

    pub fn with_catalog<F, R>(f: F) -> Result<R, ContextError>
    where
        F: for<'a> FnOnce(
            &'a LakehouseCatalog,
        ) -> Pin<Box<dyn Future<Output = Result<R, ContextError>> + 'a>>,
    {
        let mut lock = task::block_on(SESSION_CACHE.lock());

        let context = match lock.entry(SESSION_ID.to_string()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(task::block_on(Self::init())?),
        };

        let catalog_provider = context.catalog(&Self::catalog_name()?).ok_or(
            ContextError::CatalogProviderNotFound(Self::catalog_name()?.to_string()),
        )?;

        let downcast_catalog = catalog_provider
            .as_any()
            .downcast_ref::<LakehouseCatalog>()
            .ok_or(ContextError::CatalogNotFound(
                Self::catalog_name()?.to_string(),
            ))?;

        task::block_on(f(downcast_catalog))
    }

    pub fn with_schema_provider<F, R>(schema_name: &str, f: F) -> Result<R, ContextError>
    where
        F: for<'a> FnOnce(
            &'a LakehouseSchemaProvider,
        ) -> Pin<Box<dyn Future<Output = Result<R, ContextError>> + 'a>>,
    {
        let mut lock = task::block_on(SESSION_CACHE.lock());

        let context = match lock.entry(SESSION_ID.to_string()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(task::block_on(Self::init())?),
        };

        let catalog =
            context
                .catalog(&Self::catalog_name()?)
                .ok_or(ContextError::CatalogNotFound(
                    Self::catalog_name()?.to_string(),
                ))?;

        if catalog.schema(schema_name).is_none() {
            let new_schema_provider = Arc::new(task::block_on(LakehouseSchemaProvider::try_new(
                schema_name,
            ))?);
            catalog.register_schema(schema_name, new_schema_provider)?;
        }

        let schema_provider = context
            .catalog(&Self::catalog_name()?)
            .ok_or(ContextError::CatalogNotFound(
                Self::catalog_name()?.to_string(),
            ))?
            .schema(schema_name)
            .ok_or(ContextError::SchemaProviderNotFound(
                schema_name.to_string(),
            ))?;

        let downcast_provider = schema_provider
            .as_any()
            .downcast_ref::<LakehouseSchemaProvider>()
            .ok_or(ContextError::SchemaNotFound(schema_name.to_string()))?;

        drop(lock);

        task::block_on(f(downcast_provider))
    }

    pub fn catalog_name() -> Result<String, ContextError> {
        let catalog_oid = unsafe { pg_sys::MyDatabaseId };
        let database_name = unsafe { pg_sys::get_database_name(catalog_oid) };
        if database_name.is_null() {
            return Err(ContextError::DatabaseNotFound(
                catalog_oid.as_u32().to_string(),
            ));
        }

        Ok(unsafe { CStr::from_ptr(database_name).to_str()?.to_owned() })
    }

    fn init() -> Result<SessionContext, ContextError> {
        let mut session_config = SessionConfig::from_env()?.with_information_schema(true);
        let session_timezone = unsafe {
            CStr::from_ptr(pg_sys::pg_get_timezone_name(pg_sys::session_timezone))
                .to_str()
                .unwrap_or_else(|err| panic!("{:?}", err))
        };
        session_config.options_mut().execution.time_zone =
            Some(session_timezone.to_string());

        // Create a new context
        let rn_config = RuntimeConfig::new();
        let runtime_env = RuntimeEnv::new(rn_config)?;
        let mut context =
            SessionContext::new_with_config_rt(session_config, Arc::new(runtime_env));

        // Register catalog
        context.register_catalog_list(Arc::new(LakehouseCatalogList::new()));
        context.register_catalog(&Self::catalog_name()?, Arc::new(LakehouseCatalog::new()));

        Ok(context)
    }
}

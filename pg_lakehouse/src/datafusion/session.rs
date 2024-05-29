use async_std::sync::RwLock;
use datafusion::common::DataFusionError;
use datafusion::execution::runtime_env::{RuntimeConfig, RuntimeEnv};
use datafusion::prelude::{SessionConfig, SessionContext};
use once_cell::sync::Lazy;
use pgrx::*;
use std::collections::{hash_map::Entry::Vacant, HashMap};
use std::ffi::CStr;
use std::sync::Arc;
use thiserror::Error;

use super::catalog::*;
use super::schema::LakehouseSchemaProvider;

const SESSION_ID: &str = "lakehouse_session_context";

type SessionCache = Lazy<Arc<RwLock<HashMap<String, Arc<SessionContext>>>>>;
static SESSION_CACHE: SessionCache = Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

pub struct Session;

impl Session {
    pub async fn session_context() -> Result<Arc<SessionContext>, SessionError> {
        {
            let mut write_lock = SESSION_CACHE.write().await;
            if let Vacant(entry) = write_lock.entry(SESSION_ID.to_string()) {
                entry.insert(Self::init()?);
            }
        }

        let read_lock = SESSION_CACHE.read().await;
        let context = read_lock
            .get(SESSION_ID)
            .ok_or(SessionError::ContextNotFound)?;

        Ok(context.clone())
    }

    pub async fn catalog() -> Result<LakehouseCatalog, SessionError> {
        {
            let mut write_lock = SESSION_CACHE.write().await;
            if let Vacant(entry) = write_lock.entry(SESSION_ID.to_string()) {
                entry.insert(Self::init()?);
            }
        }

        let read_lock = SESSION_CACHE.read().await;
        let context = read_lock
            .get(SESSION_ID)
            .ok_or(SessionError::ContextNotFound)?;

        let catalog_provider = context.catalog(&Self::catalog_name()?).ok_or(
            SessionError::CatalogProviderNotFound(Self::catalog_name()?.to_string()),
        )?;

        let downcast_catalog = catalog_provider
            .as_any()
            .downcast_ref::<LakehouseCatalog>()
            .ok_or(SessionError::CatalogNotFound(
                Self::catalog_name()?.to_string(),
            ))?;

        Ok(downcast_catalog.clone())
    }

    pub async fn schema_provider(
        schema_name: &str,
    ) -> Result<LakehouseSchemaProvider, SessionError> {
        {
            let mut write_lock = SESSION_CACHE.write().await;
            if let Vacant(entry) = write_lock.entry(SESSION_ID.to_string()) {
                entry.insert(Self::init()?);
            }
        }

        let read_lock = SESSION_CACHE.read().await;
        let context = read_lock
            .get(SESSION_ID)
            .ok_or(SessionError::ContextNotFound)?;

        let catalog =
            context
                .catalog(&Self::catalog_name()?)
                .ok_or(SessionError::CatalogNotFound(
                    Self::catalog_name()?.to_string(),
                ))?;

        if catalog.schema(schema_name).is_none() {
            let new_schema_provider = Arc::new(LakehouseSchemaProvider::new(schema_name));
            catalog.register_schema(schema_name, new_schema_provider)?;
        }

        let schema_provider = context
            .catalog(&Self::catalog_name()?)
            .ok_or(SessionError::CatalogNotFound(
                Self::catalog_name()?.to_string(),
            ))?
            .schema(schema_name)
            .ok_or(SessionError::SchemaProviderNotFound(
                schema_name.to_string(),
            ))?;

        let downcast_provider = schema_provider
            .as_any()
            .downcast_ref::<LakehouseSchemaProvider>()
            .ok_or(SessionError::SchemaNotFound(schema_name.to_string()))?;

        Ok(downcast_provider.clone())
    }

    pub fn catalog_name() -> Result<String, SessionError> {
        let catalog_oid = unsafe { pg_sys::MyDatabaseId };
        let database_name = unsafe { pg_sys::get_database_name(catalog_oid) };
        if database_name.is_null() {
            return Err(SessionError::DatabaseNotFound(
                catalog_oid.as_u32().to_string(),
            ));
        }

        Ok(unsafe { CStr::from_ptr(database_name).to_str()?.to_owned() })
    }

    fn init() -> Result<Arc<SessionContext>, SessionError> {
        let mut session_config = SessionConfig::from_env()?.with_information_schema(true);
        let session_timezone = unsafe {
            CStr::from_ptr(pg_sys::pg_get_timezone_name(pg_sys::session_timezone))
                .to_str()
                .unwrap_or_else(|err| panic!("{}", err))
        };
        session_config.options_mut().execution.time_zone = Some(session_timezone.to_string());

        // Create a new context
        let rn_config = RuntimeConfig::new();
        let runtime_env = RuntimeEnv::new(rn_config)?;
        let mut context = SessionContext::new_with_config_rt(session_config, Arc::new(runtime_env));

        // Register catalog
        context.register_catalog_list(Arc::new(LakehouseCatalogList::new()));
        context.register_catalog(&Self::catalog_name()?, Arc::new(LakehouseCatalog::new()));

        Ok(Arc::new(context))
    }
}

#[derive(Error, Debug)]
pub enum SessionError {
    #[error(transparent)]
    DataFusionError(#[from] DataFusionError),

    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("Catalog {0} not found")]
    CatalogNotFound(String),

    #[error("Catalog provider {0} not found")]
    CatalogProviderNotFound(String),

    #[error("Session context not initialized. This is an unexpected bug with ParadeDB.")]
    ContextNotFound,

    #[error("Database {0} not found")]
    DatabaseNotFound(String),

    #[error("Schema {0} not found")]
    SchemaNotFound(String),

    #[error("Schema provider {0} not found")]
    SchemaProviderNotFound(String),
}

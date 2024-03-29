use async_std::sync::{Mutex, MutexGuard};
use async_std::task;

use deltalake::datafusion::execution::runtime_env::{RuntimeConfig, RuntimeEnv};

use deltalake::datafusion::prelude::{SessionConfig, SessionContext};
use once_cell::sync::Lazy;
use pgrx::*;
use std::any::type_name;
use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};
use std::ffi::{CStr, CString};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::datafusion::catalog::{ParadeCatalog, ParadeCatalogList};
use crate::datafusion::directory::ParadeDirectory;
use crate::datafusion::schema::ParadeSchemaProvider;
use crate::datafusion::table::Tables;
use crate::errors::{NotFound, ParadeError};

const SESSION_ID: &str = "datafusion_session_context";
const EXTENSION_NAME: &str = "pg_analytics";

static SESSION_CACHE: Lazy<Arc<Mutex<HashMap<String, SessionContext>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub struct Session;

impl Session {
    pub fn with_session_context<F, R>(f: F) -> Result<R, ParadeError>
    where
        F: for<'a> FnOnce(
            &'a SessionContext,
        ) -> Pin<Box<dyn Future<Output = Result<R, ParadeError>> + 'a>>,
    {
        let mut lock = task::block_on(SESSION_CACHE.lock());

        let context = match lock.entry(SESSION_ID.to_string()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(task::block_on(Self::init(Self::catalog_oid()?))?),
        };

        task::block_on(f(context))
    }

    pub fn with_catalog<F, R>(f: F) -> Result<R, ParadeError>
    where
        F: FnOnce(&ParadeCatalog) -> Result<R, ParadeError>,
    {
        let mut lock = task::block_on(SESSION_CACHE.lock());

        let context = match lock.entry(SESSION_ID.to_string()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(task::block_on(Self::init(Self::catalog_oid()?))?),
        };

        let catalog_provider = context
            .catalog(&Self::catalog_name()?)
            .ok_or(NotFound::Catalog(Self::catalog_name()?.to_string()))?;

        let parade_catalog = catalog_provider
            .as_any()
            .downcast_ref::<ParadeCatalog>()
            .ok_or(NotFound::Value(type_name::<ParadeCatalog>().to_string()))?;

        f(parade_catalog)
    }

    pub fn with_schema_provider<F, R>(schema_name: &str, f: F) -> Result<R, ParadeError>
    where
        F: for<'a> FnOnce(
            &'a ParadeSchemaProvider,
        ) -> Pin<Box<dyn Future<Output = Result<R, ParadeError>> + 'a>>,
    {
        let mut lock = task::block_on(SESSION_CACHE.lock());

        let context = match lock.entry(SESSION_ID.to_string()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(task::block_on(Self::init(Self::catalog_oid()?))?),
        };

        let schema_provider = context
            .catalog(&Self::catalog_name()?)
            .ok_or(NotFound::Catalog(Self::catalog_name()?.to_string()))?
            .schema(schema_name)
            .ok_or(NotFound::Schema(schema_name.to_string()))?;

        let parade_provider = schema_provider
            .as_any()
            .downcast_ref::<ParadeSchemaProvider>()
            .ok_or(NotFound::Value(
                type_name::<ParadeSchemaProvider>().to_string(),
            ))?;

        task::block_on(f(parade_provider))
    }

    pub fn with_tables<F, R>(schema_name: &str, f: F) -> Result<R, ParadeError>
    where
        F: for<'a> FnOnce(
            MutexGuard<'a, Tables>,
        ) -> Pin<Box<dyn Future<Output = Result<R, ParadeError>> + 'a>>,
    {
        let tables = Self::with_schema_provider(schema_name, |provider| {
            Box::pin(async move { provider.tables() })
        })?;

        let lock = task::block_on(tables.lock());
        task::block_on(f(lock))
    }

    pub async fn init(catalog_oid: pg_sys::Oid) -> Result<SessionContext, ParadeError> {
        let preload_libraries = unsafe {
            CStr::from_ptr(pg_sys::GetConfigOptionByName(
                CString::new("shared_preload_libraries")?.as_ptr(),
                std::ptr::null_mut(),
                true,
            ))
            .to_str()?
        };

        if !preload_libraries.contains(EXTENSION_NAME) {
            return Err(ParadeError::SharedPreload);
        }

        let session_config = SessionConfig::from_env()?.with_information_schema(true);

        let rn_config = RuntimeConfig::new();
        let runtime_env = RuntimeEnv::new(rn_config)?;
        let mut context = SessionContext::new_with_config_rt(session_config, Arc::new(runtime_env));

        // Create schema directory if it doesn't exist
        ParadeDirectory::create_catalog_path(catalog_oid)?;

        // Register catalog list
        context.register_catalog_list(Arc::new(ParadeCatalogList::try_new()?));

        // Create and register catalog
        let catalog = ParadeCatalog::try_new()?;
        catalog.init().await?;
        context.register_catalog(&Self::catalog_name()?, Arc::new(catalog));

        // Register UDFs
        // register_postgres_udfs(&context);

        Ok(context)
    }

    pub fn catalog_name() -> Result<String, ParadeError> {
        let database_name = unsafe { pg_sys::get_database_name(Self::catalog_oid()?) };
        if database_name.is_null() {
            return Err(NotFound::Database(Self::catalog_oid()?.as_u32().to_string()).into());
        }

        Ok(unsafe { CStr::from_ptr(database_name).to_str()?.to_owned() })
    }

    pub fn catalog_oid() -> Result<pg_sys::Oid, ParadeError> {
        Ok(unsafe { pg_sys::MyDatabaseId })
    }
}

// #[inline]
// fn register_postgres_udfs(context: &SessionContext) {
//     context.register_udf(create_udf(
//         "GetTransactionIdDidAbort",
//         vec![DataType::Int64],
//         Arc::new(DataType::Boolean),
//         Volatility::Immutable,
//         Arc::new(|args| transaction_did_abort(args)),
//     ));
// }

// fn transaction_did_abort(args: &[ColumnarValue]) -> Result<ColumnarValue, DataFusionError> {
//     info!("got args {:?}", args);
//     Ok(ColumnarValue::Scalar(ScalarValue::Boolean(Some(true))))
// }

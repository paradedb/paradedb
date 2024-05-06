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
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

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

        let session_config = SessionConfig::from_env()?.with_information_schema(true);
        let rn_config = RuntimeConfig::new();
        let runtime_env = RuntimeEnv::new(rn_config)?;
        let context = SessionContext::new_with_config_rt(session_config, Arc::new(runtime_env));

        let context = match lock.entry(SESSION_ID.to_string()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(context),
        };

        task::block_on(f(context))
    }
}

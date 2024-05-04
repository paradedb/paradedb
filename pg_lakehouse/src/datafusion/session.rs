use async_std::sync::Mutex;
use async_std::task;
use datafusion::prelude::SessionContext;
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

        let context = match lock.entry(SESSION_ID.to_string()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(SessionContext::new()),
        };

        task::block_on(f(context))
    }
}

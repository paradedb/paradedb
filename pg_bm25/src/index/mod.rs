#![allow(unused)]
pub mod score;
pub mod search;
pub mod state;

use once_cell::sync::Lazy;
use pgrx::pg_sys::Alias;
pub use search::*;

use crate::schema::{SearchConfig, SearchField};
use derive_more::{AsRef, From};
use shared::postgres::transaction::{Transaction, TransactionError};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
};
use thiserror::Error;

static CURRENT_SEARCH_CONFIG: Lazy<Arc<CurrentSearchConfig>> = Lazy::new(|| {
    Arc::new(CurrentSearchConfig {
        map: Arc::new(Mutex::new(HashMap::new())),
    })
});

const TRANSACTION_CALLBACK_CACHE_ID: &str = "parade_current_search_config";
const DEFAULT_ALIAS: &str = "";

#[derive(AsRef, Eq, PartialEq, Hash, From)]
#[as_ref(forward)]
struct SearchAlias(String);

impl From<&str> for SearchAlias {
    fn from(value: &str) -> Self {
        SearchAlias(value.to_string())
    }
}

struct CurrentSearchConfig {
    map: Arc<Mutex<HashMap<SearchAlias, SearchConfig>>>,
}

impl CurrentSearchConfig {
    fn register_callback(&self) -> Result<(), TransactionError> {
        let cloned_map = self.map.clone();
        Transaction::call_once_on_commit(TRANSACTION_CALLBACK_CACHE_ID, move || {
            cloned_map
                .lock()
                .map_err(CurrentSearchConfigError::from)
                .expect("could not lock search config lookup in commit callback")
                .drain();
        })?;
        let cloned_map = self.map.clone();
        Transaction::call_once_on_abort(TRANSACTION_CALLBACK_CACHE_ID, move || {
            cloned_map
                .lock()
                .map_err(CurrentSearchConfigError::from)
                .expect("could not lock search config lookup in commit callback")
                .drain();
        })?;
        Ok(())
    }

    fn set_default(&mut self, config: SearchConfig) -> Result<(), CurrentSearchConfigError> {
        match self
            .map
            .lock()
            .map_err(CurrentSearchConfigError::from)?
            .insert(DEFAULT_ALIAS.into(), config)
        {
            None => Ok(()),
            Some(_) => Err(CurrentSearchConfigError::AliasRequired),
        }
    }

    fn get_default(&self) -> Result<SearchConfig, CurrentSearchConfigError> {
        self.map
            .lock()
            .map_err(CurrentSearchConfigError::from)?
            .get(&DEFAULT_ALIAS.into())
            .cloned()
            .ok_or(CurrentSearchConfigError::NoQuery)
    }

    fn set_alias(
        &mut self,
        SearchAlias(alias): SearchAlias,
        config: SearchConfig,
    ) -> Result<(), CurrentSearchConfigError> {
        if alias == DEFAULT_ALIAS {
            Err(CurrentSearchConfigError::EmptyAlias)
        } else {
            self.map
                .lock()
                .map_err(CurrentSearchConfigError::from)?
                .insert(SearchAlias(alias.clone()), config)
                .ok_or(CurrentSearchConfigError::DuplicateAlias(alias))?;
            Ok(())
        }
    }

    fn get_alias(
        &self,
        SearchAlias(alias): SearchAlias,
    ) -> Result<SearchConfig, CurrentSearchConfigError> {
        self.map
            .lock()
            .map_err(CurrentSearchConfigError::from)?
            .get(&alias.clone().into())
            .cloned()
            .ok_or(CurrentSearchConfigError::AliasLookup(alias))
    }
}

#[derive(Debug, Error)]
enum CurrentSearchConfigError {
    #[error("to use multiple pg_search queries, pass a query alias with the 'as' parameter")]
    AliasRequired,
    #[error("no pg_search query in current transaction")]
    NoQuery,
    #[error("a pg_search alias string cannot be empty")]
    EmptyAlias,
    #[error("a pg_search alias must be unique, found duplicate: '{0}'")]
    DuplicateAlias(String),
    #[error("no query found with alias: '{0}'")]
    AliasLookup(String),
    #[error("could not lock the current search config lookup: {0}")]
    Lock(String),
}

impl<T> From<PoisonError<T>> for CurrentSearchConfigError {
    fn from(err: PoisonError<T>) -> Self {
        CurrentSearchConfigError::Lock(format!("{err}"))
    }
}

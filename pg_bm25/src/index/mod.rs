#![allow(unused)]
pub mod score;
pub mod search;
pub mod state;

use crate::schema::{SearchConfig, SearchField, SearchFieldId};
use core::panic;
use derive_more::{AsRef, Display, From};
use once_cell::sync::Lazy;
use pgrx::pg_sys::Alias;
pub use search::*;
use shared::postgres::transaction::{Transaction, TransactionError};
use std::{
    borrow::Borrow,
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
};
use tantivy::DocAddress;
use thiserror::Error;

static CURRENT_SEARCH: Lazy<Arc<Mutex<CurrentSearch>>> = Lazy::new(|| {
    Arc::new(Mutex::new(CurrentSearch {
        doc_map: HashMap::new(),
        config_map: HashMap::new(),
    }))
});

const TRANSACTION_CALLBACK_CACHE_ID: &str = "parade_current_search";

#[derive(Clone, Debug, Display, AsRef, Eq, PartialEq, Hash, From)]
#[as_ref(forward)]
pub struct SearchAlias(String);

impl From<&str> for SearchAlias {
    fn from(value: &str) -> Self {
        SearchAlias(value.to_string())
    }
}

impl Default for SearchAlias {
    fn default() -> Self {
        SearchAlias("".into())
    }
}

/// A singleton lookup that stores information about pg_search queries in the
/// current transaction. Used by functions like `rank` and `hybrid` to lookup
/// a search result given a key field. Expected to be fully cleared at the end
/// of every transaction.
pub struct CurrentSearch {
    /// A map from key_field value to document address.
    doc_map: HashMap<SearchAlias, HashMap<i64, DocAddress>>,
    /// A map from query alias to search configuration.
    config_map: HashMap<SearchAlias, SearchConfig>,
}

impl CurrentSearch {
    fn register_callback() -> Result<(), TransactionError> {
        // Commit and abort are mutually exclusive. One of the two is guaranteed
        // to be called on any transaction. We'll use that opportunity to clean
        // up the cache.
        Transaction::call_once_on_commit(TRANSACTION_CALLBACK_CACHE_ID, move || {
            pgrx::log!("IN THE COMMIT CALLBACK");
            let mut current_search = CURRENT_SEARCH
                .lock()
                .expect("could not lock current search lookup in commit callback");
            current_search.doc_map.drain();
            current_search.config_map.drain();
        })?;
        pgrx::log!("registering callback!!!!");
        Transaction::call_once_on_abort(TRANSACTION_CALLBACK_CACHE_ID, move || {
            pgrx::log!("IN THE ABORT CALLBACK");
            let mut current_search = CURRENT_SEARCH
                .lock()
                .expect("could not lock current search lookup in abort callback");
            current_search.doc_map.drain();
            current_search.config_map.drain();
        })?;
        Ok(())
    }

    fn get_doc_default(&self, key: i64) -> Result<DocAddress, CurrentSearchError> {
        self.doc_map
            .get(&SearchAlias::default())
            .ok_or(CurrentSearchError::NoQuery)?
            .get(&key)
            .cloned()
            .ok_or(CurrentSearchError::DocLookup(key))
    }

    fn get_doc_alias(
        &self,
        key: i64,
        alias: SearchAlias,
    ) -> Result<DocAddress, CurrentSearchError> {
        self.doc_map
            .get(&alias)
            .ok_or(CurrentSearchError::AliasLookup(alias))?
            .get(&key)
            .cloned()
            .ok_or(CurrentSearchError::DocLookup(key))
    }

    pub fn get_doc(key: i64, alias: Option<SearchAlias>) -> Result<DocAddress, CurrentSearchError> {
        let mut current_search = CURRENT_SEARCH.lock().map_err(CurrentSearchError::from)?;
        if let Some(alias) = alias {
            current_search.get_doc_alias(key, alias)
        } else {
            current_search.get_doc_default(key)
        }
    }

    fn set_doc_default(&mut self, key: i64, address: DocAddress) -> Result<(), CurrentSearchError> {
        self.doc_map
            .entry(SearchAlias::default())
            .or_default()
            .insert(key, address);
        Ok(())
    }

    fn set_doc_alias(
        &mut self,
        key: i64,
        address: DocAddress,
        alias: SearchAlias,
    ) -> Result<(), CurrentSearchError> {
        if alias == SearchAlias::default() {
            Err(CurrentSearchError::EmptyAlias)
        } else {
            self.doc_map.entry(alias).or_default().insert(key, address);
            Ok(())
        }
    }

    pub fn set_doc(
        key: i64,
        address: DocAddress,
        alias: Option<SearchAlias>,
    ) -> Result<(), CurrentSearchError> {
        let mut current_search = CURRENT_SEARCH.lock().map_err(CurrentSearchError::from)?;
        if let Some(alias) = alias {
            current_search.set_doc_alias(key, address, alias)
        } else {
            current_search.set_doc_default(key, address)
        }
    }

    fn get_config_default(&self) -> Result<SearchConfig, CurrentSearchError> {
        self.config_map
            .get(&SearchAlias::default())
            .cloned()
            .ok_or(CurrentSearchError::NoQuery)
    }

    fn get_config_alias(&self, alias: SearchAlias) -> Result<SearchConfig, CurrentSearchError> {
        self.config_map
            .get(&alias)
            .cloned()
            .ok_or(CurrentSearchError::AliasLookup(alias))
    }

    pub fn get_config(alias: Option<SearchAlias>) -> Result<SearchConfig, CurrentSearchError> {
        let mut current_search = CURRENT_SEARCH.lock().map_err(CurrentSearchError::from)?;
        if let Some(alias) = alias {
            current_search.get_config_alias(alias)
        } else {
            current_search.get_config_default()
        }
    }

    fn set_config_default(&mut self, config: SearchConfig) -> Result<(), CurrentSearchError> {
        match self.config_map.insert(SearchAlias::default(), config) {
            None => Ok(()),
            Some(_) => Err(CurrentSearchError::AliasRequired),
        }
    }

    fn set_config_alias(
        &mut self,
        config: SearchConfig,
        alias: SearchAlias,
    ) -> Result<(), CurrentSearchError> {
        if alias == SearchAlias::default() {
            Err(CurrentSearchError::EmptyAlias)
        } else {
            self.config_map
                .insert(alias.clone(), config)
                .ok_or(CurrentSearchError::DuplicateAlias(alias))?;
            Ok(())
        }
    }

    pub fn set_config(config: SearchConfig) -> Result<(), CurrentSearchError> {
        Self::register_callback();

        let mut current_search = CURRENT_SEARCH.lock().map_err(CurrentSearchError::from)?;
        pgrx::log!("SETTING CONFIG: {:?}", current_search.config_map);
        if let Some(ref alias) = config.alias {
            let alias = SearchAlias(alias.clone());
            current_search.set_config_alias(config, alias)
        } else {
            current_search.set_config_default(config)
        }
    }
}

#[derive(Debug, Error)]
pub enum CurrentSearchError {
    #[error("to use multiple pg_search queries, pass a query alias with the 'as' parameter")]
    AliasRequired,
    #[error("no pg_search query in current transaction")]
    NoQuery,
    #[error("a pg_search alias string cannot be empty")]
    EmptyAlias,
    #[error("a pg_search alias must be unique, found duplicate: '{0}'")]
    DuplicateAlias(SearchAlias),
    #[error("error looking up result data for document with id: '{0}'")]
    DocLookup(i64),
    #[error("no query found with alias: '{0}'")]
    AliasLookup(SearchAlias),
    #[error("could not lock the current search config lookup: {0}")]
    Lock(String),
}

impl<T> From<PoisonError<T>> for CurrentSearchError {
    fn from(err: PoisonError<T>) -> Self {
        CurrentSearchError::Lock(format!("{err}"))
    }
}

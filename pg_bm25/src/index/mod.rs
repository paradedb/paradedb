#![allow(unused)]
pub mod score;
pub mod search;
pub mod state;

use once_cell::sync::Lazy;
use pgrx::pg_sys::Alias;
pub use search::*;
use tantivy::DocAddress;

use crate::schema::{SearchConfig, SearchField, SearchFieldId};
use derive_more::{AsRef, Display, From};
use shared::postgres::transaction::{Transaction, TransactionError};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
};
use thiserror::Error;

static CURRENT_SEARCH: Lazy<Arc<CurrentSearch>> = Lazy::new(|| {
    Arc::new(CurrentSearch {
        doc_map: Arc::new(Mutex::new(HashMap::new())),
        config_map: Arc::new(Mutex::new(HashMap::new())),
    })
});

const TRANSACTION_CALLBACK_CACHE_ID: &str = "parade_current_search";
const DEFAULT_ALIAS: &str = "";

#[derive(Debug, Display, AsRef, Eq, PartialEq, Hash, From)]
#[as_ref(forward)]
struct SearchAlias(String);

impl From<&str> for SearchAlias {
    fn from(value: &str) -> Self {
        SearchAlias(value.to_string())
    }
}

struct CurrentSearch {
    /// A map from key_field value to document address.
    doc_map: Arc<Mutex<HashMap<SearchAlias, HashMap<i64, DocAddress>>>>,
    /// A map from query alias to search configuration.
    config_map: Arc<Mutex<HashMap<SearchAlias, SearchConfig>>>,
}

impl CurrentSearch {
    fn register_callback(&self) -> Result<(), TransactionError> {
        // Commit and abort are mutually exclusive. One of the two is guaranteed
        // to be called on any transaction. We'll use that opportunity to clean
        // up the cache.
        let cloned_map = self.config_map.clone();
        Transaction::call_once_on_commit(TRANSACTION_CALLBACK_CACHE_ID, move || {
            cloned_map
                .lock()
                .map_err(CurrentSearchError::from)
                .expect("could not lock search config lookup in commit callback")
                .drain();
        })?;
        let cloned_map = self.config_map.clone();
        Transaction::call_once_on_abort(TRANSACTION_CALLBACK_CACHE_ID, move || {
            cloned_map
                .lock()
                .map_err(CurrentSearchError::from)
                .expect("could not lock search config lookup in commit callback")
                .drain();
        })?;
        Ok(())
    }

    fn get_doc_default(&self, key: i64) -> Result<DocAddress, CurrentSearchError> {
        self.doc_map
            .lock()
            .map_err(CurrentSearchError::from)?
            .get(&DEFAULT_ALIAS.into())
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
            .lock()
            .map_err(CurrentSearchError::from)?
            .get(&alias)
            .ok_or(CurrentSearchError::AliasLookup(alias))?
            .get(&key)
            .cloned()
            .ok_or(CurrentSearchError::DocLookup(key))
    }

    pub fn get_doc(
        &self,
        key: i64,
        alias: Option<SearchAlias>,
    ) -> Result<DocAddress, CurrentSearchError> {
        if let Some(alias) = alias {
            self.get_doc_alias(key, alias)
        } else {
            self.get_doc_default(key)
        }
    }

    fn set_doc_default(&mut self, key: i64, address: DocAddress) -> Result<(), CurrentSearchError> {
        self.doc_map
            .lock()
            .map_err(CurrentSearchError::from)?
            .entry(DEFAULT_ALIAS.into())
            .or_insert_with(|| HashMap::new())
            .insert(key, address);
        Ok(())
    }

    fn set_doc_alias(
        &mut self,
        key: i64,
        address: DocAddress,
        SearchAlias(alias): SearchAlias,
    ) -> Result<(), CurrentSearchError> {
        if alias == DEFAULT_ALIAS {
            Err(CurrentSearchError::EmptyAlias)
        } else {
            self.doc_map
                .lock()
                .map_err(CurrentSearchError::from)?
                .entry(alias.into())
                .or_insert_with(|| HashMap::new())
                .insert(key, address);
            Ok(())
        }
    }

    pub fn set_doc(
        &mut self,
        key: i64,
        address: DocAddress,
        alias: Option<SearchAlias>,
    ) -> Result<(), CurrentSearchError> {
        if let Some(alias) = alias {
            self.set_doc_alias(key, address, alias)
        } else {
            self.set_doc_default(key, address)
        }
    }

    fn get_config_default(&self) -> Result<SearchConfig, CurrentSearchError> {
        self.config_map
            .lock()
            .map_err(CurrentSearchError::from)?
            .get(&DEFAULT_ALIAS.into())
            .cloned()
            .ok_or(CurrentSearchError::NoQuery)
    }

    fn get_config_alias(&self, alias: SearchAlias) -> Result<SearchConfig, CurrentSearchError> {
        self.config_map
            .lock()
            .map_err(CurrentSearchError::from)?
            .get(&alias)
            .cloned()
            .ok_or(CurrentSearchError::AliasLookup(alias))
    }

    pub fn get_config(
        &self,
        alias: Option<SearchAlias>,
    ) -> Result<SearchConfig, CurrentSearchError> {
        if let Some(alias) = alias {
            self.get_config_alias(alias)
        } else {
            self.get_config_default()
        }
    }

    fn set_config_default(&mut self, config: SearchConfig) -> Result<(), CurrentSearchError> {
        match self
            .config_map
            .lock()
            .map_err(CurrentSearchError::from)?
            .insert(DEFAULT_ALIAS.into(), config)
        {
            None => Ok(()),
            Some(_) => Err(CurrentSearchError::AliasRequired),
        }
    }

    fn set_config_alias(
        &mut self,
        config: SearchConfig,
        SearchAlias(alias): SearchAlias,
    ) -> Result<(), CurrentSearchError> {
        if alias == DEFAULT_ALIAS {
            Err(CurrentSearchError::EmptyAlias)
        } else {
            self.config_map
                .lock()
                .map_err(CurrentSearchError::from)?
                .insert(SearchAlias(alias.clone()), config)
                .ok_or(CurrentSearchError::DuplicateAlias(alias))?;
            Ok(())
        }
    }

    pub fn set_config(
        &mut self,
        config: SearchConfig,
        alias: Option<SearchAlias>,
    ) -> Result<(), CurrentSearchError> {
        if let Some(alias) = alias {
            self.set_config_alias(config, alias)
        } else {
            self.set_config_default(config)
        }
    }
}

#[derive(Debug, Error)]
enum CurrentSearchError {
    #[error("to use multiple pg_search queries, pass a query alias with the 'as' parameter")]
    AliasRequired,
    #[error("no pg_search query in current transaction")]
    NoQuery,
    #[error("a pg_search alias string cannot be empty")]
    EmptyAlias,
    #[error("a pg_search alias must be unique, found duplicate: '{0}'")]
    DuplicateAlias(String),
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

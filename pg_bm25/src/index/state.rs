use derive_more::{AsRef, Display, From};
use once_cell::sync::Lazy;
use shared::postgres::transaction::{Transaction, TransactionError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use tantivy::collector::TopDocs;
use tantivy::schema::FieldType;
use tantivy::{query::Query, DocAddress, Score, Searcher};
use tantivy::{DocId, SegmentReader, Snippet, SnippetGenerator};
use thiserror::Error;

use super::score::SearchIndexScore;
use super::SearchIndex;
use crate::schema::{SearchConfig, SearchFieldName, SearchIndexSchema};

static SEARCH_STATE_MANAGER: Lazy<Arc<Mutex<SearchStateManager>>> = Lazy::new(|| {
    Arc::new(Mutex::new(SearchStateManager {
        alias_map: HashMap::new(),
    }))
});

const TRANSACTION_CALLBACK_CACHE_ID: &str = "parade_current_search";

pub struct SearchStateManager {
    alias_map: HashMap<SearchAlias, SearchState>,
}

impl SearchStateManager {
    fn register_callback() -> Result<(), TransactionError> {
        // Commit and abort are mutually exclusive. One of the two is guaranteed
        // to be called on any transaction. We'll use that opportunity to clean
        // up the cache.
        Transaction::call_once_on_commit(TRANSACTION_CALLBACK_CACHE_ID, move || {
            let mut current_search = SEARCH_STATE_MANAGER
                .lock()
                .expect("could not lock current search lookup in commit callback");
            current_search.alias_map.drain();
        })?;
        Transaction::call_once_on_abort(TRANSACTION_CALLBACK_CACHE_ID, move || {
            let mut current_search = SEARCH_STATE_MANAGER
                .lock()
                .expect("could not lock current search lookup in abort callback");
            current_search.alias_map.drain();
        })?;
        Ok(())
    }

    fn get_state_default(&self) -> Result<&SearchState, SearchStateError> {
        self.alias_map
            .get(&SearchAlias::default())
            .ok_or(SearchStateError::NoQuery)
    }

    fn get_state_alias(&self, alias: SearchAlias) -> Result<&SearchState, SearchStateError> {
        self.alias_map
            .get(&alias)
            .ok_or(SearchStateError::AliasLookup(alias))
    }

    pub fn get_score(
        id: i64,
        alias: Option<SearchAlias>,
    ) -> Result<SearchIndexScore, SearchStateError> {
        let manager = SEARCH_STATE_MANAGER
            .lock()
            .map_err(SearchStateError::from)?;
        let state = manager.get_state(alias)?;
        match state.lookup.get(&id) {
            Some((score, _)) => Ok(*score),
            None => Err(SearchStateError::DocLookup(id)),
        }
    }

    pub fn get_snippet(
        id: i64,
        field_name: &str,
        max_num_chars: Option<usize>,
        alias: Option<SearchAlias>,
    ) -> Result<Snippet, SearchStateError> {
        let manager = SEARCH_STATE_MANAGER
            .lock()
            .map_err(SearchStateError::from)?;
        let state = manager.get_state(alias)?;
        Ok(state.snippet(id, field_name, max_num_chars))
    }

    fn get_state(&self, alias: Option<SearchAlias>) -> Result<&SearchState, SearchStateError> {
        if let Some(alias) = alias {
            self.get_state_alias(alias)
        } else {
            self.get_state_default()
        }
    }

    fn set_state_default(&mut self, state: SearchState) -> Result<(), SearchStateError> {
        match self.alias_map.insert(SearchAlias::default(), state) {
            None => Ok(()),
            Some(_) => Err(SearchStateError::AliasRequired),
        }
    }

    fn set_state_alias(
        &mut self,
        state: SearchState,
        alias: SearchAlias,
    ) -> Result<(), SearchStateError> {
        if alias == SearchAlias::default() {
            Err(SearchStateError::EmptyAlias)
        } else {
            if self.alias_map.insert(alias.clone(), state).is_some() {
                return Err(SearchStateError::DuplicateAlias(alias));
            }
            Ok(())
        }
    }

    pub fn set_state(state: SearchState) -> Result<(), SearchStateError> {
        Self::register_callback().map_err(SearchStateError::from)?;

        let mut current_search = SEARCH_STATE_MANAGER
            .lock()
            .map_err(SearchStateError::from)?;
        if let Some(ref alias) = state.config.alias {
            let alias = SearchAlias(alias.clone());
            current_search.set_state_alias(state, alias)
        } else {
            current_search.set_state_default(state)
        }
    }
}

#[derive(Debug, Error)]
pub enum SearchStateError {
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
    #[error("could not register callback for search state manager: {0}")]
    CallbackError(#[from] TransactionError),
}

impl<T> From<PoisonError<T>> for SearchStateError {
    fn from(err: PoisonError<T>) -> Self {
        SearchStateError::Lock(format!("{err}"))
    }
}

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

pub struct SearchState {
    pub query: Box<dyn Query>,
    pub searcher: Searcher,
    pub config: SearchConfig,
    pub lookup: HashMap<i64, (SearchIndexScore, DocAddress)>,
    pub max_score: f32,
    pub min_score: f32,
    pub schema: SearchIndexSchema,
}

impl SearchState {
    pub fn new(search_index: &SearchIndex, config: &SearchConfig) -> Self {
        let schema = search_index.schema.clone();
        let mut parser = search_index.query_parser();
        let query = config
            .query
            .clone()
            .into_tantivy_query(&schema, &mut parser)
            .expect("could not parse query");
        SearchState {
            query,
            config: config.clone(),
            searcher: search_index.searcher(),
            lookup: HashMap::new(),
            schema: schema.clone(),
            max_score: f32::NEG_INFINITY,
            min_score: f32::INFINITY,
        }
    }

    pub fn snippet(&self, id: i64, field_name: &str, max_num_chars: Option<usize>) -> Snippet {
        let field = self
            .schema
            .get_search_field(&SearchFieldName(field_name.into()))
            .expect("cannot generate snippet, field does not exist");

        let mut snippet_generator = match self.schema.schema.get_field_entry(field.into()).field_type() {
            FieldType::Str(_) => {
                SnippetGenerator::create(&self.searcher, &self.query, field.into())
                    .unwrap_or_else(|err| panic!("failed to create snippet generator for field: {field_name}... {err}"))
            },
            _ => panic!("failed to create snippet generator for field: {field_name}... can only highlight text fields")
        };

        if let Some(max_num_chars) = max_num_chars {
            snippet_generator.set_max_num_chars(max_num_chars)
        }

        let (_, address) = self
            .lookup
            .get(&id)
            .unwrap_or_else(|| panic!("could not find a search result with id {id}"));
        let doc = self
            .searcher
            .doc(*address)
            .expect("could not find document in searcher");
        snippet_generator.snippet_from_doc(&doc)
    }

    /// Search the Tantivy index for matching documents. If used outside of Postgres
    /// index access methods, this may return deleted rows until a VACUUM. If you need to scan
    /// the Tantivy index without a Postgres deduplication, you should use the `search_dedup`
    /// method instead.
    pub fn search(&mut self) -> Vec<(SearchIndexScore, DocAddress)> {
        // Extract limit and offset from the query config or set defaults.
        let limit = self.config.limit_rows.unwrap_or_else(|| {
            // We use unwrap_or_else here so this block doesn't run unless
            // we actually need the default value. This is important, because there can
            // be some cost to Tantivy API calls.
            let num_docs = self.searcher.num_docs() as usize;
            if num_docs > 0 {
                num_docs // The collector will panic if it's passed a limit of 0.
            } else {
                1 // Since there's no docs to return anyways, just use 1.
            }
        });

        let offset = self.config.offset_rows.unwrap_or(0);
        let key_field_name = self.config.key_field.clone();
        let top_docs_by_custom_score = TopDocs::with_limit(limit).and_offset(offset).tweak_score(
            // tweak_score expects a function that will return a function. A little unusual for
            // Rust, but not too much of a problem as long as you don't need to reference
            // many variables outside the function scope.
            move |segment_reader: &SegmentReader| {
                let key_field_reader = segment_reader
                    .fast_fields()
                    .i64(&key_field_name)
                    .unwrap_or_else(|err| {
                        panic!("key field {} is not a i64: {err:?}", &key_field_name)
                    })
                    .first_or_default_col(0);

                let ctid_field_reader = segment_reader
                    .fast_fields()
                    .u64("ctid")
                    .unwrap_or_else(|err| panic!("ctid field is not a u64: {err:?}"))
                    .first_or_default_col(0);

                move |doc: DocId, original_score: Score| SearchIndexScore {
                    bm25: original_score,
                    key: key_field_reader.get_val(doc),
                    ctid: ctid_field_reader.get_val(doc),
                }
            },
        );

        let top_docs = self
            .searcher
            .search(&self.query, &top_docs_by_custom_score)
            .expect("failed to search");

        top_docs
            .into_iter()
            .map(|(score, address)| {
                // We store the results so they can be looked up by
                // rank + highlight functions.
                self.lookup.insert(score.key, (score, address));
                if score.bm25 > self.max_score {
                    self.max_score = score.bm25
                }
                if score.bm25 < self.min_score {
                    self.min_score = score.bm25
                }
                (score, address)
            })
            .collect()
    }

    /// A search method that deduplicates results based on key field. This is important for
    /// searches into the Tantivy index outside of Postgres index access methods. Postgres will
    /// filter out stale rows when using the index scan, but when scanning Tantivy directly,
    /// we risk returning deleted documents if a VACUUM hasn't been performed yet.
    pub fn search_dedup(&mut self) -> impl Iterator<Item = (SearchIndexScore, DocAddress)> {
        let search_results = self.search();
        let mut dedup_map: HashMap<i64, (SearchIndexScore, DocAddress)> = HashMap::new();
        let mut order_vec: Vec<i64> = Vec::new();

        for (score, doc_addr) in search_results {
            let key = score.key;
            let is_new_or_higher = match dedup_map.get(&key) {
                Some((_, existing_doc_addr)) => doc_addr > *existing_doc_addr,
                None => true,
            };
            if is_new_or_higher && dedup_map.insert(key, (score, doc_addr)).is_none() {
                // Key was not already present, remember the order of this key
                order_vec.push(key);
            }
        }

        order_vec
            .into_iter()
            .filter_map(move |key| dedup_map.remove(&key))
    }
}

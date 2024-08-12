// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use super::score::SearchIndexScore;
use super::SearchIndex;
use crate::postgres::types::TantivyValue;
use crate::schema::{SearchConfig, SearchFieldName, SearchFieldType, SearchIndexSchema};
use std::collections::HashMap;
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::schema::{FieldType, Value};
use tantivy::{query::Query, DocAddress, Score, Searcher};
use tantivy::{Executor, SnippetGenerator, TantivyDocument};

#[derive(Clone)]
pub struct SearchState {
    pub query: Arc<dyn Query>,
    pub searcher: Searcher,
    pub config: SearchConfig,
    pub schema: SearchIndexSchema,
}

impl SearchState {
    pub fn new(search_index: &SearchIndex, config: &SearchConfig) -> Self {
        let schema = search_index.schema.clone();
        let mut parser = search_index.query_parser();
        let searcher = search_index.searcher();
        let query = config
            .query
            .clone()
            .into_tantivy_query(&schema, &mut parser, &searcher, config)
            .expect("could not parse query");
        SearchState {
            query: Arc::new(query),
            config: config.clone(),
            searcher,
            schema: schema.clone(),
        }
    }

    pub fn snippet_generator(&self, field_name: &str) -> SnippetGenerator {
        let field = self
            .schema
            .get_search_field(&SearchFieldName(field_name.into()))
            .expect("cannot generate snippet, field does not exist");

        match self.schema.schema.get_field_entry(field.into()).field_type() {
            FieldType::Str(_) => {
                SnippetGenerator::create(&self.searcher, self.query.as_ref(), field.into())
                    .unwrap_or_else(|err| panic!("failed to create snippet generator for field: {field_name}... {err}"))
            },
            _ => panic!("failed to create snippet generator for field: {field_name}... can only highlight text fields")
        }
    }

    /// Search the Tantivy index for matching documents. If used outside of Postgres
    /// index access methods, this may return deleted rows until a VACUUM. If you need to scan
    /// the Tantivy index without a Postgres deduplication, you should use the `search_dedup`
    /// method instead.
    pub fn search(&self, executor: &Executor) -> Vec<(Score, DocAddress, TantivyValue, u64)> {
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

        if self.config.stable_sort.is_some_and(|stable| stable) {
            // If the user requires a stable sort, we'll use tweak_score. This allows us to retrieve
            // the value of a fast field and use that as a secondary sort key. In the case of a
            // bm25 score tie, results will be ordered based on the value of their 'key_field'.
            // This has a big performance impact, so the user needs to opt-in.
            let key_field_name = self.config.key_field.clone();
            let schema = self.schema.clone();
            let collector = TopDocs::with_limit(limit).and_offset(offset).tweak_score(
                move |segment_reader: &tantivy::SegmentReader| -> Box<dyn FnMut(tantivy::DocId, Score) -> SearchIndexScore> {
                    let fast_fields = segment_reader
                        .fast_fields();

                    // Check the type of the field from the schema
                    match schema.get_search_field(&key_field_name.clone().into()).unwrap_or_else(|| panic!("key field {} not found", key_field_name)).type_ {
                        SearchFieldType::I64 => {
                            let key_field_reader = fast_fields
                                .i64(&key_field_name)
                                .unwrap_or_else(|err| panic!("key field {} is not a i64: {err:?}", key_field_name))
                                .first_or_default_col(0);

                            Box::new(move |doc: tantivy::DocId, original_score: tantivy::Score| {
                                let val = key_field_reader.get_val(doc);
                                SearchIndexScore {
                                    bm25: original_score,
                                    key: TantivyValue(val.into()),
                                }
                            })
                        }
                        SearchFieldType::U64 => {
                            let key_field_reader = fast_fields
                                .u64(&key_field_name)
                                .unwrap_or_else(|err| panic!("key field {} is not a u64: {err:?}", key_field_name))
                                .first_or_default_col(0);

                            Box::new(move |doc: tantivy::DocId, original_score: tantivy::Score| {
                                SearchIndexScore {
                                    bm25: original_score,
                                    key: TantivyValue(key_field_reader.get_val(doc).into()),
                                }
                            })
                        }
                        SearchFieldType::F64 => {
                            let key_field_reader = fast_fields
                                .f64(&key_field_name)
                                .unwrap_or_else(|err| panic!("key field {} is not a f64: {err:?}", key_field_name))
                                .first_or_default_col(0.0);

                            Box::new(move |doc: tantivy::DocId, original_score: tantivy::Score| {
                                SearchIndexScore {
                                    bm25: original_score,
                                    key: TantivyValue(key_field_reader.get_val(doc).into()),
                                }
                            })
                        }
                        SearchFieldType::Text => {
                            let key_field_reader = fast_fields
                                .str(&key_field_name)
                                .unwrap_or_else(|err| panic!("key field {} is not a string: {err:?}", key_field_name))
                                .unwrap();

                            Box::new(move |doc: tantivy::DocId, original_score: tantivy::Score| {
                                let mut tok_str: String = Default::default();
                                let ord = key_field_reader.term_ords(doc).nth(0).unwrap();
                                key_field_reader.ord_to_str(ord, &mut tok_str).expect("no string!!");
                                SearchIndexScore {
                                    bm25: original_score,
                                    key: TantivyValue(tok_str.clone().into()),
                                }
                            })
                        }
                        SearchFieldType::Bool => {
                            let key_field_reader = fast_fields
                                .bool(&key_field_name)
                                .unwrap_or_else(|err| panic!("key field {} is not a bool: {err:?}", key_field_name))
                                .first_or_default_col(false);

                            Box::new(move |doc: tantivy::DocId, original_score: tantivy::Score| {
                                SearchIndexScore {
                                    bm25: original_score,
                                    key: TantivyValue(key_field_reader.get_val(doc).into()),
                                }
                            })
                        }
                        SearchFieldType::Date => {
                            let key_field_reader = fast_fields
                                .date(&key_field_name)
                                .unwrap_or_else(|err| panic!("key field {} is not a date: {err:?}", key_field_name))
                                .first_or_default_col(tantivy::DateTime::MIN);

                            Box::new(move |doc: tantivy::DocId, original_score: tantivy::Score| {
                                SearchIndexScore {
                                    bm25: original_score,
                                    key: TantivyValue(key_field_reader.get_val(doc).into()),
                                }
                            })
                        }
                        _ => panic!("key field {} is not a supported field type", key_field_name)
                    }
                },
            );
            self.searcher
                .search_with_executor(
                    self.query.as_ref(),
                    &collector,
                    executor,
                    tantivy::query::EnableScoring::Enabled {
                        searcher: &self.searcher,
                        statistics_provider: &self.searcher,
                    },
                )
                .expect("failed to search")
                .into_iter()
                .map(|(score, doc_address)| {
                    // This iterator contains the results after limit + offset are applied.
                    let ctid = self.ctid_value(doc_address);
                    (score.bm25, doc_address, score.key, ctid)
                })
                .collect()
        } else {
            let collector = TopDocs::with_limit(limit).and_offset(offset);
            self.searcher
                .search_with_executor(
                    self.query.as_ref(),
                    &collector,
                    executor,
                    tantivy::query::EnableScoring::Enabled {
                        searcher: &self.searcher,
                        statistics_provider: &self.searcher,
                    },
                )
                .expect("failed to search")
                .into_iter()
                .map(|(score, doc_address)| {
                    // This iterator contains the results after limit + offset are applied.
                    let (key, ctid) = self.key_and_ctid_value(doc_address);
                    (score, doc_address, key, ctid)
                })
                .collect()
        }
    }

    pub fn key_value(&self, doc_address: DocAddress) -> TantivyValue {
        let retrieved_doc: TantivyDocument = self
            .searcher
            .doc(doc_address)
            .expect("could not retrieve document by address");

        let value = retrieved_doc
            .get_first(self.schema.key_field().id.0)
            .unwrap();

        TantivyValue(value.clone())
    }

    pub fn ctid_value(&self, doc_address: DocAddress) -> u64 {
        let retrieved_doc: TantivyDocument = self
            .searcher
            .doc(doc_address)
            .expect("could not retrieve document by address");

        retrieved_doc
            .get_first(self.schema.ctid_field().id.0)
            .unwrap()
            .as_u64()
            .expect("could not access ctid field on document")
    }

    pub fn key_and_ctid_value(&self, doc_address: DocAddress) -> (TantivyValue, u64) {
        let retrieved_doc: TantivyDocument = self
            .searcher
            .doc(doc_address)
            .expect("could not retrieve document by address");

        let value = retrieved_doc
            .get_first(self.schema.key_field().id.0)
            .unwrap();

        let key = TantivyValue(value.clone());

        let ctid = retrieved_doc
            .get_first(self.schema.ctid_field().id.0)
            .unwrap()
            .as_u64()
            .expect("could not access ctid field on document");
        (key, ctid)
    }

    /// A search method that deduplicates results based on key field. This is important for
    /// searches into the Tantivy index outside of Postgres index access methods. Postgres will
    /// filter out stale rows when using the index scan, but when scanning Tantivy directly,
    /// we risk returning deleted documents if a VACUUM hasn't been performed yet.
    pub fn search_dedup(
        &mut self,
        executor: &Executor,
    ) -> impl Iterator<Item = (Score, DocAddress)> {
        let search_results = self.search(executor);
        let mut dedup_map: HashMap<TantivyValue, (Score, DocAddress)> = HashMap::new();
        let mut order_vec: Vec<TantivyValue> = Vec::new();

        for (score, doc_addr, key, _) in search_results {
            let is_new_or_higher = match dedup_map.get(&key) {
                Some((_, existing_doc_addr)) => doc_addr > *existing_doc_addr,
                None => true,
            };
            if is_new_or_higher && dedup_map.insert(key.clone(), (score, doc_addr)).is_none() {
                // Key was not already present, remember the order of this key
                order_vec.push(key.clone());
            }
        }

        order_vec
            .into_iter()
            .filter_map(move |key| dedup_map.remove(&key))
    }
}

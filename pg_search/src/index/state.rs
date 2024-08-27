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

use super::score::{OrderByScore, SearchIndexScore};
use super::SearchIndex;
use crate::postgres::types::TantivyValue;
use crate::schema::{SearchConfig, SearchFieldName, SearchIndexSchema};
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::columnar::{ColumnValues, StrColumn};
use tantivy::fastfield::FastFieldReaders;
use tantivy::schema::FieldType;
<<<<<<< HEAD
use tantivy::{query::Query, DocAddress, Score, Searcher};
use tantivy::{Executor, Order, SnippetGenerator};

static DEFAULT_ORDER_BY_DIRECTION: &str = "asc";
static DEFAULT_BM25_SCORE_FOR_ORDER_BY: f32 = 1.0;
=======
use tantivy::{query::Query, DocAddress, DocId, Score, Searcher};
use tantivy::{Executor, SnippetGenerator};
>>>>>>> dev

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
            }
            _ => panic!("failed to create snippet generator for field: {field_name}... can only highlight text fields")
        }
    }

    /// Search the Tantivy index for matching documents. If used outside of Postgres
    /// index access methods, this may return deleted rows until a VACUUM. If you need to scan
    /// the Tantivy index without a Postgres deduplication, you should use the `search_dedup`
    /// method instead.
    pub fn search(
        &self,
        executor: &Executor,
    ) -> std::vec::IntoIter<(SearchIndexScore, DocAddress)> {
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
<<<<<<< HEAD
        let schema = self.schema.clone();

        if let Some(order_by_field) = self.config.order_by_field.clone() {
            // Lowercase the input and use default order direction
            let lowercase_direction_input = self
                .config
                .order_by_direction
                .clone()
                .unwrap_or(DEFAULT_ORDER_BY_DIRECTION.to_owned())
                .to_lowercase();

            let collector = TopDocs::with_limit(limit).and_offset(offset).custom_score(
                move |segment_reader: &tantivy::SegmentReader| -> Box<dyn FnMut(tantivy::DocId) -> OrderByScore> {
                    // Convert to tantivy Order
                    let direction = match lowercase_direction_input.as_str() {
                        "asc" => Order::Asc,
                        "desc" => Order::Desc,
                        _ => panic!("Invalid order_by_direction {}", lowercase_direction_input),
                    };

                    let fast_fields = segment_reader
                            .fast_fields();

                    let ctid_field_reader = fast_fields.u64("ctid").unwrap_or_else(|err| panic!("no u64 ctid field: {err:?}" )).first_or_default_col(0);
                    let order_field_reader = fast_fields.u64_lenient(&order_by_field).unwrap_or_else(|err| panic!("can't read field {}: {:?} ", order_by_field, err)).unwrap().0.first_or_default_col(0);

                    // Check the type of the field from the schema
                    match schema.get_search_field(&key_field_name.clone().into()).unwrap_or_else(|| panic!("key field {} not found", key_field_name)).type_ {
                        SearchFieldType::I64 => {
                            let key_field_reader = fast_fields
                                .i64(&key_field_name)
                                .unwrap_or_else(|err| panic!("key field {} is not a i64: {err:?}", key_field_name))
                                .first_or_default_col(0);

                            Box::new(move |doc: tantivy::DocId| {
                                let val = key_field_reader.get_val(doc);

                                let score = match direction {
                                    Order::Asc => u64::MAX - order_field_reader.get_val(doc),
                                    Order::Desc => order_field_reader.get_val(doc),
                                };

                                OrderByScore {
                                    score,
                                    key: TantivyValue(val.into()),
                                    ctid: ctid_field_reader.get_val(doc)
                                }
                            })
                        }
                        SearchFieldType::U64 => {
                            let key_field_reader = fast_fields
                                .u64(&key_field_name)
                                .unwrap_or_else(|err| panic!("key field {} is not a u64: {err:?}", key_field_name))
                                .first_or_default_col(0);

                            Box::new(move |doc: tantivy::DocId| {
                                let score = match direction {
                                    Order::Asc => order_field_reader.get_val(doc),
                                    Order::Desc => u64::MAX - order_field_reader.get_val(doc),
                                };

                                OrderByScore {
                                    score,
                                    key: TantivyValue(key_field_reader.get_val(doc).into()),
                                    ctid: ctid_field_reader.get_val(doc)
                                }
                            })
                        }
                        SearchFieldType::F64 => {
                            let key_field_reader = fast_fields
                                .f64(&key_field_name)
                                .unwrap_or_else(|err| panic!("key field {} is not a f64: {err:?}", key_field_name))
                                .first_or_default_col(0.0);

                            Box::new(move |doc: tantivy::DocId| {
                                let score = match direction {
                                    Order::Asc => order_field_reader.get_val(doc),
                                    Order::Desc => u64::MAX - order_field_reader.get_val(doc),
                                };

                                OrderByScore {
                                    score,
                                    key: TantivyValue(key_field_reader.get_val(doc).into()),
                                    ctid: ctid_field_reader.get_val(doc)
                                }
                            })
                        }
                        SearchFieldType::Text => {
                            let key_field_reader = fast_fields
                                .str(&key_field_name)
                                .unwrap_or_else(|err| panic!("key field {} is not a string: {err:?}", key_field_name))
                                .unwrap();

                            Box::new(move |doc: tantivy::DocId| {
                                let mut tok_str: String = Default::default();
                                let ord = key_field_reader.term_ords(doc).nth(0).unwrap();
                                key_field_reader.ord_to_str(ord, &mut tok_str).expect("no string!!");

                                let score = match direction {
                                    Order::Asc => order_field_reader.get_val(doc),
                                    Order::Desc => u64::MAX - order_field_reader.get_val(doc),
                                };

                                OrderByScore {
                                    score,
                                    key: TantivyValue(tok_str.clone().into()),
                                    ctid: ctid_field_reader.get_val(doc)
                                }
                            })
                        }
                        SearchFieldType::Bool => {
                            let key_field_reader = fast_fields
                                .bool(&key_field_name)
                                .unwrap_or_else(|err| panic!("key field {} is not a bool: {err:?}", key_field_name))
                                .first_or_default_col(false);

                            Box::new(move |doc: tantivy::DocId| {
                                let score = match direction {
                                    Order::Asc => order_field_reader.get_val(doc),
                                    Order::Desc => u64::MAX - order_field_reader.get_val(doc),
                                };

                                OrderByScore {
                                    score,
                                    key: TantivyValue(key_field_reader.get_val(doc).into()),
                                    ctid: ctid_field_reader.get_val(doc)
                                }
                            })
                        }
                        SearchFieldType::Date => {
                            let key_field_reader = fast_fields
                                .date(&key_field_name)
                                .unwrap_or_else(|err| panic!("key field {} is not a date: {err:?}", key_field_name))
                                .first_or_default_col(tantivy::DateTime::MIN);

                            Box::new(move |doc: tantivy::DocId| {
                                let score = match direction {
                                    Order::Asc => order_field_reader.get_val(doc),
                                    Order::Desc => u64::MAX - order_field_reader.get_val(doc),
                                };

                                OrderByScore {
                                    score,
                                    key: TantivyValue(key_field_reader.get_val(doc).into()),
                                    ctid: ctid_field_reader.get_val(doc)
                                }
                            })
                        }
                        _ => panic!("key field {} is not a supported field type", key_field_name)
                    }
                }
            );

            return self
                .searcher
                .search_with_executor(
                    self.query.as_ref(),
                    &collector,
                    executor,
                    // Disable scoring for performance
                    tantivy::query::EnableScoring::Disabled {
                        searcher_opt: Some(&self.searcher),
                        schema: &self.schema.schema,
                    },
                )
                .expect("failed to search")
                .into_iter()
                .map(|(score, doc_address)| {
                    (
                        DEFAULT_BM25_SCORE_FOR_ORDER_BY,
                        doc_address,
                        score.key,
                        score.ctid,
                    )
                })
                .collect();
        }

=======
>>>>>>> dev
        let collector = TopDocs::with_limit(limit).and_offset(offset).tweak_score(
            move |segment_reader: &tantivy::SegmentReader| {
                let fast_fields = segment_reader.fast_fields();
                let ctid_ff = FFType::new(fast_fields, "ctid");
                let key_ff = FFType::new(fast_fields, key_field_name.as_str());

                move |doc: DocId, original_score: Score| SearchIndexScore {
                    bm25: original_score,
                    key: key_ff.value(doc),
                    ctid: ctid_ff
                        .as_u64(doc)
                        .expect("expected the `ctid` field to be a u64"),
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
    }
}

/// Helper for working with different "fast field" types as if they're all one type
enum FFType {
    Text(StrColumn),
    I64(Arc<dyn ColumnValues<i64>>),
    F64(Arc<dyn ColumnValues<f64>>),
    U64(Arc<dyn ColumnValues<u64>>),
    Bool(Arc<dyn ColumnValues<bool>>),
    Date(Arc<dyn ColumnValues<tantivy::DateTime>>),
}

impl FFType {
    /// Construct the proper [`FFType`] for the specified `field_name`, which
    /// should be a known field name in the Tantivy index
    fn new(ffr: &FastFieldReaders, field_name: &str) -> Self {
        if let Ok(Some(ff)) = ffr.str(field_name) {
            Self::Text(ff)
        } else if let Ok(ff) = ffr.u64(field_name) {
            Self::U64(ff.first_or_default_col(0))
        } else if let Ok(ff) = ffr.i64(field_name) {
            Self::I64(ff.first_or_default_col(0))
        } else if let Ok(ff) = ffr.f64(field_name) {
            Self::F64(ff.first_or_default_col(0.0))
        } else if let Ok(ff) = ffr.bool(field_name) {
            Self::Bool(ff.first_or_default_col(false))
        } else if let Ok(ff) = ffr.date(field_name) {
            Self::Date(ff.first_or_default_col(tantivy::DateTime::MIN))
        } else {
            panic!("`{field_name}` is missing or is not configured as a fast field")
        }
    }

    /// Given a [`DocId`], what is its "fast field" value?
    #[inline(always)]
    fn value(&self, doc: DocId) -> TantivyValue {
        let value = match self {
            FFType::Text(ff) => {
                let mut s = String::new();
                let ord = ff.term_ords(doc).next().unwrap();
                ff.ord_to_str(ord, &mut s).expect("no string for term ord");
                TantivyValue(s.into())
            }
            FFType::I64(ff) => TantivyValue(ff.get_val(doc).into()),
            FFType::F64(ff) => TantivyValue(ff.get_val(doc).into()),
            FFType::U64(ff) => TantivyValue(ff.get_val(doc).into()),
            FFType::Bool(ff) => TantivyValue(ff.get_val(doc).into()),
            FFType::Date(ff) => TantivyValue(ff.get_val(doc).into()),
        };

        value
    }

    /// Given a [`DocId`], what is its "fast field" value?  In the case of a String field, we
    /// don't reconstruct the full string, and instead return the term ord as a u64
    #[inline(always)]
    #[allow(dead_code)]
    fn value_fast(&self, doc: DocId) -> TantivyValue {
        let value = match self {
            FFType::Text(ff) => {
                // just use the first term ord here.  that's enough to do a tie-break quickly
                let ord = ff.term_ords(doc).next().unwrap();
                TantivyValue(ord.into())
            }
            other => other.value(doc),
        };

        value
    }

    /// Given a [`DocId`], what is its u64 "fast field" value?
    ///
    /// If this [`FFType`] isn't [`FFType::U64`], this function returns [`None`].
    #[inline(always)]
    fn as_u64(&self, doc: DocId) -> Option<u64> {
        if let FFType::U64(ff) = self {
            Some(ff.get_val(doc))
        } else {
            None
        }
    }
}

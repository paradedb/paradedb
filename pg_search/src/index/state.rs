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

use super::SearchIndex;
use crate::postgres::types::TantivyValue;
use crate::schema::{SearchConfig, SearchFieldName, SearchIndexSchema};
use rustc_hash::FxHashMap;
use std::sync::Arc;
use tantivy::collector::{DocSetCollector, TopDocs};
use tantivy::columnar::{ColumnValues, StrColumn};
use tantivy::schema::FieldType;
use tantivy::{query::Query, DocAddress, Searcher, SegmentOrdinal};
use tantivy::{Executor, SnippetGenerator};

#[derive(Clone)]
struct BothColumnValues {
    ctids: Arc<dyn ColumnValues>,
    keys_i64: Option<Arc<dyn ColumnValues<i64>>>,
    keys_u64: Option<Arc<dyn ColumnValues<u64>>>,
    keys_f64: Option<Arc<dyn ColumnValues<f64>>>,
    keys_bool: Option<Arc<dyn ColumnValues<bool>>>,
    keys_date: Option<Arc<dyn ColumnValues<tantivy::DateTime>>>,
    keys_str: Option<StrColumn>,
}
#[derive(Clone)]
pub struct SearchState {
    pub query: Arc<dyn Query>,
    pub searcher: Searcher,
    pub config: SearchConfig,
    pub schema: SearchIndexSchema,
}

pub struct SearchHit {
    pub score: f32,
    pub doc_address: DocAddress,
    pub ctid: u64,
    pub key: Option<TantivyValue>,
}

pub type SearchHitIter<'a> = Box<dyn Iterator<Item = SearchHit> + 'a>;

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

    pub fn searcher(&self) -> Searcher {
        self.searcher.clone()
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
    pub fn search<'a>(&'a self, executor: &'a Executor) -> SearchHitIter<'a> {
        match (self.config.offset_rows, self.config.limit_rows) {
            (Some(offset), Some(limit)) => {
                Box::new(self.search_with_offset_limit(executor, offset, limit))
            }
            (Some(offset), None) => Box::new(self.search_with_offset_limit(executor, offset, 0)),
            (None, Some(limit)) => Box::new(self.search_with_offset_limit(executor, 0, limit)),
            (None, None) => Box::new(self.search_fast(executor)),
        }
    }

    fn search_with_offset_limit<'a>(
        &'a self,
        executor: &'a Executor,
        offset: usize,
        limit: usize,
    ) -> impl Iterator<Item = SearchHit> + 'a {
        let limit = limit.max(1); // tantivy's Collector will panic if the limit is zero

        let collector = TopDocs::with_limit(limit).and_offset(offset);
        let mut ff_cache: FxHashMap<SegmentOrdinal, Arc<dyn ColumnValues<u64>>> =
            FxHashMap::default();
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
            .map(move |(score, doc_address)| SearchHit {
                score,
                doc_address,
                ctid: SearchState::lookup_ctid(&mut ff_cache, &self.searcher, doc_address),
                key: None,
            })
    }

    fn search_fast<'a>(&'a self, executor: &'a Executor) -> impl Iterator<Item = SearchHit> + 'a {
        let collector = DocSetCollector;

        let mut ff_cache: FxHashMap<SegmentOrdinal, Arc<dyn ColumnValues<u64>>> =
            FxHashMap::default();
        self.searcher
            .search_with_executor(
                self.query.as_ref(),
                &collector,
                executor,
                tantivy::query::EnableScoring::Disabled {
                    schema: &self.schema.schema,
                    searcher_opt: Some(&self.searcher),
                },
            )
            .expect("failed to search")
            .into_iter()
            .map(move |doc_address| SearchHit {
                score: 0.0,
                doc_address,
                ctid: SearchState::lookup_ctid(&mut ff_cache, &self.searcher, doc_address),
                key: None,
            })
    }

    fn lookup_ctid(
        cache: &mut FxHashMap<SegmentOrdinal, Arc<dyn ColumnValues<u64>>>,
        searcher: &Searcher,
        doc_address: DocAddress,
    ) -> u64 {
        let ff = cache.entry(doc_address.segment_ord).or_insert_with(|| {
            searcher
                .segment_reader(doc_address.segment_ord)
                .fast_fields()
                .u64("ctid")
                .unwrap()
                .first_or_default_col(0)
        });
        ff.get_val(doc_address.doc_id)
    }

    // /// Search the Tantivy index for matching documents. If used outside of Postgres
    // /// index access methods, this may return deleted rows until a VACUUM. If you need to scan
    // /// the Tantivy index without a Postgres deduplication, you should use the `search_dedup`
    // /// method instead.
    // pub fn search_including_key<'a>(
    //     &'a mut self,
    //     executor: &'a Executor,
    // ) -> impl Iterator<Item = (Score, DocAddress, TantivyValue, u64)> + 'a {
    //     // Extract limit and offset from the query config or set defaults.
    //     let limit = self.config.limit_rows.unwrap_or_else(|| {
    //         // We use unwrap_or_else here so this block doesn't run unless
    //         // we actually need the default value. This is important, because there can
    //         // be some cost to Tantivy API calls.
    //         let num_docs = self.searcher.num_docs() as usize;
    //         if num_docs > 0 {
    //             num_docs // The collector will panic if it's passed a limit of 0.
    //         } else {
    //             1 // Since there's no docs to return anyways, just use 1.
    //         }
    //     });
    //
    //     let offset = self.config.offset_rows.unwrap_or(0);
    //     let key_field_name = self.config.key_field.clone();
    //     let key_field_type = self
    //         .schema
    //         .get_search_field(&key_field_name.clone().into())
    //         .unwrap_or_else(|| panic!("key field {} not found", key_field_name))
    //         .type_;
    //     let collector = TopDocs::with_limit(limit)
    //         .and_offset(offset)
    //         .tweak_score(move |segment_reader: &SegmentReader| {});
    //
    //     self.searcher
    //         .search_with_executor(
    //             self.query.as_ref(),
    //             &collector,
    //             executor,
    //             tantivy::query::EnableScoring::Enabled {
    //                 searcher: &self.searcher,
    //                 statistics_provider: &self.searcher,
    //             },
    //         )
    //         .expect("failed to search")
    //         .into_iter()
    //         .map(move |(score, doc_address)| {
    //             self.finalize_collection(&key_field_name, key_field_type, score, doc_address)
    //         })
    // }
    //
    // fn finalize_collection(
    //     &mut self,
    //     key_field_name: &str,
    //     key_field_type: SearchFieldType,
    //     score: Score,
    //     doc_address: DocAddress,
    // ) -> (Score, DocAddress, TantivyValue, u64) {
    //     let ff = self
    //         .ff_values
    //         .entry(doc_address.segment_ord)
    //         .or_insert_with(|| {
    //             let ff = self
    //                 .searcher
    //                 .segment_reader(doc_address.segment_ord)
    //                 .fast_fields();
    //             let ctids = ff.column_first_or_default("ctid".into()).unwrap();
    //
    //             BothColumnValues {
    //                 ctids,
    //                 keys_i64: matches!(key_field_type, SearchFieldType::I64)
    //                     .then(|| ff.i64(key_field_name).unwrap().first_or_default_col(0)),
    //                 keys_u64: matches!(key_field_type, SearchFieldType::U64)
    //                     .then(|| ff.u64(key_field_name).unwrap().first_or_default_col(0)),
    //                 keys_f64: matches!(key_field_type, SearchFieldType::F64)
    //                     .then(|| ff.f64(key_field_name).unwrap().first_or_default_col(0.0)),
    //                 keys_bool: matches!(key_field_type, SearchFieldType::Bool)
    //                     .then(|| ff.bool(key_field_name).unwrap().first_or_default_col(false)),
    //                 keys_date: matches!(key_field_type, SearchFieldType::Date).then(|| {
    //                     ff.date(key_field_name)
    //                         .unwrap()
    //                         .first_or_default_col(tantivy::DateTime::MIN)
    //                 }),
    //                 keys_str: matches!(key_field_type, SearchFieldType::Text)
    //                     .then(|| ff.str(key_field_name).unwrap())
    //                     .flatten(),
    //             }
    //         });
    //
    //     let key = match key_field_type {
    //         SearchFieldType::I64 => TantivyValue(
    //             ff.keys_i64
    //                 .as_ref()
    //                 .unwrap()
    //                 .get_val(doc_address.doc_id)
    //                 .into(),
    //         ),
    //         SearchFieldType::U64 => TantivyValue(
    //             ff.keys_u64
    //                 .as_ref()
    //                 .unwrap()
    //                 .get_val(doc_address.doc_id)
    //                 .into(),
    //         ),
    //         SearchFieldType::F64 => TantivyValue(
    //             ff.keys_f64
    //                 .as_ref()
    //                 .unwrap()
    //                 .get_val(doc_address.doc_id)
    //                 .into(),
    //         ),
    //         SearchFieldType::Bool => TantivyValue(
    //             ff.keys_bool
    //                 .as_ref()
    //                 .unwrap()
    //                 .get_val(doc_address.doc_id)
    //                 .into(),
    //         ),
    //         SearchFieldType::Date => TantivyValue(
    //             ff.keys_date
    //                 .as_ref()
    //                 .unwrap()
    //                 .get_val(doc_address.doc_id)
    //                 .into(),
    //         ),
    //         SearchFieldType::Text => {
    //             let col = ff.keys_str.as_ref().unwrap();
    //             let ord = col.term_ords(doc_address.doc_id).nth(0).unwrap();
    //
    //             let mut key = String::new();
    //             col.ord_to_str(ord, &mut key).expect("no string!!");
    //             TantivyValue(key.into())
    //         }
    //
    //         _ => panic!(
    //             "key field `{}` is not a supported field type",
    //             key_field_name
    //         ),
    //     };
    //
    //     let ctid = ff.ctids.get_val(doc_address.doc_id);
    //     (score, doc_address, key, ctid)
    // }
}

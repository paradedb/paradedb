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
use std::cmp::Ordering;
use std::sync::Arc;
use tantivy::collector::{DocSetCollector, TopDocs};
use tantivy::columnar::{ColumnValues, StrColumn};
use tantivy::schema::FieldType;
use tantivy::{query::Query, DocAddress, DocId, Score, Searcher, SegmentOrdinal, SegmentReader};
use tantivy::{Executor, SnippetGenerator};

#[derive(Clone)]
pub struct SearchState {
    pub query: Arc<dyn Query>,
    pub searcher: Searcher,
    pub config: SearchConfig,
    pub schema: SearchIndexSchema,
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub score: f32,
    pub ctid: u64,
    pub doc_address: Option<DocAddress>,
    pub key: Option<TantivyValue>,
}

impl Eq for SearchHit {}

impl PartialEq<Self> for SearchHit {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score && self.key == other.key
    }
}

impl PartialOrd for SearchHit {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.score.partial_cmp(&other.score) {
            Some(Ordering::Equal) => self.key.partial_cmp(&other.key),
            ne => ne,
        }
    }
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

    /// Search the Tantivy index for matching documents.
    ///
    /// If there's no `offset` or `limit` applied to the query and `stable_sort` is not `true`, this
    /// function will return documents in tantivy's [`DocId`] order, performing no scoring.  This is
    /// a "fast path" for when scores are not required by the caller.
    ///
    /// If there are either (or both), the documents are then returned in order of their BM25 score,
    /// descending.
    ///
    /// If the backing [`SearchConfig`] has `stable_sort` set, then key values will be retrieved,
    /// used for tie-breaking scores, and included in each [`SearchHit`] of the returned Iterator.
    ///
    /// If not, then key value lookups are elided, no tie-breaking occurs, and the resulting
    /// [`SearchHit`] emitted by the returned Iterator will **not** have its `key` field set.
    ///
    /// # Note
    ///
    /// This function has no understanding of Postgres MVCC visibility rules.  As such, any document
    /// returned may not actually exist in the backing Postgres table anymore.  It is the caller's
    /// responsibility to perform visibility checks where required.
    pub fn search<'a>(&'a self, executor: &'a Executor) -> SearchHitIter<'a> {
        let do_tiebreak = self.config.stable_sort.unwrap_or(false);
        match (self.config.offset_rows, self.config.limit_rows) {
            (None, None) if do_tiebreak == false => Box::new(self.search_fast(executor)),
            (None, None) if do_tiebreak == true => {
                self.search_with_offset_limit(executor, 0, None, true)
            }
            (Some(offset), limit) => {
                self.search_with_offset_limit(executor, offset, limit, do_tiebreak)
            }
            (None, limit) => self.search_with_offset_limit(executor, 0, limit, do_tiebreak),
        }
    }

    /// Search the Tantivy index for matching documents.
    ///
    /// This function will return documents in the calculated BM25 [`Score`] order, with a
    /// tie-break using the index's configured "key" field.
    ///
    /// Unlike [`SearchState::search`], this function has no fast path as it assumes scored documents
    /// are always wanted.
    ///
    /// Additionally, the resulting [`SearchHit`] emitted by the returned Iterator **will** have its
    /// `key` field set.
    ///
    /// # Note
    ///
    /// This function has no understanding of Postgres MVCC visibility rules.  As such, any document
    /// returned may not actually exist in the backing Postgres table anymore.  It is the caller's
    /// responsibility to perform visibility checks where required.
    pub fn search_with_scores<'a>(&'a self, executor: &'a Executor) -> SearchHitIter<'a> {
        self.search_with_offset_limit(
            executor,
            self.config.offset_rows.unwrap_or(0),
            self.config.limit_rows,
            true,
        )
    }

    fn search_with_offset_limit<'a>(
        &'a self,
        executor: &'a Executor,
        offset: usize,
        limit: Option<usize>,
        do_tiebreak: bool,
    ) -> Box<dyn Iterator<Item = SearchHit> + 'a> {
        let have_limit = limit.is_some();
        let limit = limit
            .unwrap_or_else(|| {
                // defer looking up num_docs as it might be slow
                self.searcher
                    .num_docs()
                    .try_into()
                    .expect("`num_docs` overflowed `usize`")
            })
            .max(1); // tantivy's Collector will panic if the limit is zero

        let key_field = self.schema.key_field();
        let key_field_name = key_field.name.0;

        if have_limit {
            // we have a limit, and we're assuming it's a small value
            // as such, we'll do the key resolution only for the final set
            // of scored docs

            let mut ff_ctid = Default::default();
            let mut ff_key = Default::default();
            let collector = TopDocs::with_limit(limit).and_offset(offset);
            let hits = self
                .searcher
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
                .map(move |(score, doc_address)| {
                    let searcher = &self.searcher;
                    let ctid = FFType::lookup(&mut ff_ctid, searcher, doc_address, "ctid")
                        .as_u64(doc_address.doc_id);
                    let key = do_tiebreak.then(|| {
                        FFType::lookup(&mut ff_key, searcher, doc_address, &key_field_name)
                            .value(doc_address.doc_id)
                    });

                    SearchHit {
                        score,
                        ctid: ctid.expect("`ctid`' must be a `u64`"),
                        doc_address: Some(doc_address),
                        key,
                    }
                });

            Box::new(hits)
        } else {
            // we have no limit.  as such it's beneficial to do the key resolution
            // in the collector itself, because if there's multiple segments, we'll be
            // processing them in parallel, amortizing the fast field lookup cost

            let collector = TopDocs::with_limit(limit).and_offset(offset).tweak_score(
                move |segment_reader: &SegmentReader| {
                    let ff_ctid = FFType::from((segment_reader, "ctid"));
                    let ff_key = FFType::from((segment_reader, key_field_name.as_ref()));

                    move |doc: DocId, score: Score| {
                        let ctid = ff_ctid.as_u64(doc);
                        let key = do_tiebreak.then(|| ff_key.value(doc));

                        SearchHit {
                            score,
                            ctid: ctid.expect("`ctid`' must be a `u64`"),
                            doc_address: None,
                            key,
                        }
                    }
                },
            );

            let hits = self
                .searcher
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
                .map(move |(mut search_hit, doc_address)| {
                    search_hit.doc_address = Some(doc_address);
                    search_hit
                });

            Box::new(hits)
        }
    }

    fn search_fast<'a>(&'a self, executor: &'a Executor) -> impl Iterator<Item = SearchHit> + 'a {
        let mut ff_ctid = Default::default();

        self.searcher
            .search_with_executor(
                self.query.as_ref(),
                &DocSetCollector,
                executor,
                tantivy::query::EnableScoring::Disabled {
                    schema: &self.schema.schema,
                    searcher_opt: Some(&self.searcher),
                },
            )
            .expect("failed to search")
            .into_iter()
            .map(move |doc_address| {
                let ctid = FFType::lookup(&mut ff_ctid, &self.searcher, doc_address, "ctid")
                    .as_u64(doc_address.doc_id);

                SearchHit {
                    score: 0.0,
                    doc_address: Some(doc_address),
                    ctid: ctid.expect("`ctid`' must be a `u64`"),
                    key: None,
                }
            })
    }
}

/// helper for dealing with different "fast field" types
enum FFType {
    Text(StrColumn),
    I64(Arc<dyn ColumnValues<i64>>),
    F64(Arc<dyn ColumnValues<f64>>),
    U64(Arc<dyn ColumnValues<u64>>),
    Bool(Arc<dyn ColumnValues<bool>>),
    Date(Arc<dyn ColumnValues<tantivy::DateTime>>),
}

impl From<(&SegmentReader, &str)> for FFType {
    /// Construct the proper fast field type ([`FFType`]) for the [`&str`] field, which
    /// should be a known field name in the Tantivy index
    fn from(value: (&SegmentReader, &str)) -> Self {
        let (segment_reader, field_name) = value;

        let ff = segment_reader.fast_fields();
        if let Ok(Some(ff)) = ff.str(field_name) {
            Self::Text(ff)
        } else if let Ok(ff) = ff.u64(field_name) {
            Self::U64(ff.first_or_default_col(0))
        } else if let Ok(ff) = ff.i64(field_name) {
            Self::I64(ff.first_or_default_col(0))
        } else if let Ok(ff) = ff.f64(field_name) {
            Self::F64(ff.first_or_default_col(0.0))
        } else if let Ok(ff) = ff.bool(field_name) {
            Self::Bool(ff.first_or_default_col(false))
        } else if let Ok(ff) = ff.date(field_name) {
            Self::Date(ff.first_or_default_col(tantivy::DateTime::MIN))
        } else {
            panic!("`{field_name}` is missing or is not configured as a fast field")
        }
    }
}

impl FFType {
    #[inline]
    fn lookup<'a>(
        cache: &'a mut FxHashMap<SegmentOrdinal, FFType>,
        searcher: &Searcher,
        doc_address: DocAddress,
        field_name: &str,
    ) -> &'a FFType {
        cache.entry(doc_address.segment_ord).or_insert_with(|| {
            FFType::from((searcher.segment_reader(doc_address.segment_ord), field_name))
        })
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

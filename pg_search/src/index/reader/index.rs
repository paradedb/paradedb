// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::api::FieldName;
use crate::api::HashMap;
use crate::index::fast_fields_helper::FFType;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::scorer::{DeferredScorer, ScorerIter};
use crate::index::setup_tokenizers;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::buffer::PinnedBuffer;
use crate::postgres::storage::metadata::MetaPage;
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;
use anyhow::Result;
use std::cmp::Ordering;
use std::fmt::{Debug, Display};
use std::path::PathBuf;
use std::sync::Arc;
use tantivy::collector::{Collector, TopDocs};
use tantivy::index::{Index, SegmentId};
use tantivy::query::{EnableScoring, QueryClone, QueryParser, Weight};
use tantivy::snippet::SnippetGenerator;
use tantivy::{
    query::Query, DocAddress, DocId, DocSet, Executor, IndexReader, Order, ReloadPolicy, Score,
    Searcher, SegmentOrdinal, SegmentReader, TantivyDocument,
};

/// Represents a matching document from a tantivy search.  Typically, it is returned as an Iterator
/// Item alongside the originating tantivy [`DocAddress`]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchIndexScore {
    pub ctid: u64,
    pub bm25: f32,
}

impl SearchIndexScore {
    #[inline]
    pub fn new(ctid: u64, score: Score) -> Self {
        Self { ctid, bm25: score }
    }
}

impl PartialOrd for SearchIndexScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.bm25.partial_cmp(&other.bm25)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SortDirection {
    Asc,
    Desc,
    None,
}

impl From<SortDirection> for Order {
    fn from(value: SortDirection) -> Self {
        match value {
            SortDirection::Asc => Order::Asc,
            SortDirection::Desc => Order::Desc,
            SortDirection::None => Order::Asc,
        }
    }
}

pub type FastFieldCache = HashMap<SegmentOrdinal, FFType>;

/// A known-size iterator of results for Top-N.
pub struct TopNSearchResults {
    results_original_len: usize,
    results: std::vec::IntoIter<(SearchIndexScore, DocAddress)>,
}

impl TopNSearchResults {
    pub fn empty() -> Self {
        Self::new(vec![])
    }

    fn new(results: Vec<(SearchIndexScore, DocAddress)>) -> Self {
        Self {
            results_original_len: results.len(),
            results: results.into_iter(),
        }
    }

    fn new_for_score(
        searcher: &Searcher,
        results: impl Iterator<Item = (Score, DocAddress)>,
    ) -> Self {
        // TODO: Execute batch lookups for ctids?
        let mut ff_lookup = FastFieldCache::default();
        Self::new(
            results
                .map(|(score, doc_address)| {
                    let ctid = ff_lookup
                        .entry(doc_address.segment_ord)
                        .or_insert_with(|| {
                            FFType::new_ctid(
                                searcher
                                    .segment_reader(doc_address.segment_ord)
                                    .fast_fields(),
                            )
                        })
                        .as_u64(doc_address.doc_id)
                        .expect("ctid should be present");

                    let scored = SearchIndexScore { ctid, bm25: score };
                    (scored, doc_address)
                })
                .collect(),
        )
    }

    pub fn original_len(&self) -> usize {
        self.results_original_len
    }
}

/// A set of search results across multiple segments.
///
/// May be consumed via `Iterator`, or directly via its methods in a segment-aware fashion.
pub struct MultiSegmentSearchResults {
    searcher: Searcher,
    ctid_column: Option<FFType>,
    iterators: Vec<ScorerIter>,
}

#[derive(PartialEq, Clone)]
pub struct TweakedScore {
    dir: SortDirection,
    score: Score,
}

impl PartialOrd for TweakedScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let cmp = self.score.partial_cmp(&other.score);
        match self.dir {
            SortDirection::Desc => cmp,
            SortDirection::Asc => cmp.map(|o| o.reverse()),
            SortDirection::None => Some(Ordering::Equal),
        }
    }
}

impl Iterator for TopNSearchResults {
    type Item = (SearchIndexScore, DocAddress);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.results.next()
    }
}

impl Iterator for MultiSegmentSearchResults {
    type Item = (SearchIndexScore, DocAddress);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let last = self.iterators.last_mut()?;
            match last.next() {
                Some((score, doc_address)) => {
                    let ctid_ff = self.ctid_column.get_or_insert_with(|| {
                        FFType::new_ctid(
                            self.searcher
                                .segment_reader(doc_address.segment_ord)
                                .fast_fields(),
                        )
                    });
                    let scored = SearchIndexScore {
                        ctid: ctid_ff
                            .as_u64(doc_address.doc_id)
                            .expect("ctid should be present"),
                        bm25: score,
                    };

                    return Some((scored, doc_address));
                }
                None => {
                    // last iterator is empty, so pop it off, clear the fast field type cache,
                    // and loop back around to get the next one
                    self.iterators.pop();
                    self.ctid_column = None;
                    continue;
                }
            }
        }
    }
}

pub struct SearchIndexReader {
    index_rel: PgSearchRelation,
    searcher: Searcher,
    schema: SearchIndexSchema,
    underlying_reader: IndexReader,
    underlying_index: Index,
    query: Box<dyn Query>,
    need_scores: bool,

    // [`PinnedBuffer`] has a Drop impl, so we hold onto it but don't otherwise use it
    //
    // also, it's an Arc b/c if we're clone'd (we do derive it, after all), we only want this
    // buffer dropped once
    _cleanup_lock: Arc<PinnedBuffer>,
}

impl Clone for SearchIndexReader {
    fn clone(&self) -> Self {
        Self {
            index_rel: self.index_rel.clone(),
            searcher: self.searcher.clone(),
            schema: self.schema.clone(),
            underlying_reader: self.underlying_reader.clone(),
            underlying_index: self.underlying_index.clone(),
            query: self.query.box_clone(),
            need_scores: self.need_scores,
            _cleanup_lock: self._cleanup_lock.clone(),
        }
    }
}

impl SearchIndexReader {
    /// Open a tantivy index where, if searched, will return zero results, but has access to all
    /// the underlying [`SegmentReader`]s and such as specified by the `mvcc_style`.
    pub fn empty(index_relation: &PgSearchRelation, mvcc_style: MvccSatisfies) -> Result<Self> {
        Self::open(index_relation, SearchQueryInput::Empty, false, mvcc_style)
    }

    /// Open a tantivy index that, when searched, will return the results of the specified [`SearchQueryInput`].
    pub fn open(
        index_relation: &PgSearchRelation,
        search_query_input: SearchQueryInput,
        need_scores: bool,
        mvcc_style: MvccSatisfies,
    ) -> Result<Self> {
        // It is possible for index only scans and custom scans, which only check the visibility map
        // and do not fetch tuples from the heap, to suffer from the concurrent TID recycling problem.
        // This problem occurs due to a race condition: after vacuum is called, a concurrent index only or custom scan
        // reads in some dead ctids. ambulkdelete finishes immediately after, and Postgres updates its visibility map,
        //rendering those dead ctids visible. The concurrent scan then returns the wrong results.
        // To prevent this, ambulkdelete acquires an exclusive cleanup lock. Readers must also acquire this lock (shared)
        // to prevent a reader from reading dead ctids right before ambulkdelete finishes.
        //
        // It's sufficient, and **required** for parallel scans to operate correctly, for us to hold onto
        // a pinned but unlocked buffer.
        let cleanup_lock = MetaPage::open(index_relation).cleanup_lock_pinned();

        let directory = mvcc_style.directory(index_relation);
        let mut index = Index::open(directory)?;
        let schema = index_relation.schema()?;
        setup_tokenizers(index_relation, &mut index)?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        let searcher = reader.searcher();

        let need_scores = need_scores || search_query_input.need_scores();
        let query = {
            let mut parser = QueryParser::for_index(
                &index,
                schema.fields().map(|(field, _)| field).collect::<Vec<_>>(),
            );
            search_query_input
                .into_tantivy_query(
                    &schema,
                    &mut parser,
                    &searcher,
                    index_relation.oid(),
                    index_relation.rel_oid(),
                )
                .unwrap_or_else(|e| panic!("{e}"))
        };

        Ok(Self {
            index_rel: index_relation.clone(),
            searcher,
            schema,
            underlying_reader: reader,
            underlying_index: index,
            query,
            need_scores,
            _cleanup_lock: Arc::new(cleanup_lock),
        })
    }

    pub fn segment_ids(&self) -> Vec<SegmentId> {
        self.searcher
            .segment_readers()
            .iter()
            .map(|r| r.segment_id())
            .collect()
    }

    pub fn need_scores(&self) -> bool {
        self.need_scores
    }

    pub fn query(&self) -> &dyn Query {
        &self.query
    }

    pub fn weight(&self) -> Box<dyn Weight> {
        self.query
            .weight(if self.need_scores {
                tantivy::query::EnableScoring::Enabled {
                    searcher: &self.searcher,
                    statistics_provider: &self.searcher,
                }
            } else {
                tantivy::query::EnableScoring::Disabled {
                    schema: self.schema.tantivy_schema(),
                    searcher_opt: Some(&self.searcher),
                }
            })
            .expect("weight should be constructable")
    }

    pub fn make_query(&self, search_query_input: SearchQueryInput) -> Box<dyn Query> {
        let mut parser = QueryParser::for_index(
            &self.underlying_index,
            self.schema
                .fields()
                .map(|(field, _)| field)
                .collect::<Vec<_>>(),
        );
        search_query_input
            .clone()
            .into_tantivy_query(
                &self.schema,
                &mut parser,
                &self.searcher,
                self.index_rel.oid(),
                self.index_rel.rel_oid(),
            )
            .unwrap_or_else(|e| panic!("{e}"))
    }

    pub fn get_doc(&self, doc_address: DocAddress) -> tantivy::Result<TantivyDocument> {
        self.searcher.doc(doc_address)
    }

    /// Returns the index size, in bytes, according to tantivy
    pub fn byte_size(&self) -> Result<u64> {
        Ok(self
            .underlying_reader
            .searcher()
            .space_usage()
            .map(|space| space.total().get_bytes())?)
    }

    pub fn segment_readers(&self) -> &[SegmentReader] {
        self.searcher.segment_readers()
    }

    pub fn schema(&self) -> &SearchIndexSchema {
        &self.schema
    }

    pub fn searcher(&self) -> &Searcher {
        &self.searcher
    }

    pub fn validate_checksum(&self) -> Result<std::collections::HashSet<PathBuf>> {
        Ok(self.underlying_index.validate_checksum()?)
    }

    pub fn snippet_generator(
        &self,
        field_name: impl AsRef<str> + Display,
        query: SearchQueryInput,
    ) -> (tantivy::schema::Field, SnippetGenerator) {
        let search_field = self
            .schema
            .search_field(&field_name)
            .unwrap_or_else(|| panic!("snippet_generator: field {field_name} should exist"));
        if search_field.is_text() || search_field.is_json() {
            let field = search_field.field();
            let generator =
                SnippetGenerator::create(&self.searcher, &self.make_query(query), field)
                    .unwrap_or_else(|err| {
                        panic!(
                            "failed to create snippet generator for field: {field_name}... {err}"
                        )
                    });
            (field, generator)
        } else {
            panic!("failed to create snippet generator for field: {field_name}... can only highlight text fields")
        }
    }

    /// Search the Tantivy index for matching documents.
    ///
    /// The order of returned docs is unspecified.
    ///
    /// It has no understanding of Postgres MVCC visibility.  It is the caller's responsibility to
    /// handle that, if it's necessary.
    pub fn search(&self, _estimated_rows: Option<usize>) -> MultiSegmentSearchResults {
        self.search_segments(
            self.searcher()
                .segment_readers()
                .iter()
                .map(|s| s.segment_id()),
        )
    }

    /// Search specific index segments for matching documents.
    ///
    /// The order of returned docs is unspecified.
    ///
    /// It has no understanding of Postgres MVCC visibility.  It is the caller's responsibility to
    /// handle that, if it's necessary.
    pub fn search_segments(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
    ) -> MultiSegmentSearchResults {
        let iterators = self.collect_segments(segment_ids, |segment_ord, segment_reader| {
            ScorerIter::new(
                DeferredScorer::new(
                    self.query().box_clone(),
                    self.need_scores,
                    segment_reader.clone(),
                    self.searcher.clone(),
                ),
                segment_ord,
                segment_reader.clone(),
            )
        });

        MultiSegmentSearchResults {
            searcher: self.searcher.clone(),
            ctid_column: Default::default(),
            iterators,
        }
    }

    /// Search the Tantivy index for the "top N" matching documents in specific segments.
    ///
    /// The documents are returned in score order.  Most relevant first if `sortdir` is [`SortDirection::Desc`],
    /// or least relevant first if it's [`SortDirection::Asc`].
    ///
    /// It has no understanding of Postgres MVCC visibility.  It is the caller's responsibility to
    /// handle that, if it's necessary.
    #[allow(clippy::too_many_arguments)]
    pub fn search_top_n_in_segments(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
        sort_field: Option<FieldName>,
        sortdir: SortDirection,
        n: usize,
        offset: usize,
    ) -> TopNSearchResults {
        if let Some(sort_field) = sort_field {
            let field = self
                .schema
                .search_field(&sort_field)
                .expect("sort field should exist in index schema");
            match field.field_entry().field_type().value_type() {
                tantivy::schema::Type::Str => self.top_by_string_field_in_segments(
                    segment_ids,
                    sort_field,
                    sortdir,
                    n,
                    offset,
                ),
                _ => self.top_by_field_in_segments(segment_ids, sort_field, sortdir, n, offset),
            }
        } else {
            self.top_by_score_in_segments(segment_ids, sortdir, n, offset)
        }
    }

    /// Search the Tantivy index for the "top N" matching documents (ordered by a field) in the given segments.
    ///
    /// The documents are returned in field order.  Largest first if `sortdir` is [`SortDirection::Desc`],
    /// or smallest first if it's [`SortDirection::Asc`].
    ///
    /// It has no understanding of Postgres MVCC visibility.  It is the caller's responsibility to
    /// handle that, if it's necessary.
    fn top_by_field_in_segments(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
        sort_field: impl AsRef<str> + Display,
        sortdir: SortDirection,
        n: usize,
        offset: usize,
    ) -> TopNSearchResults {
        let collector = TopDocs::with_limit(n)
            .and_offset(offset)
            .order_by_u64_field(&sort_field, sortdir.into());
        let weight = self
            .query
            .weight(enable_scoring(self.need_scores, &self.searcher))
            .expect("creating a Weight from a Query should not fail");

        let top_docs = self.collect_segments(segment_ids, |segment_ord, segment_reader| {
            collector
                .collect_segment(weight.as_ref(), segment_ord, segment_reader)
                .expect("should be able to collect top-n in segment")
        });

        let top_docs = collector
            .merge_fruits(top_docs)
            .expect("should be able to merge top-n in segments");
        // TODO: We are discarding a u64-encoded numeric field value here.
        // To actually fetch it, we might switch the `TopDocs::order_by_u64_field` call to
        // `TopDocs::order_by_fast_field`, which handles the decoding.
        TopNSearchResults::new_for_score(
            &self.searcher,
            top_docs.into_iter().map(|(_field_value, doc)| (1.0, doc)),
        )
    }

    /// Search the Tantivy index for the "top N" matching documents (ordered by a field) in the given segments.
    ///
    /// The documents are returned in field order.  Largest first if `sortdir` is [`SortDirection::Desc`],
    /// or smallest first if it's [`SortDirection::Asc`].
    ///
    /// It has no understanding of Postgres MVCC visibility.  It is the caller's responsibility to
    /// handle that, if it's necessary.
    fn top_by_string_field_in_segments(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
        sort_field: FieldName,
        sortdir: SortDirection,
        n: usize,
        offset: usize,
    ) -> TopNSearchResults {
        let collector = TopDocs::with_limit(n)
            .and_offset(offset)
            .order_by_string_fast_field(&sort_field, sortdir.into());
        let weight = self
            .query
            .weight(enable_scoring(self.need_scores, &self.searcher))
            .expect("creating a Weight from a Query should not fail");

        let top_docs = self.collect_segments(segment_ids, |segment_ord, segment_reader| {
            collector
                .collect_segment(weight.as_ref(), segment_ord, segment_reader)
                .expect("should be able to collect top-n in segment")
        });

        let top_docs = collector
            .merge_fruits(top_docs)
            .expect("should be able to merge top-n in segments");
        TopNSearchResults::new_for_score(
            &self.searcher,
            // TODO: We are discarding a valid string field value here, but could in theory actually
            // render it using a virtual tuple for the right query shape.
            top_docs.into_iter().map(|(_, doc)| (1.0, doc)),
        )
    }

    /// Search the Tantivy index for the "top N" matching documents (ordered by score) in the given segments.
    ///
    /// The documents are returned in score order.  Most relevant first if `sortdir` is [`SortDirection::Desc`],
    /// or least relevant first if it's [`SortDirection::Asc`].
    ///
    /// It has no understanding of Postgres MVCC visibility.  It is the caller's responsibility to
    /// handle that, if it's necessary.
    fn top_by_score_in_segments(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
        sortdir: SortDirection,
        n: usize,
        offset: usize,
    ) -> TopNSearchResults {
        match sortdir {
            // requires tweaking the score, which is a bit slower
            SortDirection::Asc => {
                let weight = self
                    .query
                    .weight(tantivy::query::EnableScoring::Enabled {
                        searcher: &self.searcher,
                        statistics_provider: &self.searcher,
                    })
                    .expect("creating a Weight from a Query should not fail");

                let collector = TopDocs::with_limit(n).and_offset(offset).tweak_score(
                    move |_segment_reader: &tantivy::SegmentReader| {
                        move |_doc: DocId, original_score: Score| TweakedScore {
                            dir: sortdir,
                            score: original_score,
                        }
                    },
                );

                let top_docs = self.collect_segments(segment_ids, |segment_ord, segment_reader| {
                    collector
                        .collect_segment(weight.as_ref(), segment_ord, segment_reader)
                        .expect("should be able to collect top-n in segment")
                });

                let top_docs = collector
                    .merge_fruits(top_docs)
                    .expect("should be able to merge top-n in segment");

                TopNSearchResults::new_for_score(
                    &self.searcher,
                    top_docs
                        .into_iter()
                        .map(|(score, doc_address)| (score.score, doc_address)),
                )
            }

            // can use tantivy's score directly
            SortDirection::Desc => {
                let weight = self
                    .query
                    .weight(tantivy::query::EnableScoring::Enabled {
                        searcher: &self.searcher,
                        statistics_provider: &self.searcher,
                    })
                    .expect("creating a Weight from a Query should not fail");

                let collector = TopDocs::with_limit(n).and_offset(offset);

                let top_docs = self.collect_segments(segment_ids, |segment_ord, segment_reader| {
                    collector
                        .collect_segment(weight.as_ref(), segment_ord, segment_reader)
                        .expect("should be able to collect top-n in segment")
                });

                let top_docs = collector
                    .merge_fruits(top_docs)
                    .expect("should be able to merge top-n in segment");

                TopNSearchResults::new_for_score(&self.searcher, top_docs.into_iter())
            }

            SortDirection::None => TopNSearchResults::new(
                self.search_segments(segment_ids)
                    .skip(offset)
                    .take(n)
                    .collect(),
            ),
        }
    }

    pub fn estimate_docs(&self, total_docs: f64) -> Option<usize> {
        debug_assert!(self.searcher.segment_readers().len() == 1, "estimate_docs(): expected an index with only one segment, which is assumed to be the largest segment by num_docs");
        let largest_reader = self.searcher.segment_reader(0);
        let weight = self.weight();
        let mut scorer = weight
            .scorer(largest_reader, 1.0)
            .expect("counting docs in the largest segment should not fail");

        // investigate the size_hint.  it will often give us a good enough value
        let mut count = scorer.size_hint() as usize;
        if count == 0 {
            // but when it doesn't, we need to do a full count
            count = scorer.count_including_deleted() as usize;
        }
        let segment_doc_proportion = largest_reader.num_docs() as f64 / total_docs;

        Some((count as f64 / segment_doc_proportion).ceil() as usize)
    }

    pub fn collect<C: Collector>(&self, collector: C) -> C::Fruit {
        self.searcher
            .search_with_executor(
                &self.query,
                &collector,
                &Executor::SingleThread,
                enable_scoring(self.need_scores, &self.searcher),
            )
            .expect("search should not fail")
    }

    fn collect_segments<T>(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
        mut collect: impl FnMut(SegmentOrdinal, &SegmentReader) -> T,
    ) -> Vec<T> {
        segment_ids
            .map(|segment_id| {
                let (segment_ord, segment_reader) = self
                    .searcher
                    .segment_readers()
                    .iter()
                    .enumerate()
                    .find(|(_, reader)| reader.segment_id() == segment_id)
                    .unwrap_or_else(|| panic!("segment {segment_id} should exist"));
                collect(segment_ord as SegmentOrdinal, segment_reader)
            })
            .collect()
    }
}

pub(super) fn enable_scoring(need_scores: bool, searcher: &Searcher) -> EnableScoring {
    if need_scores {
        EnableScoring::enabled_from_searcher(searcher)
    } else {
        EnableScoring::disabled_from_searcher(searcher)
    }
}

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

use std::cmp::Ordering;
use std::fmt::{Debug, Display};
use std::path::PathBuf;
use std::ptr::NonNull;
use std::sync::Arc;

use crate::aggregate::mvcc_collector::MVCCFilterCollector;
use crate::aggregate::vischeck::TSVisibilityChecker;
use crate::api::{HashMap, OrderByFeature, OrderByInfo, SortDirection};
use crate::index::fast_fields_helper::FFType;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::scorer::{DeferredScorer, ScorerIter};
use crate::index::setup_tokenizers;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::buffer::PinnedBuffer;
use crate::postgres::storage::metadata::MetaPage;
use crate::query::estimate_tree::QueryWithEstimates;
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;

use anyhow::Result;
use tantivy::aggregation::intermediate_agg_result::IntermediateAggregationResults;
use tantivy::aggregation::DistributedAggregationCollector;
use tantivy::collector::sort_key::{
    SortByOwnedValue, SortBySimilarityScore, SortByStaticFastValue, SortByString,
};
use tantivy::collector::{Collector, SegmentCollector, SortKeyComputer, TopDocs};
use tantivy::index::{Index, SegmentId};
use tantivy::query::{EnableScoring, QueryClone, QueryParser, Weight};
use tantivy::snippet::SnippetGenerator;
use tantivy::{
    query::Query, schema::OwnedValue, DateTime, DocAddress, DocId, DocSet, Executor, IndexReader,
    Order, ReloadPolicy, Score, Searcher, SegmentOrdinal, SegmentReader, TantivyDocument,
};

/// The maximum number of sort-features/`OrderByInfo`s supported for
/// `SearchIndexReader::search_top_n_in_segments`.
pub const MAX_TOPN_FEATURES: usize = 3;

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
        // TODO: Should also compare the ctid for stability.
        self.bm25.partial_cmp(&other.bm25)
    }
}

pub type FastFieldCache = HashMap<SegmentOrdinal, FFType>;

/// See `SearchIndexReader::top_in_segments`.
type TopNWithAggregate<T> = (
    Vec<((T, Option<Score>), DocAddress)>,
    Option<IntermediateAggregationResults>,
);

/// A known-size iterator of results for Top-N.
pub struct TopNSearchResults {
    results_original_len: usize,
    results: std::vec::IntoIter<(SearchIndexScore, DocAddress)>,
    aggregation_results: Option<IntermediateAggregationResults>,
}

impl TopNSearchResults {
    pub fn empty() -> Self {
        Self::new(vec![], None)
    }

    fn new(
        results: Vec<(SearchIndexScore, DocAddress)>,
        aggregation_results: Option<IntermediateAggregationResults>,
    ) -> Self {
        Self {
            results_original_len: results.len(),
            results: results.into_iter(),
            aggregation_results,
        }
    }

    fn new_for_score(
        searcher: &Searcher,
        results: impl IntoIterator<Item = (Score, DocAddress)>,
        aggregation_results: Option<IntermediateAggregationResults>,
    ) -> Self {
        // TODO: Execute batch lookups for ctids?
        let mut ff_lookup = FastFieldCache::default();
        Self::new(
            results
                .into_iter()
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
            aggregation_results,
        )
    }

    /// After a TopDocs search on a field, we have a valid field value, which this method will
    /// discard.
    ///
    /// TODO: We could in theory actually render that field using a virtual tuple (for the right
    /// query), similar to what we do in fast-fields execution.
    fn new_for_discarded_field<T>(searcher: &Searcher, results: TopNWithAggregate<T>) -> Self {
        let (results, aggregation_results) = results;
        Self::new_for_score(
            searcher,
            results
                .into_iter()
                .map(|((_, score), doc)| (score.unwrap_or(1.0), doc)),
            aggregation_results,
        )
    }

    pub fn original_len(&self) -> usize {
        self.results_original_len
    }

    pub fn take_aggregation_results(&mut self) -> Option<IntermediateAggregationResults> {
        self.aggregation_results.take()
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

/// A score which sorts in ascending direction.
#[derive(PartialEq, Clone, Debug)]
pub struct AscendingScore {
    score: Score,
}

impl PartialOrd for AscendingScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.score.partial_cmp(&other.score).map(|o| o.reverse())
    }
}

impl Iterator for TopNSearchResults {
    type Item = (SearchIndexScore, DocAddress);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.results.next()
    }
}

impl MultiSegmentSearchResults {
    pub fn current_segment(&mut self) -> Option<&mut ScorerIter> {
        self.iterators.last_mut()
    }

    pub fn current_segment_pop(&mut self) -> Option<ScorerIter> {
        self.iterators.pop()
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

/// Defines auxiliary `Collector`s that may be used in parallel/around TopN.
///
/// The TopDocs collectors themselves are highly specialized based on field and query types, and so
/// usually cannot have their types spelled all the way out: they are defined by the method calls
/// below `search_top_n_in_segments`. This struct defines optional wrappers and neighbors for that
/// core TopN collector.
pub struct TopNAuxiliaryCollector {
    /// If aggregations should be computed alongside TopN, the collector to use.
    pub aggregation_collector: DistributedAggregationCollector,
    /// If MVCC filtering should be applied, then the visibility checker to use for that.
    ///
    /// Note: If enabled, visibility checking is applied to to _both_ the TopN and to any
    /// aggregation collector: this is because once you've bothered to filter for MVCC, you might
    /// as well feed the filtered result to TopN too.
    pub vischeck: Option<TSVisibilityChecker>,
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
        Self::open_with_context(
            index_relation,
            search_query_input,
            need_scores,
            mvcc_style,
            None,
            None,
        )
    }

    /// Open a tantivy index with optional expression context for proper postgres expression evaluation
    pub fn open_with_context(
        index_relation: &PgSearchRelation,
        search_query_input: SearchQueryInput,
        need_scores: bool,
        mvcc_style: MvccSatisfies,
        expr_context: Option<NonNull<pgrx::pg_sys::ExprContext>>,
        planstate: Option<NonNull<pgrx::pg_sys::PlanState>>,
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
            search_query_input
                .into_tantivy_query(
                    &schema,
                    &|| {
                        QueryParser::for_index(
                            &index,
                            schema.fields().map(|(field, _)| field).collect::<Vec<_>>(),
                        )
                    },
                    &searcher,
                    index_relation.oid(),
                    index_relation.rel_oid(),
                    expr_context,
                    planstate,
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

    fn make_query(
        &self,
        search_query_input: &SearchQueryInput,
        expr_context: Option<NonNull<pgrx::pg_sys::ExprContext>>,
    ) -> Box<dyn Query> {
        search_query_input
            .clone()
            .into_tantivy_query(
                &self.schema,
                &|| {
                    QueryParser::for_index(
                        &self.underlying_index,
                        self.schema
                            .fields()
                            .map(|(field, _)| field)
                            .collect::<Vec<_>>(),
                    )
                },
                &self.searcher,
                self.index_rel.oid(),
                self.index_rel.rel_oid(),
                expr_context,
                None, // no planstate
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
        query: &SearchQueryInput,
        expr_context: Option<NonNull<pgrx::pg_sys::ExprContext>>,
    ) -> (tantivy::schema::Field, SnippetGenerator) {
        let search_field = self
            .schema
            .search_field(&field_name)
            .unwrap_or_else(|| panic!("cannot generate snippet for field {field_name} because it was not found in the index"));
        if search_field.is_text() || search_field.is_json() {
            let field = search_field.field();
            let generator = SnippetGenerator::create(
                &self.searcher,
                &self.make_query(query, expr_context),
                field,
            )
            .unwrap_or_else(|err| {
                panic!("failed to create snippet generator for field: {field_name}... {err}")
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
    pub fn search(&self) -> MultiSegmentSearchResults {
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
        let iterators = self
            .segment_readers_in_segments(segment_ids)
            .map(|(segment_ord, segment_reader)| {
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
            })
            .collect();

        MultiSegmentSearchResults {
            searcher: self.searcher.clone(),
            ctid_column: Default::default(),
            iterators,
        }
    }

    /// Search the Tantivy index for "any unordered N" matching documents in specific segments.
    ///
    /// It has no understanding of Postgres MVCC visibility.  It is the caller's responsibility to
    /// handle that, if it's necessary.
    pub fn search_top_n_unordered_in_segments(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
        n: usize,
        offset: usize,
    ) -> TopNSearchResults {
        // Do an un-ordered search.
        TopNSearchResults::new(
            self.search_segments(segment_ids)
                .skip(offset)
                .take(n)
                .collect(),
            None,
        )
    }

    /// Search the Tantivy index for the "top N" matching documents in specific segments.
    ///
    /// The documents are returned in either score or field order, in the given direction: at least
    /// one `OrderByInfo` must be defined.
    ///
    /// If a TopNAuxiliaryCollector is provided, this method can optionally pre-filter for MVCC
    /// visibility: if a collector is _not_ provided, then it is up to the caller to filter the
    /// results for MVCC visibility, and re-query if necessary.
    pub fn search_top_n_in_segments(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
        orderby_info: &[OrderByInfo],
        n: usize,
        offset: usize,
        aux_collector: Option<TopNAuxiliaryCollector>,
    ) -> TopNSearchResults {
        let (first_orderby_info, erased_features) = self.prepare_features(orderby_info);
        match first_orderby_info {
            OrderByInfo {
                feature: OrderByFeature::Field(sort_field),
                direction,
                .. // TODO(#3266): Handle nulls_first for ORDER BY field sorting
            } => {
                let field = self
                    .schema
                    .search_field(sort_field)
                    .expect("sort field should exist in index schema");
                let order: Order = (*direction).into();
                match field.field_entry().field_type().value_type() {
                    tantivy::schema::Type::Str => TopNSearchResults::new_for_discarded_field(
                        &self.searcher,
                        self.top_in_segments(
                            segment_ids,
                            (SortByString::for_field(sort_field), order),
                            erased_features,
                            n,
                            offset,
                            aux_collector,
                        ),
                    ),
                    tantivy::schema::Type::U64 => TopNSearchResults::new_for_discarded_field(
                        &self.searcher,
                        self.top_in_segments(
                            segment_ids,
                            (SortByStaticFastValue::<u64>::for_field(sort_field), order),
                            erased_features,
                            n,
                            offset,
                            aux_collector,
                        ),
                    ),
                    tantivy::schema::Type::I64 => TopNSearchResults::new_for_discarded_field(
                        &self.searcher,
                        self.top_in_segments(
                            segment_ids,
                            (SortByStaticFastValue::<i64>::for_field(sort_field), order),
                            erased_features,
                            n,
                            offset,
                            aux_collector,
                        ),
                    ),
                    tantivy::schema::Type::F64 => TopNSearchResults::new_for_discarded_field(
                        &self.searcher,
                        self.top_in_segments(
                            segment_ids,
                            (SortByStaticFastValue::<f64>::for_field(sort_field), order),
                            erased_features,
                            n,
                            offset,
                            aux_collector,
                        ),
                    ),
                    tantivy::schema::Type::Bool => TopNSearchResults::new_for_discarded_field(
                        &self.searcher,
                        self.top_in_segments(
                            segment_ids,
                            (SortByStaticFastValue::<bool>::for_field(sort_field), order),
                            erased_features,
                            n,
                            offset,
                            aux_collector,
                        ),
                    ),
                    tantivy::schema::Type::Date => TopNSearchResults::new_for_discarded_field(
                        &self.searcher,
                        self.top_in_segments(
                            segment_ids,
                            (SortByStaticFastValue::<DateTime>::for_field(sort_field), order),
                            erased_features,
                            n,
                            offset,
                            aux_collector,
                        ),
                    ),
                    x => {
                        // NOTE: This list of supported field types must be synced with
                        // `SearchField::is_sortable`.
                        panic!("Unsupported order-by field type: {x:?}");
                    }
                }
            }
            OrderByInfo {
                feature: OrderByFeature::Score,
                direction,
                .. // TODO(#3266): Handle nulls_first for ORDER BY score sorting
            } if !erased_features.is_empty() => {
                // If we've directly sorted on the score, then we have it available here.
                let order: Order = (*direction).into();
                let (top_docs, aggregation_results) = self.top_in_segments(
                    segment_ids,
                    (SortBySimilarityScore, order),
                    erased_features,
                    n,
                    offset,
                    aux_collector,
                );
                TopNSearchResults::new_for_score(
                    &self.searcher,
                    top_docs.into_iter().map(|((f, _), doc)| (f, doc)),
                    aggregation_results,
                )
            }
            OrderByInfo {
                feature: OrderByFeature::Score,
                direction,
                .. // TODO(#3266): Handle nulls_first for ORDER BY score sorting
            } => {
                // TODO: See method docs.
                self.top_by_score_in_segments(segment_ids, *direction, n, offset, aux_collector)
            }
        }
    }

    /// Called by `search_top_n_in_segments`.
    ///
    /// `search_top_n_in_segments` is specialized for all combinations of:
    /// 1. first sort field type -- via the generic `S: SortKeyComputer` parameter of this method. This
    ///    gets us unboxed/optimized comparison for the first feature, which always receives more
    ///    comparison than the remaining features (sometimes a lot more).
    /// 2. supported sort field counts (from 1 to MAX_TOPN_FEATURES) -- by calls to
    ///    `top_for_orderable_in_segments` for varying tuple lengths. Ordering on tuples is what is
    ///    supported by `TopDocs::order_by`, because it avoids allocation, and allows for the most
    ///    inlining of comparisons.
    ///
    /// To avoid a combinatorial explosion of generated code we do not support specializing more
    /// than the first sort field type: to do so, we'd likely need a macro which generated all
    /// possible permutations of `S: SortKeyComputer` types for three columns (which would be 7^3=343 copies
    /// of the method at time of writing).
    #[allow(clippy::type_complexity, clippy::too_many_arguments)]
    fn top_in_segments<S>(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
        first_feature: S,
        mut erased_features: ErasedFeatures,
        n: usize,
        offset: usize,
        aux_collector: Option<TopNAuxiliaryCollector>,
    ) -> TopNWithAggregate<S::SortKey>
    where
        S: SortKeyComputer + Clone + Send + 'static,
    {
        // if last erased feature is score, then we need to return the score
        match erased_features.len() {
            0 => {
                let top_docs_collector = TopDocs::with_limit(n)
                    .and_offset(offset)
                    .order_by::<S::SortKey>(first_feature);

                let (top_docs, aggregation_results) =
                    self.collect_maybe_auxiliary(segment_ids, top_docs_collector, aux_collector);

                (
                    top_docs
                        .into_iter()
                        .map(|(f, doc)| ((f, None), doc))
                        .collect(),
                    aggregation_results,
                )
            }
            1 => {
                let erased_feature = erased_features.pop().unwrap();
                let top_docs_collector = TopDocs::with_limit(n)
                    .and_offset(offset)
                    .order_by((first_feature, erased_feature));

                let (top_docs, aggregation_results) =
                    self.collect_maybe_auxiliary(segment_ids, top_docs_collector, aux_collector);

                (
                    top_docs
                        .into_iter()
                        .map(|((f, erased1), doc)| {
                            let maybe_score = erased_features.try_get_score(&[erased1]);
                            ((f, maybe_score), doc)
                        })
                        .collect(),
                    aggregation_results,
                )
            }
            2 => {
                let erased_feature2 = erased_features.pop().unwrap();
                let erased_feature1 = erased_features.pop().unwrap();
                let top_docs_collector = TopDocs::with_limit(n).and_offset(offset).order_by((
                    first_feature,
                    erased_feature1,
                    erased_feature2,
                ));

                let (top_docs, aggregation_results) =
                    self.collect_maybe_auxiliary(segment_ids, top_docs_collector, aux_collector);

                (
                    top_docs
                        .into_iter()
                        .map(|((f, erased1, erased2), doc)| {
                            let maybe_score = erased_features.try_get_score(&[erased1, erased2]);
                            ((f, maybe_score), doc)
                        })
                        .collect(),
                    aggregation_results,
                )
            }
            x => {
                if erased_features.score_index() == Some(x - 1) {
                    panic!(
                        "Unsupported sort-field count: {}. At most {} are supported when `pdb.score` is requested.",
                        x, MAX_TOPN_FEATURES - 1
                    )
                } else {
                    panic!(
                        "Unsupported sort-field count: {}. At most {MAX_TOPN_FEATURES} are supported.",
                        x + 1,
                    )
                }
            }
        }
    }

    /// Order by score only.
    ///
    /// NOTE: This is a special case for a single score feature: the score-only codepath is highly
    /// specializedi due to Block-WAND, and at least 50% faster than `TopDocs::order_by` when
    /// sorting on only the score. We should try to close that gap over time, but for now we
    /// special case it.
    ///
    /// TODO: Confirm after rebasing atop https://github.com/quickwit-oss/tantivy/pull/2726
    fn top_by_score_in_segments(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
        sortdir: SortDirection,
        n: usize,
        offset: usize,
        aux_collector: Option<TopNAuxiliaryCollector>,
    ) -> TopNSearchResults {
        match sortdir {
            // requires tweaking the score, which is a bit slower
            SortDirection::Asc => {
                let top_docs_collector = TopDocs::with_limit(n).and_offset(offset).tweak_score(
                    move |_segment_reader: &tantivy::SegmentReader| {
                        move |_doc: DocId, original_score: Score| AscendingScore {
                            score: original_score,
                        }
                    },
                );

                let (top_docs, aggregation_results) =
                    self.collect_maybe_auxiliary(segment_ids, top_docs_collector, aux_collector);

                TopNSearchResults::new_for_score(
                    &self.searcher,
                    top_docs
                        .into_iter()
                        .map(|(score, doc_address)| (score.score, doc_address)),
                    aggregation_results,
                )
            }

            // can use tantivy's score directly, which allows for Block-WAND
            SortDirection::Desc => {
                let top_docs_collector = TopDocs::with_limit(n).and_offset(offset).order_by_score();

                let (top_docs, aggregation_results) =
                    self.collect_maybe_auxiliary(segment_ids, top_docs_collector, aux_collector);

                TopNSearchResults::new_for_score(&self.searcher, top_docs, aggregation_results)
            }
        }
    }

    /// Given an estimate of the total number of rows in the relation, return an estimate of the
    /// number of rows which will be matched by the configured query.
    ///
    /// Expects to be called using an index opened with `MvccSatisfies::LargestSegment`, and thus
    /// to contain exactly 0 or 1 Segment.
    pub fn estimate_docs(&self, total_docs: f64) -> usize {
        match self.searcher.segment_readers().len() {
            1 => {}
            0 => return 0,
            x => {
                panic!(
                    "estimate_docs(): expected an index with only one segment, \
                    which is assumed to be the largest segment by num_docs. got: {x:?} segments.",
                );
            }
        }
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

        (count as f64 / segment_doc_proportion).ceil() as usize
    }

    /// Build a query tree with recursive estimates for EXPLAIN output.
    pub fn build_query_tree_with_estimates(
        &self,
        query_input: SearchQueryInput,
    ) -> Result<QueryWithEstimates> {
        let parser_closure = || {
            QueryParser::for_index(
                &self.underlying_index,
                self.schema
                    .fields()
                    .map(|(field, _)| field)
                    .collect::<Vec<_>>(),
            )
        };

        let (_tantivy_query, mut query_tree) = query_input.into_tantivy_query_with_tree(
            &self.schema,
            &parser_closure,
            &self.searcher,
            self.index_rel.oid(),
            self.index_rel.rel_oid(),
            None, // expr_context not needed for estimation
            None,
        )?;

        let total_docs = self.searcher.num_docs() as f64;
        self.estimate_docs_recursive(&mut query_tree, total_docs, &parser_closure);

        Ok(query_tree)
    }

    fn estimate_docs_recursive<QueryParserCtor: Fn() -> QueryParser>(
        &self,
        query_tree: &mut QueryWithEstimates,
        total_docs: f64,
        parser: &QueryParserCtor,
    ) {
        let segment_readers = self.searcher.segment_readers();

        if segment_readers.is_empty() {
            query_tree.traverse_mut(0, &mut |node, _depth| {
                node.estimated_docs = Some(0);
            });
            return;
        }

        // Find the largest segment by num_docs for estimation
        let largest_reader = segment_readers
            .iter()
            .max_by_key(|r| r.num_docs())
            .expect("should have at least one segment reader");

        let segment_doc_proportion = largest_reader.num_docs() as f64 / total_docs;
        self.estimate_node_recursive(query_tree, largest_reader, segment_doc_proportion, parser);
    }

    fn estimate_node_recursive<QueryParserCtor: Fn() -> QueryParser>(
        &self,
        node: &mut QueryWithEstimates,
        largest_reader: &SegmentReader,
        segment_doc_proportion: f64,
        parser: &QueryParserCtor,
    ) {
        use crate::query::SearchQueryInput;

        // First, recursively estimate all children
        for child in node.children_mut() {
            self.estimate_node_recursive(child, largest_reader, segment_doc_proportion, parser);
        }

        // For structural wrapper nodes (used for labeling in EXPLAIN output), inherit
        // estimate from child. These are placeholders created in into_tantivy_query_generic
        // to wrap children for better tree structure display.
        //
        // - Empty: used for Boolean clause labels ("Must Clause [0]", etc.)
        // - All: used for DisjunctionMax disjunct labels ("Disjunct [0]", etc.)
        //
        // Note: We check for exactly 1 child to distinguish structural wrappers from
        // actual leaf queries (e.g., real "All" query has 0 children and should be estimated).
        if matches!(&node.query, SearchQueryInput::Empty | SearchQueryInput::All)
            && node.children().len() == 1
        {
            if let Some(child_estimate) = node.children()[0].estimated_docs {
                node.set_estimate(child_estimate);
                return;
            }
        }

        let tantivy_query = node
            .query
            .clone()
            .into_tantivy_query(
                &self.schema,
                parser,
                &self.searcher,
                self.index_rel.oid(),
                self.index_rel.rel_oid(),
                None,
                None,
            )
            .expect("converting query for estimation should not fail");

        // Use EnableScoring::Enabled because some queries (e.g., MoreLikeThisQuery)
        // require access to the searcher to build their internal query structure.
        // We're not using the actual scores, just counting documents.
        let weight = tantivy_query
            .weight(EnableScoring::Enabled {
                searcher: &self.searcher,
                statistics_provider: &self.searcher,
            })
            .expect("creating weight for estimation should not fail");

        let mut scorer = weight
            .scorer(largest_reader, 1.0)
            .expect("creating scorer for estimation should not fail");

        let mut count = scorer.size_hint() as usize;
        if count == 0 {
            count = scorer.count_including_deleted() as usize;
        }

        let estimated = if segment_doc_proportion > 0.0 {
            (count as f64 / segment_doc_proportion).ceil() as usize
        } else {
            count
        };

        node.set_estimate(estimated);
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

    /// Collect for the given Collector, optionally paired with / wrapped with the given auxiliary
    /// Collector(s).
    fn collect_maybe_auxiliary<C: Collector>(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
        top_docs_collector: C,
        aux_collector: Option<TopNAuxiliaryCollector>,
    ) -> (C::Fruit, Option<IntermediateAggregationResults>) {
        let query = self.query();
        let weight = query
            .weight(enable_scoring(self.need_scores, &self.searcher))
            .expect("creating a Weight from a Query should not fail");

        let Some(aux_collector) = aux_collector else {
            // No auxiliary collector.
            let fruits = self.collect_segments(segment_ids, &top_docs_collector, weight.as_ref());
            let top_docs = top_docs_collector
                .merge_fruits(fruits)
                .expect("should be able to merge top-n in segments");
            return (top_docs, None);
        };

        // We are executing a compound / tuple collection with an aggregation.
        let compound_collector = (top_docs_collector, aux_collector.aggregation_collector);

        // Optionally wrap in MVCC visibility filtering, if requested.
        if let Some(vischeck) = aux_collector.vischeck {
            let collector = MVCCFilterCollector::new(compound_collector, vischeck);
            let fruits = self.collect_segments(segment_ids, &collector, weight.as_ref());
            let (top_docs, aggregation_results) = collector
                .merge_fruits(fruits)
                .expect("should be able to merge top-n in segment");
            (top_docs, Some(aggregation_results))
        } else {
            let fruits = self.collect_segments(segment_ids, &compound_collector, weight.as_ref());
            let (top_docs, aggregation_results) = compound_collector
                .merge_fruits(fruits)
                .expect("should be able to merge top-n in segment");
            (top_docs, Some(aggregation_results))
        }
    }

    /// Create erased Features for the given OrderByInfo, which must contain at least one item.
    ///
    /// See `top_in_segments` and `sort_features!`.
    ///
    /// Additionally, if we need scores, this method will ensure that at least one of
    /// these features is a SortBySimilarityScore (see comment within function below)
    fn prepare_features<'a>(
        &'_ self,
        orderby_infos: &'a [OrderByInfo],
    ) -> (&'a OrderByInfo, ErasedFeatures) {
        let (first_orderby_info, remainder) = orderby_infos
            .split_first()
            .expect("must have at least one `ORDER BY`.");
        let mut erased_features = ErasedFeatures::default();

        for orderby_info in remainder.iter() {
            match orderby_info {
                OrderByInfo {
                    feature: OrderByFeature::Field(sort_field),
                    direction,
                    .. // TODO(#3266): Handle nulls_first for ORDER BY field sorting
                } => {
                    // NOTE: The list of supported field types for `SortByOwnedValue` must be synced with
                    // `SearchField::is_sortable`.
                    erased_features.push_feature(SortByOwnedValue::for_field(sort_field), *direction);
                }
                OrderByInfo {
                    feature: OrderByFeature::Score,
                    direction,
                    .. // TODO(#3266): Handle nulls_first for ORDER BY score sorting
                } => {
                    erased_features.push_score_feature(*direction);
                }
            }
        }

        // if we need scores, but there's no score feature in the order by list,
        // we push an erased score feature to the end of the list for the purpose of holding scores
        if self.need_scores
            && erased_features.score_index().is_none()
            && !first_orderby_info.is_score()
        {
            erased_features.push_score_feature(SortDirection::Desc);
        }

        (first_orderby_info, erased_features)
    }

    /// NOTE: It is very important that this method consumes the input SegmentIds lazily, because
    /// some callers (the TopN exec method in particular) are producing them lazily by checking
    /// them out of shared mutable state as they go.
    ///
    /// TODO: See https://github.com/paradedb/paradedb/issues/2758 about removing the O(N) behavior
    /// here.
    fn segment_readers_in_segments(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
    ) -> impl Iterator<Item = (SegmentOrdinal, &SegmentReader)> {
        segment_ids.map(|segment_id| {
            let (segment_ord, segment_reader) = self
                .searcher
                .segment_readers()
                .iter()
                .enumerate()
                .find(|(_, reader)| reader.segment_id() == segment_id)
                .unwrap_or_else(|| panic!("segment {segment_id} should exist"));
            (segment_ord as SegmentOrdinal, segment_reader)
        })
    }

    fn collect_segments<C: Collector>(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
        collector: &C,
        weight: &dyn Weight,
    ) -> Vec<<<C as Collector>::Child as SegmentCollector>::Fruit> {
        self.segment_readers_in_segments(segment_ids)
            .map(|(segment_ord, segment_reader)| {
                collector
                    .collect_segment(weight, segment_ord, segment_reader)
                    .expect("should be able to collect in segment")
            })
            .collect()
    }
}

pub(super) fn enable_scoring(need_scores: bool, searcher: &Searcher) -> EnableScoring<'_> {
    if need_scores {
        EnableScoring::enabled_from_searcher(searcher)
    } else {
        EnableScoring::disabled_from_searcher(searcher)
    }
}

#[derive(Default)]
pub struct ErasedFeatures {
    features: Vec<(SortByOwnedValue, SortDirection)>,
    // which, if any, of the erased features is the score feature
    // note: once https://github.com/quickwit-oss/tantivy/pull/2681#issuecomment-3340222261 is resolved,
    // this will be unnecessary
    score_index: Option<usize>,
}

impl ErasedFeatures {
    pub fn len(&self) -> usize {
        self.features.len()
    }

    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }

    pub fn pop(&mut self) -> Option<(SortByOwnedValue, Order)> {
        self.features.pop().map(|(s, sort_direction)| {
            let order: Order = sort_direction.into();
            (s, order)
        })
    }

    /// Push a non-score feature.
    pub fn push_feature(&mut self, feature: SortByOwnedValue, direction: SortDirection) {
        self.features.push((feature, direction));
    }

    /// Push a score feature.
    pub fn push_score_feature(&mut self, direction: SortDirection) {
        self.score_index = Some(self.features.len());
        self.features
            .push((SortByOwnedValue::for_score(), direction));
    }

    pub fn score_index(&self) -> Option<usize> {
        self.score_index
    }

    pub fn try_get_score(&self, values: &[OwnedValue]) -> Option<Score> {
        self.score_index.and_then(|i| match values[i] {
            OwnedValue::F64(f) => Some(f as Score),
            OwnedValue::Null => None,
            _ => panic!("expected a f64 for the score"),
        })
    }
}

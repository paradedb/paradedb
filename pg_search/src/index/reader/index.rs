// Copyright (c) 2023-2026 ParadeDB, Inc.
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
use std::collections::BTreeMap;
use std::fmt::{Debug, Display};
use std::path::PathBuf;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::Arc;

use crate::aggregate::mvcc_collector::MVCCFilterCollector;
use crate::api::version::Version;
use crate::api::{FieldName, HashSet, OrderByFeature, OrderByInfo, SortDirection};
use crate::index::fast_fields_helper::{FFType, resolve_ctid};
use crate::index::mvcc::{MVCCDirectory, MvccSatisfies, SegmentPins, SegmentView};
use crate::index::reader::io_stats;
use crate::index::reader::scorer::{DeferredScorer, LazyWeight, ScorerIter};
use crate::index::reader::sort_by_range::SortByRange;
use crate::index::segment_pruning::SegmentStatsSnapshot;
use crate::index::setup_tokenizers;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::options::{SortByDirection, SortByField};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::sequentialscan::KeySet;
use crate::postgres::storage::buffer::PinnedBuffer;
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::types::TantivyValue;
use crate::query::SearchQueryInput;
use crate::query::estimate_tree::QueryWithEstimates;
use crate::query::segment_pruning::SegmentPruner;
use crate::scan::info::RowEstimate;
use crate::schema::{SearchFieldType, SearchIndexSchema};

use anyhow::Result;
use tantivy::aggregation::DistributedAggregationCollector;
use tantivy::aggregation::intermediate_agg_result::IntermediateAggregationResults;
use tantivy::collector::sort_key::{
    ComparatorEnum, SortByBytes, SortByErasedType, SortBySimilarityScore, SortByStaticFastValue,
    SortByString,
};
use tantivy::collector::{Collector, SegmentCollector, SortKeyComputer, TopDocs};
use tantivy::index::{Index, Order, SegmentId};
use tantivy::query::{EnableScoring, QueryClone, QueryParser, Weight};
use tantivy::snippet::SnippetGenerator;
use tantivy::vector::ProbeStats;
use tantivy::vector::ivf::AdaptiveProbeParams;
use tantivy::{
    DateTime, DocAddress, DocId, DocSet, IndexReader, ReloadPolicy, Score, Searcher,
    SegmentOrdinal, SegmentReader, TantivyDocument, Term, query::Query, schema::OwnedValue,
};

/// The maximum number of sort-features/`OrderByInfo`s supported for
/// `SearchIndexReader::search_top_k_in_segments`.
pub const MAX_TOPK_FEATURES: usize = 5;

#[derive(Debug, Clone, Copy)]
pub struct DocsEstimate {
    pub matching_docs: usize,
    pub total_docs: u64,
    pub query_cost: u64,
}

/// A count-only summary of the pruning proof for this reader's execution snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SegmentPruningEstimate {
    pub(crate) candidate_segments: usize,
    pub(crate) candidate_docs: u64,
}

fn scale_largest_segment_estimate(value: u64, segment_doc_proportion: f64) -> u64 {
    if segment_doc_proportion > 0.0 {
        (value as f64 / segment_doc_proportion).ceil() as u64
    } else {
        value
    }
}

/// Represents a matching document from a tantivy search.  Typically, it is returned as an Iterator
/// Item alongside the originating tantivy [`DocAddress`]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchIndexScore {
    pub bm25: f32,
}

impl SearchIndexScore {
    #[inline]
    pub fn new(score: Score) -> Self {
        Self { bm25: score }
    }
}

impl PartialOrd for SearchIndexScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.bm25.partial_cmp(&other.bm25)
    }
}

/// See `SearchIndexReader::top_in_segments`.
type TopKWithAggregate<T> = (
    Vec<((T, Option<Score>), DocAddress)>,
    Option<IntermediateAggregationResults>,
);

/// A known-size iterator of results for Top K.
pub struct TopKSearchResults {
    results_original_len: usize,
    results: std::vec::IntoIter<(SearchIndexScore, DocAddress)>,
    aggregation_results: Option<IntermediateAggregationResults>,
}

/// Docs (+ optional aggregations) from a TopK search, plus opaque per-segment
/// JSON info harvested from the collector Fruit (e.g. vector probe stats).
pub struct TopKSearch {
    pub results: TopKSearchResults,
    pub segment_info: BTreeMap<SegmentId, serde_json::Value>,
}

impl TopKSearch {
    fn from_results(results: TopKSearchResults) -> Self {
        Self {
            results,
            segment_info: BTreeMap::new(),
        }
    }

    fn with_segment_info(
        results: TopKSearchResults,
        segment_info: BTreeMap<SegmentId, serde_json::Value>,
    ) -> Self {
        Self {
            results,
            segment_info,
        }
    }
}

impl From<TopKSearchResults> for TopKSearch {
    fn from(results: TopKSearchResults) -> Self {
        Self::from_results(results)
    }
}

fn probe_stats_to_segment_info(
    segment_ids: &[SegmentId],
    stats: &[ProbeStats],
) -> BTreeMap<SegmentId, serde_json::Value> {
    assert_eq!(
        segment_ids.len(),
        stats.len(),
        "vector Fruit must yield one ProbeStats per collected segment"
    );
    segment_ids
        .iter()
        .zip(stats.iter())
        .map(|(id, s)| {
            let value = serde_json::to_value(s).expect("ProbeStats should serialize to JSON");
            (*id, value)
        })
        .collect()
}

impl TopKSearchResults {
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
        results: impl IntoIterator<Item = (Score, DocAddress)>,
        aggregation_results: Option<IntermediateAggregationResults>,
    ) -> Self {
        Self::new(
            results
                .into_iter()
                .map(|(score, doc_address)| {
                    let scored = SearchIndexScore { bm25: score };
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
    fn new_for_discarded_field<T>(results: TopKWithAggregate<T>) -> Self {
        let (results, aggregation_results) = results;
        Self::new_for_score(
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
    iterators: Vec<ScorerIter>,
    lazy_iterators: Option<Box<dyn Iterator<Item = ScorerIter> + Send>>,
    lazy_estimated_rows: Option<u64>,
}

/// A score which sorts in ascending direction.
#[derive(PartialEq, Clone, Debug)]
struct AscendingScore {
    score: Score,
}

impl PartialOrd for AscendingScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.score.partial_cmp(&other.score).map(|o| o.reverse())
    }
}

impl Iterator for TopKSearchResults {
    type Item = (SearchIndexScore, DocAddress);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.results.next()
    }
}

impl MultiSegmentSearchResults {
    pub fn current_segment(&mut self) -> Option<&mut ScorerIter> {
        if self.iterators.is_empty()
            && let Some(ref mut lazy) = self.lazy_iterators
            && let Some(next_iter) = lazy.next()
        {
            self.iterators.push(next_iter);
        }
        self.iterators.last_mut()
    }

    pub fn current_segment_pop(&mut self) -> Option<ScorerIter> {
        self.iterators.pop()
    }

    pub fn segment_ids(&self) -> Vec<SegmentId> {
        self.iterators.iter().map(|it| it.segment_id()).collect()
    }

    /// Returns the total estimated number of documents across all segments in these results.
    ///
    /// This has no visible sideeffects, but it requires actually opening all DeferredScorers
    /// for this iterator (if they are not lazy).
    pub fn estimated_doc_count(&self) -> u64 {
        if let Some(rows) = self.lazy_estimated_rows {
            rows
        } else {
            self.iterators
                .iter()
                .map(|iter| iter.estimated_doc_count() as u64)
                .sum()
        }
    }

    /// Consumes and returns all segment iterators along with the searcher.
    ///
    /// This is useful for DataFusion integration where each segment iterator
    /// becomes a separate partition in the execution plan. The searcher is needed
    /// to create single-segment wrappers via `from_single_segment`.
    pub fn into_segments(self) -> (Searcher, Vec<ScorerIter>) {
        (self.searcher, self.iterators)
    }

    /// Creates a new `MultiSegmentSearchResults` from a single segment iterator.
    ///
    /// This is used for per-segment partition scanning in DataFusion integration.
    pub fn from_single_segment(searcher: Searcher, scorer_iter: ScorerIter) -> Self {
        Self {
            searcher,
            iterators: vec![scorer_iter],
            lazy_iterators: None,
            lazy_estimated_rows: None,
        }
    }

    pub fn searcher(&self) -> &Searcher {
        &self.searcher
    }
}

impl Iterator for MultiSegmentSearchResults {
    type Item = (SearchIndexScore, DocAddress);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let last = self.current_segment()?;
            match last.next() {
                Some((score, doc_address)) => {
                    return Some((SearchIndexScore { bm25: score }, doc_address));
                }
                None => {
                    // last iterator is empty, pop it and loop around to the next one
                    self.current_segment_pop();
                    continue;
                }
            }
        }
    }
}

/// Defines auxiliary `Collector`s that may be used in parallel/around Top K.
///
/// The TopDocs collectors themselves are highly specialized based on field and query types, and so
/// usually cannot have their types spelled all the way out: they are defined by the method calls
/// below `search_top_k_in_segments`. This struct defines optional wrappers and neighbors for that
/// core Top K collector.
pub struct TopKAuxiliaryCollector {
    /// If aggregations should be computed alongside Top K, the collector to use.
    pub aggregation_collector: DistributedAggregationCollector,
    /// If MVCC filtering should be applied up front, then the visibility checker to use for that.
    ///
    /// Note: If set, visibility checking is applied to _both_ the Top K and to any
    /// aggregation collector: this is because once you've bothered to filter for MVCC, you might
    /// as well feed the filtered result to Top K too.
    ///
    /// `None` means either that MVCC filtering was not requested, or that it is solved lazily
    /// inside `aggregation_collector` (cardinality-only over string fields). In both cases Top K
    /// gets no pre-filtering here: the caller must verify visibility of the results and re-query
    /// if necessary, exactly as when no auxiliary collector is present.
    pub vischeck: Option<VisibilityChecker>,
}

pub struct SearchIndexReader {
    index_rel: PgSearchRelation,
    searcher: Searcher,
    schema: SearchIndexSchema,
    underlying_reader: IndexReader,
    underlying_index: Index,
    query: Box<dyn Query>,
    segment_stats_snapshot: Arc<SegmentStatsSnapshot>,
    /// Decides whether a segment is worth searching, one segment at a time, when that segment is
    /// about to be searched. Nothing is decided at open, so a planning reader that only estimates
    /// never reads statistics.
    search_query_input: SearchQueryInput,
    need_scores: bool,
    total_segment_count: usize,
    total_docs: u64,
    index_created_by_version: Option<Version>,
    /// The directory `underlying_index` opened over; kept to capture this reader's
    /// [`SegmentView`].
    directory: MVCCDirectory,

    // [`PinnedBuffer`] has a Drop impl, so we hold onto it but don't otherwise use it
    //
    // also, it's an Arc b/c if we're clone'd (we do derive it, after all), we only want this
    // buffer dropped once
    _cleanup_lock: Arc<PinnedBuffer>,
}

/// A queryless snapshot of visible segments used to initialize parallel JoinScan and
/// AggregateScan sources without requiring executor state. Clones are cheap handles to the same
/// backend-local components, keeping the captured segment set and its pins alive together.
#[derive(Clone)]
pub struct SearchIndexManifest(Rc<SearchIndexManifestInner>);

struct SearchIndexManifestInner {
    components: IndexComponents,
}

impl std::fmt::Debug for SearchIndexManifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchIndexManifest")
            .field(
                "segments",
                &self.components().searcher.segment_readers().len(),
            )
            .finish_non_exhaustive()
    }
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
            segment_stats_snapshot: Arc::clone(&self.segment_stats_snapshot),
            search_query_input: self.search_query_input.clone(),
            need_scores: self.need_scores,
            total_segment_count: self.total_segment_count,
            total_docs: self.total_docs,
            index_created_by_version: self.index_created_by_version,
            directory: self.directory.clone(),
            _cleanup_lock: self._cleanup_lock.clone(),
        }
    }
}

#[cfg(any(test, feature = "pg_test"))]
pub(crate) mod test_support {
    use super::PgSearchRelation;
    use pgrx::Spi;
    use std::sync::atomic::AtomicUsize;

    /// How many times the index was actually opened (metadata walk, pins, searcher build).
    /// Tests use it to prove reuse paths perform zero additional opens.
    pub(crate) static INDEX_COMPONENT_OPENS: AtomicUsize = AtomicUsize::new(0);
    /// Test fixture shared by the reuse/laziness tests: a table + ParadeDB index laid out as
    /// `immutable_batches` frozen segments of 10 rows (batch 0's titles contain "silver dragon",
    /// later batches "quiet river"), plus an optional 5-row mutable segment. Returns the opened
    /// index relation and the heap relation's OID.
    pub(crate) fn segmented_index_fixture(
        name: &str,
        immutable_batches: usize,
        with_mutable: bool,
    ) -> (PgSearchRelation, pgrx::pg_sys::Oid) {
        let mut sql = format!(
            "CREATE TABLE {name} (id bigint PRIMARY KEY, title text NOT NULL);
             CREATE INDEX {name}_idx ON {name}
             USING paradedb (id, (title::pdb.unicode_words('columnar=true')))
             WITH (target_segment_count = 8, background_layer_sizes = '0');
             SET paradedb.global_mutable_segment_rows = 0;"
        );
        for batch in 0..immutable_batches {
            let (lo, hi) = (batch * 10 + 1, batch * 10 + 10);
            let words = if batch == 0 {
                "silver dragon"
            } else {
                "quiet river"
            };
            sql.push_str(&format!(
                "INSERT INTO {name} SELECT g, '{words} ' || g FROM generate_series({lo}, {hi}) g;"
            ));
        }
        if with_mutable {
            let lo = immutable_batches * 10 + 1;
            sql.push_str(&format!(
                "SET paradedb.global_mutable_segment_rows = 10000;
                 INSERT INTO {name} SELECT g, 'mutable ' || g FROM generate_series({lo}, {}) g;",
                lo + 4
            ));
        }
        sql.push_str("RESET paradedb.global_mutable_segment_rows;");
        Spi::run(&sql).expect("fixture setup");
        unsafe { pgrx::pg_sys::CommandCounterIncrement() };

        let index_oid =
            Spi::get_one::<pgrx::pg_sys::Oid>(&format!("SELECT '{name}_idx'::regclass::oid"))
                .unwrap()
                .unwrap();
        let heap_oid =
            Spi::get_one::<pgrx::pg_sys::Oid>(&format!("SELECT '{name}'::regclass::oid"))
                .unwrap()
                .unwrap();
        (PgSearchRelation::open(index_oid), heap_oid)
    }
}

#[derive(Clone)]
struct IndexComponents {
    cleanup_lock: Arc<PinnedBuffer>,
    directory: MVCCDirectory,
    index: Index,
    /// Statistics of `searcher`'s segments, opened on first use. Built once per open, so readers
    /// sharing a manifest share one lazily opened cache.
    segment_stats_snapshot: Arc<SegmentStatsSnapshot>,
    reader: IndexReader,
    searcher: Searcher,
    total_segment_count: usize,
    total_docs: u64,
    schema: SearchIndexSchema,
}

impl SearchIndexReader {
    fn open_index_components(
        index_relation: &PgSearchRelation,
        mvcc_style: MvccSatisfies,
        needs_tokenizer_manager: bool,
    ) -> Result<IndexComponents> {
        #[cfg(any(test, feature = "pg_test"))]
        test_support::INDEX_COMPONENT_OPENS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let cleanup_lock = Arc::new(MetaPage::open(index_relation).cleanup_lock_pinned());

        let directory = mvcc_style.directory(index_relation);
        let mut index = Index::open(directory.clone())?;
        let total_segment_count = directory
            .total_segment_count()
            .load(std::sync::atomic::Ordering::Relaxed);
        let total_docs = directory
            .total_docs()
            .load(std::sync::atomic::Ordering::Relaxed) as u64;
        let schema = index_relation.schema()?;
        if needs_tokenizer_manager {
            setup_tokenizers(index_relation, &mut index)?;
        }

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        let searcher = reader.searcher();
        let segment_stats_snapshot = SegmentStatsSnapshot::capture(&searcher);

        Ok(IndexComponents {
            cleanup_lock,
            directory,
            index,
            segment_stats_snapshot,
            reader,
            searcher,
            total_segment_count,
            total_docs,
            schema,
        })
    }

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
        let needs_tokenizer_manager = search_query_input.needs_tokenizer();
        Self::open_with_context(
            index_relation,
            search_query_input,
            need_scores,
            mvcc_style,
            None,
            None,
            needs_tokenizer_manager,
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
        needs_tokenizer_manager: bool,
    ) -> Result<Self> {
        // Derive the tokenizer need from the query as well as the caller's flag: a caller
        // passing `false` alongside a query that tokenizes must not silently parse wrong.
        let needs_tokenizer_manager =
            needs_tokenizer_manager || search_query_input.needs_tokenizer();
        let components =
            Self::open_index_components(index_relation, mvcc_style, needs_tokenizer_manager)?;
        Self::from_components(
            index_relation,
            components,
            search_query_input,
            need_scores,
            expr_context,
            planstate,
        )
    }

    /// Build a reader over `manifest`'s already-open components: same searcher, same frozen
    /// segment set, no additional I/O. A fresh open would also install the index's
    /// tokenizers, so [`register_tokenizers`] does that here, into the managers the
    /// manifest's searcher already shares.
    ///
    /// [`register_tokenizers`]: crate::index::search::register_tokenizers
    pub fn from_manifest(
        manifest: &SearchIndexManifest,
        index_relation: &PgSearchRelation,
        search_query_input: SearchQueryInput,
        need_scores: bool,
        expr_context: Option<NonNull<pgrx::pg_sys::ExprContext>>,
        needs_tokenizer_manager: bool,
    ) -> Result<Self> {
        if needs_tokenizer_manager || search_query_input.needs_tokenizer() {
            crate::index::search::register_tokenizers(
                index_relation,
                &manifest.components().index,
            )?;
        }
        Self::from_components(
            index_relation,
            manifest.components().clone(),
            search_query_input,
            need_scores,
            expr_context,
            None,
        )
    }

    /// The shared tail of [`Self::open_with_context`] and [`Self::from_manifest`].
    fn from_components(
        index_relation: &PgSearchRelation,
        components: IndexComponents,
        search_query_input: SearchQueryInput,
        need_scores: bool,
        expr_context: Option<NonNull<pgrx::pg_sys::ExprContext>>,
        planstate: Option<NonNull<pgrx::pg_sys::PlanState>>,
    ) -> Result<Self> {
        let IndexComponents {
            cleanup_lock,
            directory,
            index,
            segment_stats_snapshot,
            reader,
            searcher,
            total_segment_count,
            total_docs,
            schema,
        } = components;

        let index_created_by_version = index_relation.created_by_version();
        let need_scores = need_scores || search_query_input.need_scores();
        let parser = || {
            QueryParser::for_index(
                &index,
                schema.fields().map(|(field, _)| field).collect::<Vec<_>>(),
            )
        };
        let candidate_query = search_query_input.clone();
        let query = search_query_input
            .into_tantivy_query(
                &schema,
                index_created_by_version,
                &parser,
                &searcher,
                index_relation.oid(),
                index_relation.rel_oid(),
                expr_context,
                planstate,
            )
            .unwrap_or_else(|e| panic!("{e}"));

        Ok(Self {
            index_rel: index_relation.clone(),
            searcher,
            schema,
            underlying_reader: reader,
            underlying_index: index,
            query,
            segment_stats_snapshot,
            search_query_input: candidate_query,
            need_scores,
            total_segment_count,
            total_docs,
            index_created_by_version,
            directory,
            _cleanup_lock: cleanup_lock,
        })
    }

    pub fn segment_ids(&self) -> Vec<SegmentId> {
        self.searcher
            .segment_readers()
            .iter()
            .map(|r| r.segment_id())
            .collect()
    }

    /// This reader's segment view, for other readers to replay through
    /// [`MvccSatisfies::ParallelWorker`].
    pub fn segment_view(&self) -> SegmentView {
        SegmentView::capture(self.searcher.segment_readers(), &self.directory)
    }

    /// Take over this reader's pins on its segments, so they outlive the reader. See
    /// [`SegmentPins`].
    pub fn segment_pins(&self) -> SegmentPins {
        self.directory.segment_pins()
    }

    pub fn need_scores(&self) -> bool {
        self.need_scores
    }

    /// The compiled query. Segment pruning happens in the reader's search and collection
    /// methods, not in the query.
    pub fn query(&self) -> &dyn Query {
        &self.query
    }

    /// Pruning keeps deciding from the original query input, which stays correct because a
    /// further AND never makes a rejected segment match. The compiled query is kept as is: it may
    /// contain Params resolved with an execution ExprContext.
    pub fn and_query(&self, additional_query: Box<dyn Query>) -> Self {
        let mut clone = self.clone();
        let existing = std::mem::replace(&mut clone.query, Box::new(tantivy::query::EmptyQuery));
        clone.query = Box::new(tantivy::query::BooleanQuery::new(vec![
            (tantivy::query::Occur::Must, existing),
            (tantivy::query::Occur::Must, additional_query),
        ]));
        clone
    }

    /// Compiled without an expression context: only the reader's own query carries resolved
    /// Params, and it is kept as is.
    pub(crate) fn and_query_input(&self, query: &SearchQueryInput) -> Self {
        self.and_query(self.make_query(query, None))
    }

    /// Whether a scan restricted to `segment_ids` can omit `query`: every segment in the list
    /// that this reader would search is fully covered by it, and scores are not needed, since
    /// omitting a query drops its constant score contribution.
    pub(crate) fn partition_filter_is_redundant(
        &self,
        query: &SearchQueryInput,
        segment_ids: &[SegmentId],
    ) -> bool {
        let pruner = self.pruner();
        let mut searched = segment_ids
            .iter()
            .map(|id| {
                self.segment_ordinal_by_id(id)
                    .unwrap_or_else(|| panic!("segment {id} should exist"))
            })
            .filter(|ord| self.is_candidate(*ord))
            .peekable();
        !self.need_scores
            && searched.peek().is_some()
            && searched.all(|ord| pruner.matches_all(ord, query))
    }

    fn pruner(&self) -> SegmentPruner<'_> {
        SegmentPruner::new(
            &self.segment_stats_snapshot,
            &self.schema,
            self.index_created_by_version,
        )
    }

    fn is_candidate(&self, ord: SegmentOrdinal) -> bool {
        self.searcher.segment_reader(ord).num_docs() > 0
            && self.pruner().can_match(ord, &self.search_query_input)
    }

    /// Compiles a tantivy `Weight` for a tagged search query.
    pub fn compile_match_weight(
        &self,
        query_input: &SearchQueryInput,
        need_scores: bool,
    ) -> tantivy::Result<Box<dyn tantivy::query::Weight>> {
        let tantivy_query = self.make_query(query_input, None);
        tantivy_query.weight(enable_scoring(need_scores, self.searcher()))
    }

    /// Count matched docs by summing `Weight::count` across segments.
    ///
    /// For term-like queries (term, or term wrapped in boost/const-score) on
    /// segments without deletes this reads the stored doc_freq from the term
    /// dictionary without touching postings; other queries drain their
    /// docsets without scoring or collection overhead. Counts raw index
    /// entries: no MVCC filtering.
    pub fn count_matched_docs(&self) -> tantivy::Result<u64> {
        let weight = self.weight();
        let mut total = 0u64;
        for (_, segment_reader) in self.candidate_segment_readers() {
            total += u64::from(weight.count(segment_reader)?);
        }
        Ok(total)
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
                self.index_created_by_version,
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

    pub fn index_created_by_version(&self) -> Option<Version> {
        self.index_created_by_version
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

    /// The full frozen view, including segments excluded from this reader's search.
    pub fn segment_readers(&self) -> &[SegmentReader] {
        self.searcher.segment_readers()
    }

    pub fn schema(&self) -> &SearchIndexSchema {
        &self.schema
    }

    /// Collect the visible CTID of every matching document into a memory-bounded [`KeySet`],
    /// resolving HOT chains against the active snapshot before building the set.
    pub fn collect_ctidset(&self, visibility: &mut VisibilityChecker) -> KeySet {
        const VISIBILITY_BATCH_SIZE: usize = 1024;

        let mut search_results = self.search();
        let mut ctid_cache: Option<(SegmentOrdinal, FFType)> = None;
        let mut visible_ctids = Vec::new().into_iter();

        KeySet::build_from(std::iter::from_fn(move || {
            loop {
                if let Some(ctid) = visible_ctids.next() {
                    return Some(
                        TantivyValue::try_from(ctid)
                            .expect("ctid should convert to a Tantivy value"),
                    );
                }

                let ctids: Vec<_> = search_results
                    .by_ref()
                    .take(VISIBILITY_BATCH_SIZE)
                    .map(|(_, doc_address)| {
                        Some(resolve_ctid(&mut ctid_cache, self.searcher(), doc_address))
                    })
                    .collect();
                if ctids.is_empty() {
                    return None;
                }

                let mut resolved = vec![None; ctids.len()];
                visibility.resolve_batch(&ctids, &mut resolved);
                visible_ctids = resolved
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .into_iter();
            }
        }))
    }

    pub fn searcher(&self) -> &Searcher {
        &self.searcher
    }

    /// Returns the total number of segments in the index, according to the MVCC directory.
    pub fn total_segment_count(&self) -> usize {
        self.total_segment_count
    }

    /// Returns the total number of docs in the index, according to the MVCC directory.
    pub fn total_docs(&self) -> u64 {
        self.total_docs
    }

    /// Counts exactly the segments a search of this reader would open.
    pub(crate) fn segment_pruning_estimate(&self) -> SegmentPruningEstimate {
        let (candidate_segments, candidate_docs) = self
            .candidate_segment_readers()
            .fold((0, 0), |(count, docs), (_, reader)| {
                (count + 1, docs + u64::from(reader.num_docs()))
            });
        SegmentPruningEstimate {
            candidate_segments,
            candidate_docs,
        }
    }

    pub(crate) fn segment_stats_snapshot(&self) -> &SegmentStatsSnapshot {
        &self.segment_stats_snapshot
    }

    /// Returns the sort order of the index segments, if the index was created with `sort_by`.
    ///
    /// This reads from the Tantivy index settings stored in the index metadata.
    /// Returns `None` if the index was not created with segment sorting.
    pub fn sort_order(&self) -> Option<SortByField> {
        let settings = self.underlying_index.settings();
        settings.sort_by_field.as_ref().map(|sort_field| {
            let direction = match sort_field.order {
                Order::Asc => SortByDirection::Asc,
                Order::Desc => SortByDirection::Desc,
            };
            SortByField::new(FieldName::from(sort_field.field.clone()), direction)
        })
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
            panic!(
                "failed to create snippet generator for field: {field_name}... can only highlight text fields"
            )
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
        let weight = Arc::new(LazyWeight::new(
            self.query().box_clone(),
            self.need_scores,
            self.searcher.clone(),
        ));
        let iterators = self
            .segment_readers_in_segments(segment_ids)
            .map(|(segment_ord, segment_reader)| {
                ScorerIter::new(
                    DeferredScorer::new(Arc::clone(&weight), segment_reader.clone()),
                    segment_ord,
                    segment_reader.clone(),
                )
            })
            .collect();

        MultiSegmentSearchResults {
            searcher: self.searcher.clone(),
            iterators,
            lazy_iterators: None,
            lazy_estimated_rows: None,
        }
    }

    /// Search all available index segments for matching documents using lazy checkout from the
    /// parallel state to allow load balancing across parallel workers.
    ///
    /// `source_idx = Some(i)` routes to `checkout_segment_for_source(i)` for MPP
    /// non-partitioning sources. `None` uses the single-counter `checkout_segment_for_source(0)` path.
    ///
    /// `estimated_rows` is required because a lazily-evaluated iterator does not inherently know
    /// which or how many segments it will eventually open, and thus cannot compute an accurate
    /// sum of matching documents by asking each segment upfront. It should be passed the value
    /// computed during Postgres query planning.
    pub fn search_lazy(
        &self,
        parallel_state: *mut crate::postgres::ParallelScanState,
        source_idx: Option<usize>,
        estimated_rows: u64,
    ) -> MultiSegmentSearchResults {
        /// Claims segments from the shared scan state and yields the ordinal of each one this
        /// reader would search, deciding per claimed segment so a worker never evaluates
        /// segments it does not own.
        struct ParallelCandidateIterator {
            parallel_state: *mut crate::postgres::ParallelScanState,
            source_idx: Option<usize>,
            reader: SearchIndexReader,
        }
        // SAFETY: the pointer addresses DSM shared memory; the state's mutex serializes
        // every access, including the cross-process claims this iterator drives. The reader
        // holds `Rc` schema state that is only ever touched from this iterator. `Send`/`Sync`
        // are required so DataFusion can wrap the iterator in
        // `Box<dyn Iterator<...> + Send + Sync>` even though the runtime is current-thread.
        unsafe impl Send for ParallelCandidateIterator {}
        unsafe impl Sync for ParallelCandidateIterator {}
        impl Iterator for ParallelCandidateIterator {
            type Item = SegmentOrdinal;
            fn next(&mut self) -> Option<Self::Item> {
                loop {
                    pgrx::check_for_interrupts!();
                    let segment_id = unsafe {
                        match self.source_idx {
                            Some(idx) => (*self.parallel_state).checkout_segment_for_source(idx),
                            None => {
                                crate::postgres::customscan::parallel::checkout_segment_for_source(
                                    self.parallel_state,
                                    0,
                                )
                            }
                        }
                    }?;
                    let ord = self
                        .reader
                        .segment_ordinal_by_id(&segment_id)
                        .unwrap_or_else(|| {
                            panic!(
                                "segment {segment_id} should exist in this {} reader's {} segments",
                                self.reader.directory.mvcc_style_name(),
                                self.reader.searcher.segment_readers().len()
                            )
                        });
                    if self.reader.is_candidate(ord) {
                        return Some(ord);
                    }
                }
            }
        }

        let candidates = ParallelCandidateIterator {
            parallel_state,
            source_idx,
            reader: self.clone(),
        };
        let searcher = self.searcher.clone();
        let weight = Arc::new(LazyWeight::new(
            self.query.box_clone(),
            self.need_scores,
            searcher.clone(),
        ));

        let lazy_iterators = candidates.map(move |segment_ord| {
            let segment_reader = searcher.segment_reader(segment_ord);
            ScorerIter::new(
                DeferredScorer::new(Arc::clone(&weight), segment_reader.clone()),
                segment_ord,
                segment_reader.clone(),
            )
        });

        MultiSegmentSearchResults {
            searcher: self.searcher.clone(),
            iterators: vec![],
            lazy_iterators: Some(Box::new(lazy_iterators)),
            lazy_estimated_rows: Some(estimated_rows),
        }
    }

    /// Search the Tantivy index for "any unordered N" matching documents in specific segments.
    ///
    /// It has no understanding of Postgres MVCC visibility.  It is the caller's responsibility to
    /// handle that, if it's necessary.
    pub fn search_top_k_unordered_in_segments(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
        n: usize,
        offset: usize,
    ) -> TopKSearchResults {
        // Do an un-ordered search.
        TopKSearchResults::new(
            self.search_segments(segment_ids)
                .skip(offset)
                .take(n)
                .collect(),
            None,
        )
    }

    /// Mirrors the sort-shape branch in `search_top_k_in_segments`.
    pub(crate) fn orderby_uses_score_desc_topk_collector(orderby_info: &[OrderByInfo]) -> bool {
        matches!(
            orderby_info.first(),
            Some(OrderByInfo {
                feature: OrderByFeature::Score { .. },
                direction,
            }) if !direction.is_asc()
        )
    }

    /// Search the Tantivy index for the Top K matching documents in specific segments.
    ///
    /// The documents are returned in either score or field order, in the given direction: at least
    /// one `OrderByInfo` must be defined.
    ///
    /// If a TopKAuxiliaryCollector with a vischeck is provided, this method pre-filters for MVCC
    /// visibility. Otherwise — no auxiliary collector, or one whose vischeck is `None` because
    /// MVCC is solved lazily inside its aggregation collector — it is up to the caller to filter
    /// the results for MVCC visibility, and re-query if necessary.
    ///
    /// `parallel_state_holding_shared_threshold` should only be passed if we intend to query with a shared_threshold
    ///
    /// Fruit-side metrics (e.g. vector probe stats) are returned as opaque
    /// per-segment JSON in [`TopKSearch::segment_info`], not bolted onto
    /// [`TopKSearchResults`].
    pub fn search_top_k_in_segments(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
        orderby_info: &[OrderByInfo],
        n: usize,
        offset: usize,
        aux_collector: Option<TopKAuxiliaryCollector>,
        parallel_state_holding_shared_threshold: Option<*mut crate::postgres::ParallelScanState>,
    ) -> TopKSearch {
        let (first_orderby_info, erased_features) = self.prepare_features(orderby_info);
        match first_orderby_info {
            OrderByInfo {
                feature:
                    OrderByFeature::Field {
                        name: sort_field, ..
                    },
                direction,
            } => {
                let field = self
                    .schema
                    .search_field(sort_field)
                    .expect("sort field should exist in index schema");
                let order: ComparatorEnum = (*direction).into();

                macro_rules! sort_fast_value {
                    ($type:ty) => {{
                        let mut computer = SortByStaticFastValue::<$type>::for_field(sort_field);
                        if let Some(state) = parallel_state_holding_shared_threshold {
                            computer = computer.with_shared_threshold(Some(std::sync::Arc::new(
                                crate::postgres::shared_threshold::new_fast_value_threshold(
                                    state,
                                    order.clone(),
                                ),
                            )));
                        }
                        TopKSearchResults::new_for_discarded_field(self.top_in_segments(
                            segment_ids,
                            (computer, order),
                            erased_features,
                            n,
                            offset,
                            aux_collector,
                        ))
                        .into()
                    }};
                }

                // A range is indexed as a tantivy JSON object, so `value_type()` below reports
                // `Type::Json`, which no arm handles — it would reach the catch-all `panic!`.
                // Dispatch on the Postgres type instead: tantivy cannot distinguish a range's
                // JSON from a user-supplied JSON column, and only the former is sortable (see
                // `SearchField::is_sortable`). `SortByRange` compares the bound sub-columns the
                // way Postgres' `range_cmp` does.
                if matches!(field.field_type(), SearchFieldType::Range(_)) {
                    return TopKSearchResults::new_for_discarded_field(self.top_in_segments(
                        segment_ids,
                        (SortByRange::for_field(sort_field), order),
                        erased_features,
                        n,
                        offset,
                        aux_collector,
                    ))
                    .into();
                }

                match field.field_entry().field_type().value_type() {
                    tantivy::schema::Type::Str => {
                        TopKSearchResults::new_for_discarded_field(self.top_in_segments(
                            segment_ids,
                            (SortByString::for_field(sort_field), order),
                            erased_features,
                            n,
                            offset,
                            aux_collector,
                        ))
                        .into()
                    }
                    tantivy::schema::Type::U64 => sort_fast_value!(u64),
                    tantivy::schema::Type::I64 => sort_fast_value!(i64),
                    tantivy::schema::Type::F64 => sort_fast_value!(f64),
                    tantivy::schema::Type::Bool => sort_fast_value!(bool),
                    tantivy::schema::Type::Date => sort_fast_value!(DateTime),
                    tantivy::schema::Type::Bytes => {
                        TopKSearchResults::new_for_discarded_field(self.top_in_segments(
                            segment_ids,
                            (SortByBytes::for_field(sort_field), order),
                            erased_features,
                            n,
                            offset,
                            aux_collector,
                        ))
                        .into()
                    }
                    tantivy::schema::Type::Facet => {
                        unimplemented!("Cannot sort by facet field")
                    }
                    x => {
                        // NOTE: This list of supported field types must be synced with
                        // `SearchField::is_sortable`.
                        panic!("Unsupported order-by field type: {x:?}");
                    }
                }
            }
            OrderByInfo {
                feature: OrderByFeature::Var { .. },
                ..
            } => unimplemented!("Sorting by variable is not supported in raw index search"),
            OrderByInfo {
                feature: OrderByFeature::Score { .. },
                direction,
            } if !erased_features.is_empty() => {
                // If we've directly sorted on the score, then we have it available here.
                let order: ComparatorEnum = (*direction).into();
                let mut computer = SortBySimilarityScore::new();
                if let Some(state) = parallel_state_holding_shared_threshold {
                    computer =
                        SortBySimilarityScore::with_shared_threshold(Some(std::sync::Arc::new(
                            crate::postgres::shared_threshold::new_score_threshold(state),
                        )));
                }
                let (top_docs, aggregation_results) = self.top_in_segments(
                    segment_ids,
                    (computer, order),
                    erased_features,
                    n,
                    offset,
                    aux_collector,
                );
                TopKSearchResults::new_for_score(
                    top_docs.into_iter().map(|((f, _), doc)| (f, doc)),
                    aggregation_results,
                )
                .into()
            }
            OrderByInfo {
                feature: OrderByFeature::Score { .. },
                direction,
            } => self
                .top_by_score_in_segments(
                    segment_ids,
                    *direction,
                    n,
                    offset,
                    aux_collector,
                    parallel_state_holding_shared_threshold,
                )
                .into(),
            OrderByInfo {
                feature: OrderByFeature::NullTest { .. },
                ..
            } => unreachable!("NullTest ORDER BY is only used in JoinScan"),
            OrderByInfo {
                feature: OrderByFeature::ScoreSum { .. },
                ..
            } => unreachable!("ScoreSum ORDER BY is only used in JoinScan"),
            OrderByInfo {
                feature:
                    OrderByFeature::VectorDistance {
                        name, query_vector, ..
                    },
                ..
            } => {
                if orderby_info[1..].iter().any(|o| o.is_score()) {
                    panic!(
                        "pdb.score() cannot tie-break a vector distance ORDER BY: no score is computed when ordering by a vector field"
                    );
                }
                let field = self
                    .schema
                    .search_field(name)
                    .expect("vector field should exist in index schema");
                let tantivy_field = field.field();
                let query_vector = query_vector
                    .resolved()
                    .expect("vector ORDER BY query vector was never resolved")
                    .to_vec();
                // Testing knob: push the GUC's work-model open cost into
                // tantivy so this search's probe budget reflects it.
                tantivy::vector::set_fixed_probe_cost_rows(
                    crate::gucs::vector_fixed_probe_cost_rows(),
                );
                let collector = TopDocs::with_limit(n)
                    .and_offset(offset)
                    .order_by_similarity(tantivy_field, query_vector)
                    .with_adaptive_params(AdaptiveProbeParams {
                        max_probe_fraction: crate::gucs::vector_cluster_max_probe(),
                        ..Default::default()
                    });

                let mut erased_features = erased_features;
                let score_index = erased_features.score_index();
                let mut tie_breaks = Vec::with_capacity(erased_features.len());
                while let Some(feature) = erased_features.pop() {
                    tie_breaks.push(feature);
                }
                tie_breaks.reverse();
                if let Some(i) = score_index {
                    tie_breaks.remove(i);
                }

                // Record only collected candidates, in the same lazy order as their fruits.
                // Rejected segment IDs have no corresponding ProbeStats entry.
                let collected_ids = std::cell::RefCell::new(Vec::new());
                let readers =
                    self.segment_readers_in_segments(segment_ids)
                        .inspect(|(_, reader)| {
                            collected_ids.borrow_mut().push(reader.segment_id());
                        });
                // Fruit is `VectorSimilarityFruit` — hits plus per-segment
                // ProbeStats — for every tie-break shape.
                let tie_break_count = tie_breaks.len();
                let mut tie_breaks = tie_breaks.into_iter();
                let mut next = || tie_breaks.next().expect("tie-break feature should exist");
                let (fruit, aggregation_results) = match tie_break_count {
                    0 => self.collect_maybe_auxiliary(readers, collector, aux_collector),
                    1 => self.collect_maybe_auxiliary(
                        readers,
                        collector.with_tie_break(next()),
                        aux_collector,
                    ),
                    2 => self.collect_maybe_auxiliary(
                        readers,
                        collector.with_tie_break((next(), next())),
                        aux_collector,
                    ),
                    3 => self.collect_maybe_auxiliary(
                        readers,
                        collector.with_tie_break((next(), next(), next())),
                        aux_collector,
                    ),
                    4 => self.collect_maybe_auxiliary(
                        readers,
                        collector.with_tie_break((next(), next(), next(), next())),
                        aux_collector,
                    ),
                    x => panic!(
                        "Unsupported sort-field count: {}. At most {MAX_TOPK_FEATURES} are supported.",
                        x + 1
                    ),
                };
                let segment_ids = collected_ids.into_inner();
                let mut segment_info = probe_stats_to_segment_info(&segment_ids, &fruit.stats);
                io_stats::attach(&mut segment_info);
                TopKSearch::with_segment_info(
                    TopKSearchResults::new_for_score(fruit.results, aggregation_results),
                    segment_info,
                )
            }
        }
    }

    /// Called by `search_top_k_in_segments`.
    ///
    /// `search_top_k_in_segments` is specialized for all combinations of:
    /// 1. first sort field type -- via the generic `S: SortKeyComputer` parameter of this method. This
    ///    gets us unboxed/optimized comparison for the first feature, which always receives more
    ///    comparison than the remaining features (sometimes a lot more).
    /// 2. supported sort field counts (from 1 to MAX_TOPK_FEATURES) -- by calls to
    ///    `top_for_orderable_in_segments` for varying tuple lengths. Ordering on tuples is what is
    ///    supported by `TopDocs::order_by`, because it avoids allocation, and allows for the most
    ///    inlining of comparisons.
    ///
    /// To avoid a combinatorial explosion of generated code we do not support specializing more
    /// than the first sort field type: to do so, we'd likely need a macro which generated all
    /// possible permutations of `S: SortKeyComputer` types for three columns (which would be 7^3=343 copies
    /// of the method at time of writing).
    #[allow(clippy::type_complexity)]
    fn top_in_segments<S>(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
        first_feature: S,
        mut erased_features: ErasedFeatures,
        n: usize,
        offset: usize,
        aux_collector: Option<TopKAuxiliaryCollector>,
    ) -> TopKWithAggregate<S::SortKey>
    where
        S: SortKeyComputer + Clone + Send + 'static,
    {
        let readers = self.segment_readers_in_segments(segment_ids);
        // if last erased feature is score, then we need to return the score
        match erased_features.len() {
            0 => {
                let top_docs_collector = TopDocs::with_limit(n)
                    .and_offset(offset)
                    .order_by::<S::SortKey>(first_feature);

                let (top_docs, aggregation_results) =
                    self.collect_maybe_auxiliary(readers, top_docs_collector, aux_collector);

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
                    self.collect_maybe_auxiliary(readers, top_docs_collector, aux_collector);

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
                    self.collect_maybe_auxiliary(readers, top_docs_collector, aux_collector);

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
            3 => {
                let erased_feature3 = erased_features.pop().unwrap();
                let erased_feature2 = erased_features.pop().unwrap();
                let erased_feature1 = erased_features.pop().unwrap();
                let top_docs_collector = TopDocs::with_limit(n).and_offset(offset).order_by((
                    first_feature,
                    erased_feature1,
                    erased_feature2,
                    erased_feature3,
                ));

                let (top_docs, aggregation_results) =
                    self.collect_maybe_auxiliary(readers, top_docs_collector, aux_collector);

                (
                    top_docs
                        .into_iter()
                        .map(|((f, erased1, erased2, erased3), doc)| {
                            let maybe_score =
                                erased_features.try_get_score(&[erased1, erased2, erased3]);
                            ((f, maybe_score), doc)
                        })
                        .collect(),
                    aggregation_results,
                )
            }
            4 => {
                let erased_feature4 = erased_features.pop().unwrap();
                let erased_feature3 = erased_features.pop().unwrap();
                let erased_feature2 = erased_features.pop().unwrap();
                let erased_feature1 = erased_features.pop().unwrap();
                let top_docs_collector = TopDocs::with_limit(n).and_offset(offset).order_by((
                    first_feature,
                    erased_feature1,
                    erased_feature2,
                    erased_feature3,
                    erased_feature4,
                ));

                let (top_docs, aggregation_results) =
                    self.collect_maybe_auxiliary(readers, top_docs_collector, aux_collector);

                (
                    top_docs
                        .into_iter()
                        .map(|((f, erased1, erased2, erased3, erased4), doc)| {
                            let maybe_score = erased_features
                                .try_get_score(&[erased1, erased2, erased3, erased4]);
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
                        x,
                        MAX_TOPK_FEATURES - 1
                    )
                } else {
                    panic!(
                        "Unsupported sort-field count: {}. At most {MAX_TOPK_FEATURES} are supported.",
                        x + 1,
                    )
                }
            }
        }
    }

    /// Order by score only.
    ///
    /// NOTE: This is a special case for a single score feature: the score-only codepath is highly
    /// specialized due to Block-WAND, and at least 15% faster than `TopDocs::order_by` when
    /// sorting on only the score. We should try to close that gap over time, but for now we
    /// special case it.
    ///
    /// NOTE: Scores cannot be NULL, so we do not need to differentiate the nulls-first/last cases.
    ///
    /// `parallel_state_holding_shared_threshold` should only be passed if we intend to query with a shared_threshold
    fn top_by_score_in_segments(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
        sortdir: SortDirection,
        n: usize,
        offset: usize,
        aux_collector: Option<TopKAuxiliaryCollector>,
        parallel_state_holding_shared_threshold: Option<*mut crate::postgres::ParallelScanState>,
    ) -> TopKSearchResults {
        // NOTE: which `sortdir` arm uses the Block-WAND pruning collector below
        // (only Desc, via `order_by::<Score>`) defines
        // `orderby_uses_score_desc_topk_collector` -- the plan-time gate that costs
        // ordered TopK as serial-vs-parallel (#4664). If you change which direction
        // prunes here, update that predicate to match, or the planner will mis-cost.
        let readers = self.segment_readers_in_segments(segment_ids);
        match sortdir {
            // requires tweaking the score, which is a bit slower
            SortDirection::AscNullsFirst | SortDirection::AscNullsLast => {
                let top_docs_collector = TopDocs::with_limit(n).and_offset(offset).tweak_score(
                    move |_segment_reader: &tantivy::SegmentReader| {
                        move |_doc: DocId, original_score: Score| AscendingScore {
                            score: original_score,
                        }
                    },
                );

                let (top_docs, aggregation_results) =
                    self.collect_maybe_auxiliary(readers, top_docs_collector, aux_collector);

                TopKSearchResults::new_for_score(
                    top_docs
                        .into_iter()
                        .map(|(score, doc_address)| (score.score, doc_address)),
                    aggregation_results,
                )
            }

            // can use tantivy's score directly, which allows for Block-WAND
            SortDirection::DescNullsFirst | SortDirection::DescNullsLast => {
                let mut computer = SortBySimilarityScore::new();
                if let Some(state) = parallel_state_holding_shared_threshold {
                    computer =
                        SortBySimilarityScore::with_shared_threshold(Some(std::sync::Arc::new(
                            crate::postgres::shared_threshold::new_score_threshold(state),
                        )));
                }

                let top_docs_collector = TopDocs::with_limit(n)
                    .and_offset(offset)
                    .order_by::<Score>(computer);

                let (top_docs, aggregation_results) =
                    self.collect_maybe_auxiliary(readers, top_docs_collector, aux_collector);

                TopKSearchResults::new_for_score(top_docs, aggregation_results)
            }
        }
    }

    /// Given an estimate of the total number of rows in the relation, return estimates of:
    /// 1. The number of rows which will be matched by the configured query.
    /// 2. The total number of rows in the index (estimated if total_docs is Unknown).
    /// 3. Tantivy's relative cost to drive the configured query's docset.
    ///
    /// Expects to be called using an index opened with `MvccSatisfies::LargestSegment`, and thus
    /// to contain exactly 0 or 1 Segment.
    pub fn estimate_docs(&self, total_docs: RowEstimate) -> DocsEstimate {
        match self.searcher.segment_readers().len() {
            1 => {}
            0 => {
                return DocsEstimate {
                    matching_docs: 0,
                    total_docs: 0,
                    query_cost: 0,
                };
            }
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
        let mut cost = scorer.cost();
        if count == 0 {
            // but when it doesn't, we need to do a full count
            count = scorer.count_including_deleted() as usize;
            cost = cost.max(count as u64);
        }
        if let Some(shortest_posting_list) = self.shortest_posting_list(largest_reader) {
            cost = cost.max(shortest_posting_list);
        }

        // When the caller's total is unknown or 0 we can't use the heap
        // proportion, so fall back to the index's own doc count. Either way the
        // largest segment is then scaled up to that total.
        let total_docs = match total_docs {
            RowEstimate::Known(total_docs) if total_docs > 0 => total_docs,
            _ => self.total_docs(),
        };
        let segment_doc_proportion = largest_reader.num_docs() as f64 / total_docs as f64;
        DocsEstimate {
            matching_docs: scale_largest_segment_estimate(count as u64, segment_doc_proportion)
                as usize,
            total_docs,
            query_cost: scale_largest_segment_estimate(cost, segment_doc_proportion),
        }
    }

    /// The length of the shortest posting list this query walks for its positional terms, or
    /// `None` when it has none. Lengths come from the term dictionary, so no postings are decoded.
    ///
    /// This floors what driving a phrase costs. Tantivy derives a phrase's cost from an
    /// intersection estimate that assumes the terms are independent, and the words of a phrase are
    /// anything but. The estimate shrinks with every term added while the scan keeps walking the
    /// same posting list, until a top-K over a common phrase looks cheap enough to leave serial.
    /// The scan still advances its cheapest list end to end and seeks the others in step, so it
    /// can never touch fewer documents than that list holds.
    ///
    /// A leaf that exposes no positional terms, a range or a plain term for one, never lowers the
    /// floor. A filter selective enough to drive a phrase scan itself therefore reads as more work
    /// than it is, costing one parallel setup. The under-count it replaces costs a whole-docset
    /// scan on a single core.
    fn shortest_posting_list(&self, segment_reader: &SegmentReader) -> Option<u64> {
        /// Past this many terms a query is a set union, not a conjunction, and a union already
        /// costs more than any one of its lists. Reading the rest would only spend plan time.
        const MAX_TERMS_INSPECTED: usize = 64;

        // Only the queries that need positions cost what they do because of the intersection
        // estimate, so only they need the floor. Everything else already reports the driving list
        // and would pay for a dictionary lookup that cannot change the answer.
        //
        // Each query reports the terms it holds whatever field it is asked about, so one pass
        // collects them. Asking per field would re-walk the tree once per column, and a proximity
        // clause would re-expand its regex every time.
        let (any_field, _) = self.schema.fields().next()?;
        let mut terms: HashSet<Term> = HashSet::default();
        self.query.query_terms(
            any_field,
            segment_reader,
            &mut |term: &Term, needs_positions| {
                if needs_positions && terms.len() < MAX_TERMS_INSPECTED {
                    terms.insert(term.clone());
                }
            },
        );

        terms
            .iter()
            .filter_map(|term| {
                let inverted_index = segment_reader.inverted_index(term.field()).ok()?;
                inverted_index.doc_freq(term).ok().map(u64::from)
            })
            .min()
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
            self.index_created_by_version,
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
            && let Some(child_estimate) = node.children()[0].estimated_docs
        {
            node.set_estimate(child_estimate);
            return;
        }

        let tantivy_query = node
            .query
            .clone()
            .into_tantivy_query(
                &self.schema,
                self.index_created_by_version,
                parser,
                &self.searcher,
                self.index_rel.oid(),
                self.index_rel.rel_oid(),
                None,
                None,
            )
            .expect("converting query for estimation should not fail");

        let weight = tantivy_query
            .weight(enable_scoring(node.query.need_scores(), &self.searcher))
            .expect("creating weight for estimation should not fail");

        let mut scorer = weight
            .scorer(largest_reader, 1.0)
            .expect("creating scorer for estimation should not fail");

        let mut count = scorer.size_hint() as usize;
        if count == 0 {
            count = scorer.count_including_deleted() as usize;
        }

        let estimated =
            scale_largest_segment_estimate(count as u64, segment_doc_proportion) as usize;

        node.set_estimate(estimated);
    }

    pub fn collect<C: Collector>(&self, collector: C) -> C::Fruit {
        // Tantivy's `search` validates the collector before any segment is visited; do the same,
        // so a collector over an unknown field fails even when every segment is pruned.
        collector
            .check_schema(self.searcher.schema())
            .expect("search should not fail");
        let weight = self
            .query
            .weight(enable_scoring(self.need_scores, &self.searcher))
            .expect("creating a Weight from a Query should not fail");
        let fruits = self.collect_segment_readers(
            self.candidate_segment_readers(),
            &collector,
            weight.as_ref(),
        );
        collector
            .merge_fruits(fruits)
            .expect("search should not fail")
    }

    /// Collect for the given Collector, optionally paired with / wrapped with the given auxiliary
    /// Collector(s).
    fn collect_maybe_auxiliary<'a, C: Collector>(
        &self,
        readers: impl Iterator<Item = (SegmentOrdinal, &'a SegmentReader)>,
        top_docs_collector: C,
        aux_collector: Option<TopKAuxiliaryCollector>,
    ) -> (C::Fruit, Option<IntermediateAggregationResults>) {
        let query = self.query();
        let weight = query
            .weight(enable_scoring(self.need_scores, &self.searcher))
            .expect("creating a Weight from a Query should not fail");

        let Some(aux_collector) = aux_collector else {
            // No auxiliary collector.
            let fruits =
                self.collect_segment_readers(readers, &top_docs_collector, weight.as_ref());
            let top_docs = top_docs_collector
                .merge_fruits(fruits)
                .expect("should be able to merge Top K in segments");
            return (top_docs, None);
        };

        // We are executing a compound / tuple collection with an aggregation.
        let compound_collector = (top_docs_collector, aux_collector.aggregation_collector);

        // Optionally wrap in MVCC visibility filtering, if requested.
        if let Some(vischeck) = aux_collector.vischeck {
            let collector = MVCCFilterCollector::new(compound_collector, vischeck);
            let fruits = self.collect_segment_readers(readers, &collector, weight.as_ref());
            let (top_docs, aggregation_results) = collector
                .merge_fruits(fruits)
                .expect("should be able to merge Top K in segment");
            (top_docs, Some(aggregation_results))
        } else {
            let fruits =
                self.collect_segment_readers(readers, &compound_collector, weight.as_ref());
            let (top_docs, aggregation_results) = compound_collector
                .merge_fruits(fruits)
                .expect("should be able to merge Top K in segment");
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
                    feature:
                        OrderByFeature::Field {
                            name: sort_field, ..
                        },
                    direction,
                } => {
                    // NOTE: The list of supported field types for `SortByErasedType` must be synced with
                    // `SearchField::is_sortable`, except Range: `is_sortable` accepts it, but only as
                    // the leading key, which `sortable_at_position` enforces before we get here.
                    erased_features
                        .push_feature(SortByErasedType::for_field(sort_field), *direction);
                }
                OrderByInfo {
                    feature: OrderByFeature::Score { .. },
                    direction,
                } => {
                    erased_features.push_score_feature(*direction);
                }
                OrderByInfo {
                    feature: OrderByFeature::Var { .. },
                    ..
                } => unimplemented!("Sorting by variable is not supported in raw index search"),
                OrderByInfo {
                    feature: OrderByFeature::NullTest { .. },
                    ..
                } => unreachable!("NullTest ORDER BY is only used in JoinScan"),
                OrderByInfo {
                    feature: OrderByFeature::ScoreSum { .. },
                    ..
                } => unreachable!("ScoreSum ORDER BY is only used in JoinScan"),
                OrderByInfo {
                    feature: OrderByFeature::VectorDistance { .. },
                    ..
                } => {
                    // Vector distance cannot be a secondary sort key
                    unimplemented!("Vector distance ORDER BY can only be the primary sort key")
                }
            }
        }

        // if we need scores, but there's no score feature in the order by list,
        // we push an erased score feature to the end of the list for the purpose of holding scores
        if self.need_scores
            && erased_features.score_index().is_none()
            && !first_orderby_info.is_score()
        {
            erased_features.push_score_feature(SortDirection::DescNullsFirst);
        }

        (first_orderby_info, erased_features)
    }

    fn segment_ordinal_by_id(&self, segment_id: &SegmentId) -> Option<SegmentOrdinal> {
        self.segment_stats_snapshot
            .segment_index(*segment_id)
            .map(|ord| ord as SegmentOrdinal)
    }

    fn candidates(
        &self,
        ordinals: impl Iterator<Item = SegmentOrdinal>,
    ) -> impl Iterator<Item = (SegmentOrdinal, &SegmentReader)> {
        ordinals
            .filter(move |ord| self.is_candidate(*ord))
            .map(move |ord| (ord, self.searcher.segment_reader(ord)))
    }

    fn candidate_segment_readers(&self) -> impl Iterator<Item = (SegmentOrdinal, &SegmentReader)> {
        self.candidates(0..self.searcher.segment_readers().len() as SegmentOrdinal)
    }

    /// NOTE: It is very important that this method consumes the input SegmentIds lazily, because
    /// some callers (the Top K exec method in particular) are producing them lazily by checking
    /// them out of shared mutable state as they go.
    fn segment_readers_in_segments(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
    ) -> impl Iterator<Item = (SegmentOrdinal, &SegmentReader)> {
        self.candidates(segment_ids.map(move |segment_id| {
            self.segment_ordinal_by_id(&segment_id).unwrap_or_else(|| {
                panic!(
                    "segment {segment_id} should exist in this {} reader's {} segments",
                    self.directory.mvcc_style_name(),
                    self.searcher.segment_readers().len()
                )
            })
        }))
    }

    fn collect_segment_readers<'a, C: Collector>(
        &self,
        readers: impl Iterator<Item = (SegmentOrdinal, &'a SegmentReader)>,
        collector: &C,
        weight: &dyn Weight,
    ) -> Vec<<<C as Collector>::Child as SegmentCollector>::Fruit> {
        io_stats::reset();
        readers
            .map(|(segment_ord, segment_reader)| {
                let fruit = collector
                    .collect_segment(weight, segment_ord, segment_reader)
                    .expect("should be able to collect in segment");
                io_stats::end_segment(segment_reader.segment_id());
                fruit
            })
            .collect()
    }
}

/// Shape-only inspection — never reads segment contents. The planning-time
/// gate relies on this to use a one-segment (`LargestSegment`) reader.
impl SearchIndexManifest {
    fn components(&self) -> &IndexComponents {
        &self.0.components
    }

    /// Capture the currently visible segment set without building a search query.
    pub fn capture(index_relation: &PgSearchRelation, mvcc_style: MvccSatisfies) -> Result<Self> {
        let components =
            SearchIndexReader::open_index_components(index_relation, mvcc_style, false)?;
        Ok(Self(Rc::new(SearchIndexManifestInner { components })))
    }

    /// This manifest's segment view, for other readers to replay through
    /// [`MvccSatisfies::ParallelWorker`].
    pub fn segment_view(&self) -> SegmentView {
        SegmentView::capture(
            self.components().searcher.segment_readers(),
            &self.components().directory,
        )
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
    features: Vec<(SortByErasedType, SortDirection)>,
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

    pub fn pop(&mut self) -> Option<(SortByErasedType, ComparatorEnum)> {
        self.features.pop().map(|(s, sort_direction)| {
            let order: ComparatorEnum = sort_direction.into();
            (s, order)
        })
    }

    /// Push a non-score feature.
    pub fn push_feature(&mut self, feature: SortByErasedType, direction: SortDirection) {
        self.features.push((feature, direction));
    }

    /// Push a score feature.
    pub fn push_score_feature(&mut self, direction: SortDirection) {
        self.score_index = Some(self.features.len());
        self.features
            .push((SortByErasedType::for_score(), direction));
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

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::test_support::{INDEX_COMPONENT_OPENS, segmented_index_fixture};
    use super::*;
    use crate::index::segment_pruning::{InjectedStatsFailure, STATS_OPENS, inject_stats_failure};
    use crate::index::stats::SegmentStats;
    use crate::postgres::pdb_owned_value::PdbOwnedValue;
    use crate::query::pdb_query::pdb;
    use crate::scan::range_partitioning::RangePartitioning;
    use pgrx::prelude::*;
    use std::ops::Bound;
    use tantivy::index::SegmentComponent;

    fn range_query(field: &str, lower: i64, upper: i64) -> SearchQueryInput {
        SearchQueryInput::FieldedQuery {
            field: FieldName::from(field),
            query: pdb::Query::Range {
                lower_bound: Bound::Included(PdbOwnedValue::I64(lower)),
                upper_bound: Bound::Included(PdbOwnedValue::I64(upper)),
            },
        }
    }

    fn term_query(field: &str, value: &str) -> SearchQueryInput {
        SearchQueryInput::FieldedQuery {
            field: FieldName::from(field),
            query: pdb::Query::Term {
                value: PdbOwnedValue::Str(value.to_string()),
            },
        }
    }

    fn open_snapshot_reader(
        index_rel: &PgSearchRelation,
        query: SearchQueryInput,
        need_scores: bool,
    ) -> SearchIndexReader {
        SearchIndexReader::open(index_rel, query, need_scores, MvccSatisfies::Snapshot).unwrap()
    }

    fn assert_pruning_matches_tantivy(reader: &SearchIndexReader, expected: usize) {
        use tantivy::collector::{Count, TopDocs};

        // Bypass candidate filtering for the oracle, retaining the identical query and Searcher.
        let query = reader.query();
        assert_eq!(
            reader.searcher().search(query, &Count).unwrap(),
            expected,
            "{query:?}"
        );
        assert_eq!(reader.search().count(), expected, "{query:?}");
        assert_eq!(
            reader.count_matched_docs().unwrap(),
            expected as u64,
            "{query:?}"
        );
        assert_eq!(reader.collect(Count), expected, "{query:?}");
        if reader.need_scores() {
            assert_eq!(
                reader.collect(TopDocs::with_limit(10).order_by_score()),
                reader
                    .searcher()
                    .search(query, &TopDocs::with_limit(10).order_by_score())
                    .unwrap(),
                "{query:?}"
            );
        }
    }

    /// A negative `minimum_should_match` is compiled by pg_search as a huge `usize`; the
    /// predicate-level oracle test covers the Boolean rules, this pins the compiled shape.
    #[pg_test]
    fn negative_minimum_under_negation_keeps_the_query_authoritative() {
        let (index_rel, _) = segmented_index_fixture("pruning_single_boolean", 1, false);
        let inner = SearchQueryInput::Boolean {
            must: vec![],
            should: vec![SearchQueryInput::All],
            must_not: vec![],
            minimum_should_match: Some(-1),
        };
        let query = SearchQueryInput::Boolean {
            must: vec![SearchQueryInput::All],
            should: vec![],
            must_not: vec![inner],
            minimum_should_match: None,
        };
        for scoring in [false, true] {
            let reader = open_snapshot_reader(&index_rel, query.clone(), scoring);
            assert_pruning_matches_tantivy(&reader, 0);
        }
    }

    #[pg_test]
    fn segment_pruning_preserves_unsigned_terms_encoded_as_signed_values() {
        use crate::query::TermInput;

        Spi::run(
            "CREATE TABLE pruning_signed_terms (id bigint PRIMARY KEY, x bigint NOT NULL);
             CREATE INDEX pruning_signed_terms_idx ON pruning_signed_terms USING paradedb (id, x)
             WITH (background_layer_sizes = '0');
             SET paradedb.global_mutable_segment_rows = 0;
             INSERT INTO pruning_signed_terms VALUES (1, -1), (2, -2), (3, '-9223372036854775808');
             RESET paradedb.global_mutable_segment_rows;",
        )
        .unwrap();
        let oid = Spi::get_one::<pg_sys::Oid>("SELECT 'pruning_signed_terms_idx'::regclass::oid")
            .unwrap()
            .unwrap();
        let index_rel = PgSearchRelation::open(oid);
        let field = FieldName::from("x");
        let mut queries = Vec::new();
        for value in [i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX - 1, u64::MAX] {
            let expected = usize::from(value > i64::MAX as u64);
            let value = PdbOwnedValue::U64(value);
            for query in [
                SearchQueryInput::FieldedQuery {
                    field: field.clone(),
                    query: pdb::Query::Term {
                        value: value.clone(),
                    },
                },
                SearchQueryInput::FieldedQuery {
                    field: field.clone(),
                    query: pdb::Query::TermSet {
                        terms: vec![value.clone()],
                    },
                },
                SearchQueryInput::TermSet {
                    terms: vec![TermInput {
                        field: field.clone(),
                        value,
                    }],
                },
            ] {
                queries.push((query, expected));
            }
        }
        // Neither endpoint overflows during range normalization, but both wrap in term encoding.
        queries.push((
            SearchQueryInput::FieldedQuery {
                field,
                query: pdb::Query::Range {
                    lower_bound: Bound::Included(PdbOwnedValue::U64(u64::MAX - 1)),
                    upper_bound: Bound::Excluded(PdbOwnedValue::U64(u64::MAX)),
                },
            },
            1,
        ));
        for (query, expected) in queries {
            for scoring in [false, true] {
                let reader = open_snapshot_reader(&index_rel, query.clone(), scoring);
                let snapshot = reader.segment_stats_snapshot();
                let field = reader.schema().search_field("x").unwrap();
                assert!(snapshot.len() > 0);
                for ordinal in 0..snapshot.len() {
                    assert!(
                        snapshot.empirical(ordinal, &field).is_some(),
                        "must exercise statistics"
                    );
                }
                assert_pruning_matches_tantivy(&reader, expected);
            }
        }
    }

    /// `from_manifest` must reuse the capture's open (zero additional index opens) and must
    /// still install the index's tokenizers: the `Parse` query below matches rows only if
    /// the shared managers now hold them.
    #[pg_test]
    fn from_manifest_reuses_the_captured_open() {
        let (index_rel, _heap) = segmented_index_fixture("manifest_reuse_test", 2, false);
        let manifest = SearchIndexManifest::capture(&index_rel, MvccSatisfies::Snapshot)
            .expect("manifest capture");
        assert!(
            manifest.segment_view().len() >= 2,
            "fixture must span several segments"
        );

        let opens_before = INDEX_COMPONENT_OPENS.load(std::sync::atomic::Ordering::Relaxed);
        let reader = SearchIndexReader::from_manifest(
            &manifest,
            &index_rel,
            SearchQueryInput::Parse {
                query_string: "title:silver".to_string(),
                lenient: None,
                conjunction_mode: None,
            },
            /* need_scores */ false,
            None,
            /* needs_tokenizer_manager */ true,
        )
        .expect("from_manifest");

        assert_eq!(
            INDEX_COMPONENT_OPENS.load(std::sync::atomic::Ordering::Relaxed),
            opens_before,
            "building a reader from a manifest must not open the index again"
        );
        assert_eq!(
            reader.segment_view(),
            manifest.segment_view(),
            "the reader replays the capture's frozen segment set"
        );
        assert_eq!(
            reader.search().count(),
            10,
            "the tokenized query resolves through the registered tokenizers"
        );
        // Registration must reach managers shared with the already-built searcher (execution
        // paths like MoreLikeThis and fast-field normalization look tokenizers up through it,
        // not through the parse-time index handle).
        use tantivy::tokenizer::{RawTokenizer, TextAnalyzer};
        let probe = || TextAnalyzer::from(RawTokenizer::default());
        manifest
            .components()
            .index
            .tokenizers()
            .register("test_probe", probe());
        manifest
            .components()
            .index
            .fast_field_tokenizer()
            .register("test_probe", probe());
        assert!(
            reader
                .searcher()
                .index()
                .tokenizers()
                .get("test_probe")
                .is_some(),
            "registering into the manifest's manager must be visible through the searcher"
        );
        assert!(
            reader
                .searcher()
                .index()
                .fast_field_tokenizer()
                .get("test_probe")
                .is_some(),
            "the fast-field manager must be shared the same way"
        );
    }

    #[pg_test]
    fn static_pruning_skips_scorer_construction() {
        use crate::index::reader::scorer::test_support::SCORERS_OPENED;

        let (index_rel, _heap) = segmented_index_fixture("static_segment_pruning_test", 4, false);
        let query = range_query("id", 1, 10);
        let reader = open_snapshot_reader(&index_rel, query, false);
        assert_eq!(reader.segment_pruning_estimate().candidate_segments, 1);
        SCORERS_OPENED.store(0, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(reader.search().count(), 10);
        assert_eq!(
            SCORERS_OPENED.load(std::sync::atomic::Ordering::Relaxed),
            1,
            "only the candidate segment may open a scorer"
        );
    }

    #[pg_test]
    fn analyzed_text_and_uuid_terms_keep_segments_eligible() {
        Spi::run(
            "CREATE TABLE analyzed_pruning (id bigint PRIMARY KEY, title text NOT NULL, u uuid NOT NULL);
             CREATE INDEX analyzed_pruning_idx ON analyzed_pruning
             USING paradedb (id, (title::pdb.unicode_words('columnar=true')), u)
             WITH (target_segment_count = 8, background_layer_sizes = '0',
                   text_fields = '{\"u\": {\"tokenizer\": {\"type\": \"default\"}, \"fast\": true}}');
             SET paradedb.global_mutable_segment_rows = 0;
             INSERT INTO analyzed_pruning
             SELECT g, 'silver dragon ' || g, '550e8400-e29b-41d4-a716-446655440000'::uuid
             FROM generate_series(1, 10) g;
             INSERT INTO analyzed_pruning
             SELECT g, 'quiet river ' || g, '650e8400-e29b-41d4-a716-446655440000'::uuid
             FROM generate_series(11, 20) g;
             RESET paradedb.global_mutable_segment_rows;",
        ).unwrap();
        unsafe { pgrx::pg_sys::CommandCounterIncrement() };
        let oid = Spi::get_one::<pg_sys::Oid>("SELECT 'analyzed_pruning_idx'::regclass::oid")
            .unwrap()
            .unwrap();
        let index_rel = PgSearchRelation::open(oid);
        let probe = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        assert_eq!(probe.total_segment_count(), 2);
        // UUID keeps its search field type even though its Tantivy field is analyzed text.
        // Both tokens lie outside the whole-value bounds in at least one segment.
        for (name, token, expected) in [("title", "silver", 10), ("u", "e29b", 20)] {
            let field = probe.schema().search_field(name).unwrap();
            let snapshot = probe.segment_stats_snapshot();
            assert!(
                (0..snapshot.len()).all(|ord| snapshot.empirical(ord, &field).is_some()),
                "the analyzed-field guard must run with available whole-value statistics"
            );
            let value = PdbOwnedValue::Str(token.to_string());
            for query in [
                pdb::Query::Term {
                    value: value.clone(),
                },
                pdb::Query::TermSet {
                    terms: vec![value.clone()],
                },
                pdb::Query::Range {
                    lower_bound: Bound::Included(value.clone()),
                    upper_bound: Bound::Included(value),
                },
            ] {
                let reader = open_snapshot_reader(
                    &index_rel,
                    SearchQueryInput::FieldedQuery {
                        field: FieldName::from(name),
                        query,
                    },
                    false,
                );
                assert_eq!(
                    reader.segment_pruning_estimate().candidate_segments,
                    2,
                    "whole-value statistics cannot reject analyzed tokens: {name}"
                );
            }
            let reader = open_snapshot_reader(&index_rel, term_query(name, token), false);
            assert_pruning_matches_tantivy(&reader, expected);
        }
    }

    #[pg_test]
    fn literal_text_terms_still_use_segment_pruning() {
        Spi::run(
            "CREATE TABLE literal_text_pruning_test (
                 id bigint PRIMARY KEY,
                 title text NOT NULL
             );
             CREATE INDEX literal_text_pruning_test_idx
             ON literal_text_pruning_test
             USING paradedb (id, (title::pdb.literal))
             WITH (target_segment_count = 8,
                   background_layer_sizes = '0');
             SET paradedb.global_mutable_segment_rows = 0;
             INSERT INTO literal_text_pruning_test
             SELECT g, 'silver dragon ' || g FROM generate_series(1, 10) g;
             INSERT INTO literal_text_pruning_test
             SELECT g, 'quiet river ' || g FROM generate_series(11, 20) g;
             RESET paradedb.global_mutable_segment_rows;",
        )
        .unwrap();
        unsafe { pgrx::pg_sys::CommandCounterIncrement() };

        let index_oid = Spi::get_one::<pgrx::pg_sys::Oid>(
            "SELECT 'literal_text_pruning_test_idx'::regclass::oid",
        )
        .unwrap()
        .unwrap();
        let index_rel = PgSearchRelation::open(index_oid);
        let query = term_query("title", "silver dragon 1");

        let reader = open_snapshot_reader(&index_rel, query, false);
        assert_eq!(reader.segment_pruning_estimate().candidate_segments, 1);
        assert_eq!(reader.search().count(), 1);
    }

    #[pg_test]
    fn partition_filter_is_omitted_only_when_redundant() {
        use crate::index::stats::segments_for_partition;

        let check_filter = |reader: &SearchIndexReader,
                            bounds: &SearchQueryInput,
                            segment_ids: &[SegmentId],
                            expected: usize,
                            covered: bool| {
            let redundant = reader.partition_filter_is_redundant(bounds, segment_ids);
            assert_eq!(redundant, covered && !reader.need_scores(), "{bounds:?}");
            let scan = if redundant {
                reader.clone()
            } else {
                reader.and_query_input(bounds)
            };
            assert_eq!(
                scan.search_segments(segment_ids.iter().copied()).count(),
                expected,
                "{bounds:?}"
            );
            // The exact query over every segment is the oracle.
            let exact = reader.and_query_input(bounds);
            assert_eq!(exact.search().count(), expected, "{bounds:?}");
            if reader.need_scores() {
                let top_docs = |reader: &SearchIndexReader, ids: Vec<SegmentId>| {
                    let order = [OrderByInfo {
                        feature: OrderByFeature::Score { rti: 0 },
                        direction: SortDirection::DescNullsFirst,
                    }];
                    reader
                        .search_top_k_in_segments(ids.into_iter(), &order, 20, 0, None, None)
                        .results
                        .map(|(score, doc)| ((doc.segment_ord, doc.doc_id), score.bm25))
                        .collect::<BTreeMap<_, _>>()
                };
                assert_eq!(
                    top_docs(&scan, segment_ids.to_vec()),
                    top_docs(&exact, reader.segment_ids()),
                    "{bounds:?}"
                );
            }
        };

        // Fixture ids 1..=20 over two NOT NULL segments holding 1..=10 and 11..=20.
        let (index_rel, _heap) = segmented_index_fixture("contained_range_filter_test", 2, false);
        for scoring in [false, true] {
            let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, scoring);
            for (split_points, partition, expected, covered) in [
                (vec![11], 0, 10, true),
                (vec![11], 1, 10, true),
                (vec![15], 0, 14, false),
                (vec![15], 1, 6, false),
                (vec![100], 1, 0, false),
            ] {
                let partitioning = RangePartitioning {
                    partition_by: FieldName::from("id"),
                    split_points: split_points.into_iter().map(PdbOwnedValue::I64).collect(),
                };
                let segment_ids = segments_for_partition(&reader, &partitioning, partition);
                let bounds = partitioning.partition_bounds(partition);
                check_filter(&reader, &bounds, &segment_ids, expected, covered);
            }
        }

        Spi::run(
            "CREATE TABLE nullable_partition_filter (id bigint PRIMARY KEY, value bigint);
             CREATE INDEX nullable_partition_filter_idx ON nullable_partition_filter
             USING paradedb (id, value)
             WITH (target_segment_count = 8, background_layer_sizes = '0');
             SET paradedb.global_mutable_segment_rows = 0;
             INSERT INTO nullable_partition_filter VALUES (1, 10), (2, 20), (3, NULL);
             RESET paradedb.global_mutable_segment_rows;",
        )
        .unwrap();
        unsafe { pgrx::pg_sys::CommandCounterIncrement() };
        let index_oid = Spi::get_one::<pgrx::pg_sys::Oid>(
            "SELECT 'nullable_partition_filter_idx'::regclass::oid",
        )
        .unwrap()
        .unwrap();
        let index_rel = PgSearchRelation::open(index_oid);
        let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        let field = reader.schema().search_field("value").unwrap();
        assert_eq!(reader.segment_ids().len(), 1);
        assert!(
            reader
                .segment_stats_snapshot()
                .empirical(0, &field)
                .unwrap()
                .nullable
        );
        let bounds = range_query("value", 10, 20);
        check_filter(&reader, &bounds, &reader.segment_ids(), 2, false);
    }

    #[pg_test]
    fn segment_checks_short_circuit_and_share_only_component_opens() {
        use crate::index::segment_pruning::EMPIRICAL_READS;
        use std::sync::atomic::Ordering::Relaxed;

        let (index_rel, _heap) = segmented_index_fixture("direct_segment_checks", 4, true);
        let manifest = SearchIndexManifest::capture(&index_rel, MvccSatisfies::Snapshot).unwrap();
        let query = SearchQueryInput::Boolean {
            must: vec![range_query("id", 1, 30), range_query("id", 5, 25)],
            should: vec![],
            must_not: vec![],
            minimum_should_match: None,
        };
        EMPIRICAL_READS.store(0, Relaxed);
        STATS_OPENS.store(0, Relaxed);
        let reader =
            SearchIndexReader::from_manifest(&manifest, &index_rel, query, false, None, false)
                .unwrap();
        assert_eq!(reader.segment_ids().len(), 5);
        assert_eq!(
            reader.segment_pruning_estimate().candidate_segments,
            4,
            "the mutable segment has no statistics and must remain a candidate"
        );
        assert_eq!(
            EMPIRICAL_READS.load(Relaxed),
            7,
            "three segments need both conjuncts; the fourth stops after the impossible first conjunct"
        );
        assert_pruning_matches_tantivy(&reader, 21);
        let derived = reader.and_query_input(&range_query("id", 11, 20));
        assert_pruning_matches_tantivy(&derived, 10);
        let sibling = SearchIndexReader::from_manifest(
            &manifest,
            &index_rel,
            range_query("id", 31, 40),
            false,
            None,
            false,
        )
        .unwrap();
        assert_pruning_matches_tantivy(&sibling, 10);
        let id = reader.schema().search_field("id").unwrap();
        let range = RangePartitioning {
            partition_by: FieldName::from("id"),
            split_points: vec![PdbOwnedValue::I64(11), PdbOwnedValue::I64(21)],
        }
        .partition_range(1)
        .unwrap();
        assert_eq!(
            reader
                .segment_stats_snapshot()
                .segments_intersecting_partition(&id, &range)
                .count(),
            2,
            "retain the matching immutable segment and the mutable segment without statistics"
        );
        assert_eq!(
            STATS_OPENS.load(Relaxed),
            5,
            "derived readers, manifest siblings and partition routing reuse opened components, including absence"
        );
        let fresh = open_snapshot_reader(&index_rel, range_query("id", 31, 40), false);
        assert_pruning_matches_tantivy(&fresh, 10);
        assert_eq!(
            STATS_OPENS.load(Relaxed),
            10,
            "a new execution view opens its own components"
        );
    }

    #[pg_test]
    fn collect_validates_the_collector_before_pruning() {
        let (index_rel, _heap) = segmented_index_fixture("collector_schema_check", 2, false);
        let reader = open_snapshot_reader(&index_rel, range_query("id", 100, 200), false);
        assert_eq!(
            reader.segment_pruning_estimate().candidate_segments,
            0,
            "the query must prune every segment so no segment collection can raise the error"
        );
        let error = pgrx::PgTryBuilder::new(std::panic::AssertUnwindSafe(|| {
            reader.collect(
                TopDocs::with_limit(1)
                    .order_by(SortByStaticFastValue::<i64>::for_field("no_such_field")),
            );
            None
        }))
        .catch_others(|caught| match caught {
            pgrx::pg_sys::panic::CaughtError::RustPanic { ereport, .. } => {
                Some(ereport.message().to_string())
            }
            other => other.rethrow(),
        })
        .execute()
        .expect("an unknown sort field must fail even with nothing to collect");
        assert!(error.contains("no_such_field"), "{error}");
    }

    #[pg_test]
    fn term_sets_read_each_field_at_most_once_per_segment() {
        use crate::index::segment_pruning::EMPIRICAL_READS;
        use crate::query::TermInput;
        use std::sync::atomic::Ordering::Relaxed;

        Spi::run("CREATE TABLE grouped_term_proofs (id bigint PRIMARY KEY, x bigint NOT NULL, title text NOT NULL);
            CREATE INDEX grouped_term_proofs_idx ON grouped_term_proofs USING paradedb
            (id, x, (title::pdb.unicode_words('columnar=true')))
            WITH (background_layer_sizes = '0');
            SET paradedb.global_mutable_segment_rows = 0;
            INSERT INTO grouped_term_proofs SELECT g, g + 1000, 'alpha beta' FROM generate_series(1, 10) g;
            INSERT INTO grouped_term_proofs SELECT g, g + 1000, 'alpha beta' FROM generate_series(11, 20) g;
            RESET paradedb.global_mutable_segment_rows;").unwrap();
        let oid = Spi::get_one::<pg_sys::Oid>("SELECT 'grouped_term_proofs_idx'::regclass::oid")
            .unwrap()
            .unwrap();
        let index_rel = PgSearchRelation::open(oid);
        let mut same_field = (1000..1512)
            .map(|value| TermInput {
                field: FieldName::from("id"),
                value: PdbOwnedValue::I64(value),
            })
            .collect::<Vec<_>>();
        // Duplicates neither multiply results nor create additional field proof passes.
        same_field.extend([10, 10].map(|value| TermInput {
            field: FieldName::from("id"),
            value: PdbOwnedValue::I64(value),
        }));
        let mut mixed_fields = same_field.clone();
        mixed_fields.push(TermInput {
            field: FieldName::from("x"),
            value: PdbOwnedValue::I64(1011),
        });
        let mut unsupported = mixed_fields.clone();
        unsupported.push(TermInput {
            field: FieldName::from("title"),
            value: PdbOwnedValue::Str("alpha".into()),
        });
        for (terms, expected, candidates, passes) in [
            (vec![], 0, 0, 0),
            (same_field, 1, 1, 1),
            (mixed_fields, 2, 2, 2),
            (unsupported, 20, 2, 2),
        ] {
            for scoring in [false, true] {
                EMPIRICAL_READS.store(0, Relaxed);
                let reader = open_snapshot_reader(
                    &index_rel,
                    SearchQueryInput::TermSet {
                        terms: terms.clone(),
                    },
                    scoring,
                );
                assert_eq!(reader.segment_ids().len(), 2);
                assert_eq!(
                    reader.segment_pruning_estimate().candidate_segments,
                    candidates
                );
                assert!(
                    EMPIRICAL_READS.load(Relaxed) <= passes * 2,
                    "each supported field is read at most once per segment; disjunction can stop earlier"
                );
                if passes == 1 {
                    assert_eq!(
                        EMPIRICAL_READS.load(Relaxed),
                        2,
                        "the single-field case checks both segments despite having hundreds of terms"
                    );
                }
                assert_pruning_matches_tantivy(&reader, expected);
            }
        }
    }

    #[pg_test]
    fn candidate_iteration_keeps_ordinals_and_lazy_checkout() {
        use std::cell::Cell;
        let (index_rel, _heap) = segmented_index_fixture("candidate_iteration", 4, false);
        let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        let checked_out = Cell::new(0);
        let ids = reader.segment_ids();
        let mut readers = reader.segment_readers_in_segments(
            ids.iter()
                .copied()
                .inspect(|_| checked_out.set(checked_out.get() + 1)),
        );
        assert_eq!(checked_out.get(), 0);
        assert_eq!(readers.next().unwrap().0, 0);
        assert_eq!(
            checked_out.get(),
            1,
            "requesting one segment must not consume the remaining work"
        );
        drop(readers);
        let narrowed = open_snapshot_reader(&index_rel, range_query("id", 11, 20), false);
        let weakened = narrowed.and_query(Box::new(tantivy::query::AllQuery));
        assert_eq!(weakened.segment_pruning_estimate().candidate_segments, 1);
        assert_eq!(weakened.search().count(), 10);
    }

    #[pg_test]
    fn vector_topk_reports_only_collected_candidates() {
        use crate::api::QueryVector;
        use crate::vector::metric::VectorMetric;
        Spi::run(
            "CREATE TABLE candidate_vectors (id bigint PRIMARY KEY, embedding vector(3));
             CREATE INDEX candidate_vectors_idx ON candidate_vectors
             USING paradedb (id, embedding vector_l2_ops)
             WITH (target_segment_count = 8, background_layer_sizes = '0');
             SET paradedb.global_mutable_segment_rows = 0;
             INSERT INTO candidate_vectors SELECT g, ARRAY[g::real,1,0]::vector FROM generate_series(1,10) g;
             INSERT INTO candidate_vectors SELECT g, ARRAY[g::real,1,0]::vector FROM generate_series(11,20) g;
             RESET paradedb.global_mutable_segment_rows;",
        ).unwrap();
        unsafe { pgrx::pg_sys::CommandCounterIncrement() };
        let oid = Spi::get_one::<pg_sys::Oid>("SELECT 'candidate_vectors_idx'::regclass::oid")
            .unwrap()
            .unwrap();
        let index_rel = PgSearchRelation::open(oid);
        let base = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        assert_eq!(base.segment_ids().len(), 2);
        for (lo, hi, expected_rows, expected_segments) in [
            (1, 10, 10, 1),
            (11, 20, 10, 1),
            (1, 20, 20, 2),
            (99, 100, 0, 0),
        ] {
            let query = range_query("id", lo, hi);
            for reader in [open_snapshot_reader(&index_rel, query.clone(), false)] {
                for reverse in [false, true] {
                    let mut ids = reader.segment_ids();
                    if reverse {
                        ids.reverse();
                    }
                    for tie_break in [false, true] {
                        let mut order = vec![OrderByInfo {
                            feature: OrderByFeature::VectorDistance {
                                name: FieldName::from("embedding"),
                                rti: 0,
                                query_vector: QueryVector::Resolved(vec![1.0, 0.0, 0.0]),
                                metric: VectorMetric::L2,
                            },
                            direction: SortDirection::AscNullsLast,
                        }];
                        if tie_break {
                            order.push(OrderByInfo {
                                feature: OrderByFeature::Field {
                                    name: FieldName::from("id"),
                                    rti: 0,
                                },
                                direction: SortDirection::AscNullsLast,
                            });
                        }
                        let top = reader.search_top_k_in_segments(
                            ids.iter().copied(),
                            &order,
                            50,
                            0,
                            None,
                            None,
                        );
                        let docs = top.results.map(|(_, doc)| doc).collect::<Vec<_>>();
                        assert_eq!(docs.len(), expected_rows);
                        for doc in &docs {
                            let id = reader
                                .searcher()
                                .segment_reader(doc.segment_ord)
                                .fast_fields()
                                .i64("id")
                                .unwrap()
                                .first(doc.doc_id)
                                .unwrap();
                            assert!((lo..=hi).contains(&id), "excluded segment returned id {id}");
                        }
                        assert_eq!(top.segment_info.len(), expected_segments);
                        let returned_segments = docs
                            .iter()
                            .map(|doc| {
                                reader
                                    .searcher()
                                    .segment_reader(doc.segment_ord)
                                    .segment_id()
                            })
                            .collect::<HashSet<_>>();
                        assert_eq!(
                            top.segment_info.keys().copied().collect::<HashSet<_>>(),
                            returned_segments,
                            "statistics must name the actual returned segments, regardless of input order"
                        );
                    }
                }
            }
        }
    }

    #[pg_test]
    fn statistics_are_read_only_when_a_decision_needs_them() {
        use std::sync::atomic::Ordering::Relaxed;

        let (index_rel, _heap) = segmented_index_fixture("lazy_stats_open_test", 4, false);

        STATS_OPENS.store(0, Relaxed);
        let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        assert_eq!(reader.search().count(), 40);
        assert_eq!(
            STATS_OPENS.load(Relaxed),
            0,
            "a query without a provable predicate must not open any .stats component"
        );

        // Planning estimates open the largest segment only, as the join planner does.
        let estimating = SearchIndexReader::open(
            &index_rel,
            range_query("id", 100, 200),
            false,
            MvccSatisfies::LargestSegment,
        )
        .unwrap();
        estimating.estimate_docs(RowEstimate::Known(40));
        assert_eq!(
            STATS_OPENS.load(Relaxed),
            0,
            "a reader that only estimates must not open statistics"
        );
        assert_eq!(estimating.segment_pruning_estimate().candidate_segments, 0);
        assert_eq!(
            STATS_OPENS.load(Relaxed),
            1,
            "the first search decision opens the segment's statistics"
        );

        STATS_OPENS.store(0, Relaxed);
        let reader = open_snapshot_reader(&index_rel, range_query("id", 1, 10), false);
        assert_eq!(reader.segment_pruning_estimate().candidate_segments, 1);
        assert_eq!(
            STATS_OPENS.load(Relaxed),
            4,
            "a range decision opens each immutable segment's .stats exactly once"
        );
        assert_eq!(reader.search().count(), 10);
        assert_eq!(
            STATS_OPENS.load(Relaxed),
            4,
            "executing the search must not reopen statistics"
        );
    }

    #[pg_test]
    fn unreadable_stats_abort_query() {
        let (index_rel, _heap) = segmented_index_fixture("unreadable_stats_test", 2, false);
        let probe = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        let id = probe.schema().search_field("id").unwrap();
        // No fixture row has an id in this range, so readable statistics rule every segment out.
        let range = RangePartitioning {
            partition_by: FieldName::from("id"),
            split_points: vec![PdbOwnedValue::I64(100), PdbOwnedValue::I64(201)],
        }
        .partition_range(1)
        .unwrap();
        let snapshot = probe.segment_stats_snapshot();
        let segment_ids = probe.segment_ids();
        assert_eq!(segment_ids.len(), 2);
        assert_eq!(
            snapshot
                .segments_intersecting_partition(&id, &range)
                .count(),
            0,
            "readable statistics must reject all segments before failure injection"
        );
        let segment = segment_ids[0];
        for operation in [
            InjectedStatsFailure::Open,
            InjectedStatsFailure::Empirical,
            InjectedStatsFailure::Logical,
        ] {
            let failure = inject_stats_failure(segment, operation);
            let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
            let snapshot = reader.segment_stats_snapshot();
            let expected_message = match operation {
                InjectedStatsFailure::Open => format!(
                    "could not open segment statistics for {segment:?}: injected Open statistics failure"
                ),
                InjectedStatsFailure::Empirical => format!(
                    "could not read empirical statistics for field {:?} in segment {segment:?}: injected Empirical statistics failure",
                    id.field()
                ),
                InjectedStatsFailure::Logical => format!(
                    "could not read logical statistics for field {:?} in segment {segment:?}: injected Logical statistics failure",
                    id.field()
                ),
            };
            // Both consumers of the snapshot raise the same error: partition routing reads every
            // entry, the reader's own candidate decision reads only the empirical one.
            let pruning_reader =
                open_snapshot_reader(&index_rel, range_query("id", 100, 200), false);
            let routing = || {
                snapshot
                    .segments_intersecting_partition(&id, &range)
                    .count();
            };
            let deciding = || {
                pruning_reader.segment_pruning_estimate();
            };
            let mut probes: Vec<&dyn Fn()> = vec![&routing];
            if operation != InjectedStatsFailure::Logical {
                probes.push(&deciding);
            }
            for (attempt, probe) in (1..).zip(probes) {
                let error = pgrx::PgTryBuilder::new(std::panic::AssertUnwindSafe(|| {
                    probe();
                    None
                }))
                .catch_others(|caught| match caught {
                    pgrx::pg_sys::panic::CaughtError::ErrorReport(report) => {
                        Some(report.message().to_string())
                    }
                    other => other.rethrow(),
                })
                .execute();
                assert_eq!(
                    error.as_deref(),
                    Some(expected_message.as_str()),
                    "an unreadable component must raise the specific query error: {operation:?}"
                );
                assert_eq!(
                    failure.hits(),
                    attempt,
                    "each probe must reach the real operation's injected error"
                );
            }
            drop(failure);
            assert_eq!(
                snapshot
                    .segments_intersecting_partition(&id, &range)
                    .count(),
                0,
                "a failed read must not cache absence or make the segment permanently eligible"
            );
        }
    }

    /// Missing statistics retain mutable segments without materializing their in-memory index.
    #[pg_test]
    fn mutable_segments_remain_eligible_without_statistics() {
        let (index_rel, _heap) = segmented_index_fixture("mutable_stats_test", 1, true);
        let directory = MvccSatisfies::Snapshot.directory(&index_rel);
        let index = Index::open(directory.clone()).unwrap();
        let mutable = index
            .searchable_segments()
            .unwrap()
            .into_iter()
            .find(|segment| directory.is_mutable(&segment.id()))
            .expect("fixture must include a mutable segment");
        let mutable_id = mutable.id();
        assert_eq!(
            directory.mutable_segment_materialized(&mutable_id),
            Some(false)
        );
        assert!(SegmentStats::of_segment(&mutable).unwrap().is_none());
        assert_eq!(
            directory.mutable_segment_materialized(&mutable_id),
            Some(false),
            "probing .stats must not build the in-memory index"
        );
        // A component the segment does have must still materialize it.
        mutable.open_read(SegmentComponent::Terms).unwrap();
        assert_eq!(
            directory.mutable_segment_materialized(&mutable_id),
            Some(true)
        );

        let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        let field = reader.schema().search_field("id").unwrap();
        assert_eq!(reader.segment_ids().len(), 2);
        let snapshot = reader.segment_stats_snapshot();
        let ordinal = snapshot.segment_index(mutable_id).unwrap();
        assert!(snapshot.empirical(ordinal, &field).is_none());
        let range = RangePartitioning {
            partition_by: FieldName::from("id"),
            split_points: vec![PdbOwnedValue::I64(100), PdbOwnedValue::I64(201)],
        }
        .partition_range(1)
        .unwrap();
        for _ in 0..2 {
            assert_eq!(
                snapshot
                    .segments_intersecting_partition(&field, &range)
                    .collect::<Vec<_>>(),
                vec![mutable_id],
                "missing statistics retain the mutable segment while readable bounds prune the other"
            );
        }
        assert_eq!(reader.search().count(), 15);
        let impossible = open_snapshot_reader(&index_rel, range_query("id", 100, 200), false);
        assert_eq!(impossible.segment_pruning_estimate().candidate_segments, 1);
        assert_pruning_matches_tantivy(&impossible, 0);
        // The immutable segment fits this range; the mutable one cannot prove coverage.
        let bounds = range_query("id", 1, 10);
        assert!(!reader.partition_filter_is_redundant(&bounds, &reader.segment_ids()));
        assert_pruning_matches_tantivy(&reader.and_query_input(&bounds), 10);
    }

    #[pg_test]
    fn prepared_plan_reopens_the_execution_manifest_after_a_segment_merge() {
        use crate::index::writer::index::{Mergeable, SearchIndexMerger};

        let (index_rel, _heap) = segmented_index_fixture("merge_freshness_pruning_test", 4, false);
        // The regress suite covers new, updated and deleted rows under a generic plan; a merge is
        // the case where the planned segments no longer exist at all.
        Spi::run(
            "SET plan_cache_mode = force_generic_plan;
             PREPARE merge_freshness_query(bigint, bigint) AS
             SELECT count(*)
             FROM merge_freshness_pruning_test
             WHERE title @@@ 'quiet' AND id BETWEEN $1 AND $2;
             EXECUTE merge_freshness_query(11, 20);",
        )
        .unwrap();

        let mut merger =
            SearchIndexMerger::open(&index_rel, MvccSatisfies::Mergeable).expect("open merger");
        let mut segment_ids = merger
            .searchable_segment_ids()
            .expect("mergeable segments")
            .into_iter()
            .collect::<Vec<_>>();
        segment_ids.sort_unstable();
        assert_eq!(
            segment_ids.len(),
            4,
            "fixture must begin with four segments"
        );
        assert!(
            merger
                .merge_segments(&segment_ids)
                .expect("foreground merge")
                .is_some(),
            "the test must replace the original segment generation"
        );
        unsafe { pgrx::pg_sys::CommandCounterIncrement() };

        assert_eq!(
            Spi::get_one::<i64>("EXECUTE merge_freshness_query(21, 30)")
                .unwrap()
                .unwrap(),
            10,
            "execution after the merge must resolve the new segment generation"
        );
        Spi::run("DEALLOCATE merge_freshness_query; RESET plan_cache_mode;").unwrap();
    }
}

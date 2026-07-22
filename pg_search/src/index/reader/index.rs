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
use std::fmt::{Debug, Display};
use std::path::PathBuf;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

use crate::aggregate::mvcc_collector::MVCCFilterCollector;
use crate::api::operator::keyset::KeySet;
use crate::api::version::Version;
use crate::api::{FieldName, HashMap, OrderByFeature, OrderByInfo, SortDirection};
use crate::index::fast_fields_helper::FFHelper;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::scorer::{DeferredScorer, LazyWeight, ScorerIter};
use crate::index::setup_tokenizers;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::options::{SortByDirection, SortByField};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::buffer::PinnedBuffer;
use crate::postgres::storage::metadata::MetaPage;
use crate::query::estimate_tree::QueryWithEstimates;
use crate::query::SearchQueryInput;
use crate::scan::info::RowEstimate;
use crate::schema::SearchIndexSchema;

use anyhow::Result;
use tantivy::aggregation::intermediate_agg_result::IntermediateAggregationResults;
use tantivy::aggregation::DistributedAggregationCollector;
use tantivy::collector::sort_key::{
    ComparatorEnum, SortByBytes, SortByErasedType, SortBySimilarityScore, SortByStaticFastValue,
    SortByString,
};
use tantivy::collector::{Collector, SegmentCollector, SortKeyComputer, TopDocs};
use tantivy::index::{Index, Order, SegmentId};
use tantivy::query::{EnableScoring, QueryClone, QueryParser, Weight};
use tantivy::snippet::SnippetGenerator;
use tantivy::vector::ivf::AdaptiveProbeParams;
use tantivy::vector::{ProbeStats, ProbeTermination};
use tantivy::{
    query::Query, schema::OwnedValue, DateTime, DocAddress, DocId, DocSet, Executor, IndexReader,
    ReloadPolicy, Score, Searcher, SegmentOrdinal, SegmentReader, TantivyDocument,
};

/// The maximum number of sort-features/`OrderByInfo`s supported for
/// `SearchIndexReader::search_top_k_in_segments`.
pub const MAX_TOPK_FEATURES: usize = 5;

/// Aggregate the per-segment IVF [`ProbeStats`] of one vector query into a
/// single parseable `probe_stats …` line. Scalars sum across segments;
/// `termination` is tallied per variant (so a `Ceiling` on any segment stays
/// visible). The summed line preserves the per-segment invariant
/// `visited == pruned_filter + pruned_dead + pruned_seen + scored`.
///
/// Line-grammar note: tantivy's `ProbeStats::routing.visited_count` is
/// surfaced as `router_scored=`. It counts centroids the router actually
/// scored — the beam's visit set when routing went through the persisted
/// RNG, or all of them on the linear fallback.
///
/// The two trailing compound tokens reuse `termination=`'s `key:N` comma
/// style and sit at the end of the line so prefix parsers are unaffected.
/// They always print, zeros included — the grammar is constant-shape.
/// `postings=` buckets probed IVF clusters by posting-fetch outcome (one
/// stride-sized read per surviving row / skipped outright when the gate
/// pre-pass leaves zero survivors); the two buckets partition the probed
/// clusters, so `row + skipped == clusters_probed` — asserted in the unit
/// tests. `exact_rows=` counts flat-path stride-sized row reads (one per
/// survivor scored). At 0% selectivity the empty-filter short-circuit
/// returns before the probe loop, so every posting counter is zero by
/// design and the partition invariant holds trivially (0 == 0 clusters
/// probed).
fn format_probe_stats(per_segment: &[ProbeStats]) -> String {
    let mut visited = 0usize;
    let mut pruned_filter = 0usize;
    let mut pruned_dead = 0usize;
    let mut pruned_seen = 0usize;
    let mut scored = 0usize;
    let mut clusters_probed = 0usize;
    let mut router_scored = 0usize;
    let mut min_candidates = 0usize;
    let (mut ceiling, mut gate, mut exhausted) = (0usize, 0usize, 0usize);
    let (mut postings_row, mut postings_skipped) = (0usize, 0usize);
    let mut exact_rows = 0usize;
    for s in per_segment {
        visited += s.vectors_visited;
        pruned_filter += s.pruned_filter;
        pruned_dead += s.pruned_dead;
        pruned_seen += s.pruned_seen;
        scored += s.candidates_scored;
        clusters_probed += s.probed_clusters.len();
        router_scored += s.routing.visited_count;
        min_candidates += s.min_candidates;
        match s.termination {
            ProbeTermination::Ceiling => ceiling += 1,
            ProbeTermination::Gate => gate += 1,
            ProbeTermination::Exhausted => exhausted += 1,
        }
        postings_row += s.postings_row;
        postings_skipped += s.postings_skipped;
        exact_rows += s.exact_rows_read;
    }
    format!(
        "probe_stats visited={visited} pruned_filter={pruned_filter} \
         pruned_dead={pruned_dead} pruned_seen={pruned_seen} scored={scored} \
         clusters_probed={clusters_probed} router_scored={router_scored} \
         min_candidates={min_candidates} termination=ceiling:{ceiling},gate:{gate},exhausted:{exhausted} \
         postings=row:{postings_row},skipped:{postings_skipped} \
         exact_rows={exact_rows}"
    )
}

/// Emit the aggregated probe stats as one NOTICE. Only called when
/// `paradedb.log_probe_stats` is on (off by default, zero-cost when off).
fn emit_probe_stats_notice(per_segment: &[ProbeStats]) {
    pgrx::notice!("{}", format_probe_stats(per_segment));
}

#[derive(Debug, Clone, Copy)]
pub struct DocsEstimate {
    pub matching_docs: usize,
    pub total_docs: u64,
    pub query_cost: u64,
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
        if self.iterators.is_empty() {
            if let Some(ref mut lazy) = self.lazy_iterators {
                if let Some(next_iter) = lazy.next() {
                    self.iterators.push(next_iter);
                }
            }
        }
        self.iterators.last_mut()
    }

    pub fn current_segment_pop(&mut self) -> Option<ScorerIter> {
        self.iterators.pop()
    }

    pub fn segment_ids(&self) -> Vec<tantivy::index::SegmentId> {
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
    /// If MVCC filtering should be applied, then the visibility checker to use for that.
    ///
    /// Note: If enabled, visibility checking is applied to _both_ the Top K and to any
    /// aggregation collector: this is because once you've bothered to filter for MVCC, you might
    /// as well feed the filtered result to Top K too.
    pub vischeck: Option<VisibilityChecker>,
}

pub struct SearchIndexReader {
    index_rel: PgSearchRelation,
    searcher: Searcher,
    schema: SearchIndexSchema,
    underlying_reader: IndexReader,
    underlying_index: Index,
    query: Box<dyn Query>,
    need_scores: bool,
    total_segment_count: usize,
    total_docs: u64,
    index_created_by_version: Option<Version>,
    segment_ordinal_by_id: HashMap<SegmentId, SegmentOrdinal>,

    // [`PinnedBuffer`] has a Drop impl, so we hold onto it but don't otherwise use it
    //
    // also, it's an Arc b/c if we're clone'd (we do derive it, after all), we only want this
    // buffer dropped once
    _cleanup_lock: Arc<PinnedBuffer>,
}

/// A queryless snapshot of visible segments used to capture canonical manifests for parallel
/// JoinScan initialization without requiring executor state.
pub struct SearchIndexManifest {
    searcher: Searcher,
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
            total_segment_count: self.total_segment_count,
            total_docs: self.total_docs,
            index_created_by_version: self.index_created_by_version,
            segment_ordinal_by_id: self.segment_ordinal_by_id.clone(),
            _cleanup_lock: self._cleanup_lock.clone(),
        }
    }
}

struct IndexComponents {
    cleanup_lock: Arc<PinnedBuffer>,
    index: Index,
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

        Ok(IndexComponents {
            cleanup_lock,
            index,
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
        let components =
            Self::open_index_components(index_relation, mvcc_style, needs_tokenizer_manager)?;
        let IndexComponents {
            cleanup_lock,
            index,
            reader,
            searcher,
            total_segment_count,
            total_docs,
            schema,
        } = components;

        let index_created_by_version = index_relation.created_by_version();
        let need_scores = need_scores || search_query_input.need_scores();
        let query = {
            search_query_input
                .into_tantivy_query(
                    &schema,
                    index_created_by_version,
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
        let segment_ord_by_id = searcher
            .segment_readers()
            .iter()
            .enumerate()
            .map(|(ord, reader)| (reader.segment_id(), ord as SegmentOrdinal))
            .collect();

        Ok(Self {
            index_rel: index_relation.clone(),
            searcher,
            schema,
            underlying_reader: reader,
            underlying_index: index,
            query,
            need_scores,
            total_segment_count,
            total_docs,
            index_created_by_version,
            segment_ordinal_by_id: segment_ord_by_id,
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

    pub fn need_scores(&self) -> bool {
        self.need_scores
    }

    pub fn query(&self) -> &dyn Query {
        &self.query
    }

    pub fn and_query(&mut self, additional_query: Box<dyn Query>) {
        let existing = std::mem::replace(&mut self.query, Box::new(tantivy::query::EmptyQuery));
        let boolean_query = tantivy::query::BooleanQuery::new(vec![
            (tantivy::query::Occur::Must, existing),
            (tantivy::query::Occur::Must, additional_query),
        ]);
        self.query = Box::new(boolean_query);
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

    pub fn segment_readers(&self) -> &[SegmentReader] {
        self.searcher.segment_readers()
    }

    pub fn schema(&self) -> &SearchIndexSchema {
        &self.schema
    }

    /// Collect the key-field value of every matching document into a memory-bounded [`KeySet`],
    /// which spills to a temporary file once it would exceed `work_mem`.
    pub fn collect_keyset(&self) -> KeySet {
        let key_ff_helper = FFHelper::with_fields(
            self,
            &[(self.schema.key_field_name(), self.schema.key_field_type()).into()],
        );

        KeySet::build_from(self.search().map(|(_, doc_address)| {
            key_ff_helper
                .value(0, doc_address)
                .expect("key_field value should not be null")
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
    /// non-partitioning sources. `None` uses the single-counter `checkout_segment` path.
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
        struct ParallelSegmentIterator {
            parallel_state: *mut crate::postgres::ParallelScanState,
            source_idx: Option<usize>,
        }
        // SAFETY: the pointer addresses DSM shared memory; the state's mutex serializes
        // every access, including the cross-process claims this iterator drives.
        // `Send`/`Sync` are required so DataFusion can wrap the iterator in
        // `Box<dyn Iterator<...> + Send + Sync>` even though the runtime is
        // current-thread.
        unsafe impl Send for ParallelSegmentIterator {}
        unsafe impl Sync for ParallelSegmentIterator {}
        impl Iterator for ParallelSegmentIterator {
            type Item = tantivy::index::SegmentId;
            fn next(&mut self) -> Option<Self::Item> {
                pgrx::check_for_interrupts!();
                unsafe {
                    match self.source_idx {
                        Some(idx) => (*self.parallel_state).checkout_segment_for_source(idx),
                        None => crate::postgres::customscan::parallel::checkout_segment(
                            self.parallel_state,
                        ),
                    }
                }
            }
        }

        let segment_ids = ParallelSegmentIterator {
            parallel_state,
            source_idx,
        };
        let searcher = self.searcher.clone();
        let weight = Arc::new(LazyWeight::new(
            self.query.box_clone(),
            self.need_scores,
            searcher.clone(),
        ));

        let lazy_iterators = segment_ids.map(move |segment_id| {
            let (segment_ord, segment_reader) = searcher
                .segment_readers()
                .iter()
                .enumerate()
                .find(|(_, reader)| reader.segment_id() == segment_id)
                .unwrap_or_else(|| panic!("segment {segment_id} should exist"));
            let segment_ord = segment_ord as SegmentOrdinal;

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
    /// If a TopKAuxiliaryCollector is provided, this method can optionally pre-filter for MVCC
    /// visibility: if a collector is _not_ provided, then it is up to the caller to filter the
    /// results for MVCC visibility, and re-query if necessary.
    ///
    /// `parallel_state_holding_shared_threshold` should only be passed if we intend to query with a shared_threshold
    pub fn search_top_k_in_segments(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
        orderby_info: &[OrderByInfo],
        n: usize,
        offset: usize,
        aux_collector: Option<TopKAuxiliaryCollector>,
        parallel_state_holding_shared_threshold: Option<*mut crate::postgres::ParallelScanState>,
    ) -> TopKSearchResults {
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
                    }};
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
            }
            OrderByInfo {
                feature: OrderByFeature::Score { .. },
                direction,
            } => self.top_by_score_in_segments(
                segment_ids,
                *direction,
                n,
                offset,
                aux_collector,
                parallel_state_holding_shared_threshold,
            ),
            OrderByInfo {
                feature: OrderByFeature::NullTest { .. },
                ..
            } => unreachable!("NullTest ORDER BY is only used in JoinScan"),
            OrderByInfo {
                feature:
                    OrderByFeature::VectorDistance {
                        name, query_vector, ..
                    },
                ..
            } => {
                let only_score_feature =
                    erased_features.len() == 1 && erased_features.score_index() == Some(0);
                if !(erased_features.is_empty() || only_score_feature) {
                    panic!("secondary ORDER BY fields are not supported for vector distance");
                }
                let field = self
                    .schema
                    .search_field(name)
                    .expect("vector field should exist in index schema");
                let tantivy_field = field.field();
                let collector = TopDocs::with_limit(n)
                    .and_offset(offset)
                    .order_by_similarity(tantivy_field, query_vector.clone())
                    .with_adaptive_params(AdaptiveProbeParams {
                        epsilon: crate::gucs::vector_cluster_probe_epsilon(),
                        max_probe_fraction: crate::gucs::vector_cluster_max_probe_fraction(),
                        ..Default::default()
                    });
                // Probe-stats NOTICE (GUC `paradedb.log_probe_stats`, off by
                // default, zero-cost when off). The collector pushes one
                // ProbeStats per segment into the sink; we aggregate the scalars
                // and tally `termination`, then emit a single line. This vector
                // search runs once per scan, so the NOTICE is once per query.
                let probe_stats_sink =
                    crate::gucs::log_probe_stats().then(|| Arc::new(Mutex::new(Vec::new())));
                let collector = match &probe_stats_sink {
                    Some(sink) => collector.with_probe_stats_sink(Arc::clone(sink)),
                    None => collector,
                };
                let (top_docs, aggregation_results) =
                    self.collect_maybe_auxiliary(segment_ids, collector, aux_collector);
                if let Some(sink) = probe_stats_sink {
                    emit_probe_stats_notice(&sink.lock().unwrap());
                }
                TopKSearchResults::new_for_score(top_docs, aggregation_results)
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
                    self.collect_maybe_auxiliary(segment_ids, top_docs_collector, aux_collector);

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
                    self.collect_maybe_auxiliary(segment_ids, top_docs_collector, aux_collector);

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
                        x, MAX_TOPK_FEATURES - 1
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
                    self.collect_maybe_auxiliary(segment_ids, top_docs_collector, aux_collector);

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
                    self.collect_maybe_auxiliary(segment_ids, top_docs_collector, aux_collector);

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
                }
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
        aux_collector: Option<TopKAuxiliaryCollector>,
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
                .expect("should be able to merge Top K in segments");
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
                .expect("should be able to merge Top K in segment");
            (top_docs, Some(aggregation_results))
        } else {
            let fruits = self.collect_segments(segment_ids, &compound_collector, weight.as_ref());
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
                    // `SearchField::is_sortable`.
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
        self.segment_ordinal_by_id.get(segment_id).copied()
    }

    /// NOTE: It is very important that this method consumes the input SegmentIds lazily, because
    /// some callers (the Top K exec method in particular) are producing them lazily by checking
    /// them out of shared mutable state as they go.
    fn segment_readers_in_segments(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
    ) -> impl Iterator<Item = (SegmentOrdinal, &SegmentReader)> {
        segment_ids.map(|segment_id| {
            let ord = self
                .segment_ordinal_by_id(&segment_id)
                .unwrap_or_else(|| panic!("segment {segment_id} should exist"));
            (ord, self.searcher.segment_reader(ord))
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

/// Shape-only inspection — never reads segment contents. The planning-time
/// gate relies on this to use a one-segment (`LargestSegment`) reader.
impl SearchIndexManifest {
    /// Capture the currently visible segment set without building a search query.
    pub fn capture(index_relation: &PgSearchRelation, mvcc_style: MvccSatisfies) -> Result<Self> {
        let components =
            SearchIndexReader::open_index_components(index_relation, mvcc_style, false)?;
        Ok(Self {
            searcher: components.searcher,
            _cleanup_lock: components.cleanup_lock,
        })
    }

    pub fn segment_readers(&self) -> &[SegmentReader] {
        self.searcher.segment_readers()
    }

    pub fn segment_count(&self) -> usize {
        self.searcher.segment_readers().len()
    }

    /// Total live document count across all visible segments. Used by MPP
    /// to pick the partitioning source — the source whose row count makes
    /// it most worth slicing N ways. Defers to `Searcher::num_docs`, which
    /// is the canonical `max_doc - num_deleted` sum.
    pub fn total_doc_count(&self) -> u64 {
        self.searcher.num_docs()
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

#[cfg(test)]
mod probe_stats_tests {
    use super::*;
    use tantivy::vector::IvfSearchMetrics;

    fn seg(termination: ProbeTermination) -> ProbeStats {
        // visited (25) == pruned_filter(5) + pruned_dead(2) + pruned_seen(8) + scored(10);
        // postings row(2) + skipped(1) == probed_clusters.len() (3).
        ProbeStats {
            probed_clusters: vec![0, 1, 2],
            candidates_scored: 10,
            vectors_visited: 25,
            pruned_filter: 5,
            pruned_dead: 2,
            pruned_seen: 8,
            postings_row: 2,
            postings_skipped: 1,
            exact_rows_read: 0,
            routing: IvfSearchMetrics {
                visited_count: 9,
                graph: None,
            },
            min_candidates: 16,
            termination,
        }
    }

    /// Pull `key=N` off the formatted line.
    fn scalar(line: &str, key: &str) -> usize {
        let tail = line.split(&format!("{key}=")).nth(1).unwrap();
        tail.split(' ').next().unwrap().parse().unwrap()
    }

    /// Pull `sub:N` out of a `key=a:N,b:N,…` compound token.
    fn compound(line: &str, key: &str, sub: &str) -> usize {
        let tail = line.split(&format!("{key}=")).nth(1).unwrap();
        let token = tail.split(' ').next().unwrap();
        let entry = token
            .split(',')
            .find(|e| e.starts_with(&format!("{sub}:")))
            .unwrap();
        entry.split(':').nth(1).unwrap().parse().unwrap()
    }

    #[test]
    fn format_probe_stats_sums_scalars_and_tallies_termination() {
        let line =
            format_probe_stats(&[seg(ProbeTermination::Gate), seg(ProbeTermination::Ceiling)]);
        // Scalars summed; termination tallied per variant; summed invariant
        // holds (visited 50 == 10+4+16+20).
        assert_eq!(
            line,
            "probe_stats visited=50 pruned_filter=10 pruned_dead=4 pruned_seen=16 scored=20 clusters_probed=6 router_scored=18 min_candidates=32 termination=ceiling:1,gate:1,exhausted:0 postings=row:4,skipped:2 exact_rows=0"
        );
    }

    #[test]
    fn format_probe_stats_empty() {
        assert_eq!(
            format_probe_stats(&[]),
            "probe_stats visited=0 pruned_filter=0 pruned_dead=0 pruned_seen=0 scored=0 clusters_probed=0 router_scored=0 min_candidates=0 termination=ceiling:0,gate:0,exhausted:0 postings=row:0,skipped:0 exact_rows=0"
        );
    }

    /// The two trailing tokens: `postings=` sums the per-segment fetch-mode
    /// buckets, `exact_rows=` the flat-path row reads, and the emitted line
    /// upholds the partition invariant `row + skipped == clusters_probed`
    /// (an IVF segment's buckets partition its probed clusters; a flat
    /// segment contributes zero to all three sides).
    #[test]
    fn format_probe_stats_posting_and_exact_tokens() {
        // One IVF-shaped segment plus one flat-shaped segment, which fills
        // only the `exact_rows_read` counter (everything else default).
        let flat = ProbeStats {
            exact_rows_read: 11,
            ..Default::default()
        };
        let line = format_probe_stats(&[seg(ProbeTermination::Exhausted), flat]);
        assert_eq!(
            line,
            "probe_stats visited=25 pruned_filter=5 pruned_dead=2 pruned_seen=8 scored=10 clusters_probed=3 router_scored=9 min_candidates=16 termination=ceiling:0,gate:0,exhausted:2 postings=row:2,skipped:1 exact_rows=11"
        );
        // Partition invariant, parsed back off the emitted line.
        assert_eq!(
            compound(&line, "postings", "row") + compound(&line, "postings", "skipped"),
            scalar(&line, "clusters_probed"),
        );
    }
}

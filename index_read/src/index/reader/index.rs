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

use std::cell::OnceCell;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::{Debug, Display};
use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use crate::aggregate::mvcc_collector::MVCCFilterCollector;
use crate::api::version::Version;
use crate::api::{CTID_FIELD_NAME, FieldName, HashSet, OrderByFeature, OrderByInfo, SortDirection};
use crate::gucs;
use crate::index::fast_fields_helper::FFHelper;
use crate::index::mvcc::{MVCCDirectory, MvccSatisfies, SegmentPins, SegmentView};
use crate::index::reader::io_stats;
use crate::index::reader::scorer::{DeferredScorer, LazyWeight, ScorerIter};
use crate::index::reader::sort_by_range::SortByRange;
use crate::index::reader::vector::{
    SharedVectorRouting, VectorFilterWeight, VectorRoutingResults, probe_params,
};
use crate::index::segment_pruning::SegmentStatsSnapshot;
use crate::index::setup_tokenizers;
use crate::index::stats::{EmpiricalStats, PartitionSegments, SegmentStats};
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::options::{SortByDirection, SortByField};
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::sequentialscan::KeySet;
use crate::postgres::storage::metadata::{CleanupAdmission, MetaPage};
use crate::postgres::types::TantivyValue;
use crate::query::SearchQueryInput;
use crate::query::estimate::{
    EstimateContext, QueryEstimate, can_drive_lookup_from_postings, is_zero_estimate_exact,
};
use crate::query::estimate_tree::QueryWithEstimates;
use crate::query::segment_pruning::SegmentPruner;
use crate::scan::info::RowEstimate;
use crate::schema::{SearchFieldType, SearchIndexSchema};

use anyhow::{Context, Result};
use pgrx::PgLogLevel;
use pgrx::pg_sys::BlockNumber;
use tantivy::aggregation::DistributedAggregationCollector;
use tantivy::aggregation::intermediate_agg_result::IntermediateAggregationResults;
use tantivy::collector::sort_key::{
    ComparatorEnum, SortByBytes, SortByErasedType, SortBySimilarityScore, SortByStaticFastValue,
    SortByString,
};
use tantivy::collector::{Collector, SegmentCollector, SortKeyComputer, TopDocs};
use tantivy::columnar::Cardinality;
use tantivy::directory::error::Incompatibility;
use tantivy::index::{Index, Order, SegmentId};
use tantivy::query::{ConstScoreQuery, EnableScoring, QueryClone, QueryParser, Scorer, Weight};
use tantivy::schema::Field;
use tantivy::snippet::SnippetGenerator;
use tantivy::vector::{
    ProbeStats, TopClustersBySimilarityCollector, TopDocsByVectorSimilarity,
    TopDocsByVectorSimilarityBuilder, VectorSearchBudget, VectorSegment, VectorSimilarityFruit,
};
use tantivy::{
    DateTime, DocAddress, DocId, DocSet, IndexReader, ReloadPolicy, Score, Searcher,
    SegmentOrdinal, SegmentReader, TantivyDocument, Term, query::Query, schema::OwnedValue,
};

/// The maximum number of sort-features/`OrderByInfo`s supported for
/// `SearchIndexReader::search_top_k_in_segments`.
pub const MAX_TOPK_FEATURES: usize = 5;

const DEFAULT_VECTOR_MATCH_FRACTION: f64 = 0.01;

#[cold]
#[track_caller]
fn report_segment_collection_error(error: tantivy::TantivyError) -> ! {
    // Tantivy exposes this failure as a string, not a distinct error variant.
    // Keep its quoted field name and leave unrelated collection errors intact.
    if let tantivy::TantivyError::InvalidArgument(message) = &error
        && let Some(field) = message
            .strip_prefix("Field ")
            .and_then(|message| message.strip_suffix(" is not configured as fast field"))
    {
        pgrx::pg_sys::panic::ErrorReport::new(
            pgrx::PgSqlErrorCode::ERRCODE_INVALID_PARAMETER_VALUE,
            format!("field {field} requires columnar storage"),
            pgrx::function_name!(),
        )
        .set_hint(
            "Recreate the index with 'columnar=true' on the field's tokenizer, or use \
             pdb.literal or pdb.literal_normalized, which enable columnar storage by default. \
             For a JSON path, configure its parent JSON field.",
        )
        .report(pgrx::PgLogLevel::ERROR);
    }
    panic!("should be able to collect in segment: {error:?}");
}

#[derive(Debug, Clone, Copy)]
pub struct DocsEstimate {
    pub matching_docs: usize,
    pub total_docs: u64,
    pub query_cost: u64,
}

/// A count-only summary of the pruning proof for this reader's segment snapshot.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SegmentPruningEstimate {
    pub candidate_segments: usize,
    pub pruned_segments: usize,
    pub candidate_docs: u64,
}

struct EstimationSample<'a> {
    candidates: Vec<&'a SegmentReader>,
    candidate_docs: u64,
    /// Proportion `reader_docs / total_docs` covered when summing across all `candidates`.
    candidates_doc_proportion: f64,
    largest_reader: &'a SegmentReader,
    /// Proportion `(largest_seg_docs / candidate_docs) * (reader_docs / total_docs)` covered by
    /// `largest_reader` alone.
    largest_doc_proportion: f64,
}

fn scale_largest_segment_estimate(value: u64, segment_doc_proportion: f64) -> u64 {
    if segment_doc_proportion > 0.0 {
        (value as f64 / segment_doc_proportion).ceil() as u64
    } else {
        value
    }
}

fn scorer_estimate(weight: &dyn Weight, reader: &SegmentReader) -> (u64, u64) {
    let mut scorer = weight
        .scorer(reader, 1.0)
        .expect("creating scorer for estimation should not fail");
    let mut count = scorer.size_hint() as u64;
    let mut cost = scorer.cost();
    if count == 0 {
        count = scorer.count_including_deleted() as u64;
        cost = cost.max(count);
    }
    (count, cost)
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

/// Maximum number of documents to return in a single Top-K window pull.
#[derive(Clone, Copy, Debug)]
pub struct TopKLimit(pub usize);

/// Number of leading documents to skip in a Top-K window pull.
#[derive(Clone, Copy, Debug)]
pub struct TopKOffset(pub usize);

/// Window bounds (`limit` and `offset`) for a Top-K search pull.
#[derive(Clone, Copy, Debug)]
pub struct TopKWindow {
    pub limit: TopKLimit,
    pub offset: TopKOffset,
}

pub enum VectorRouting {
    Local,
    Shared(
        Arc<SharedVectorRouting>,
        Arc<tantivy::vector::AtomicVectorWork>,
    ),
}

#[derive(Clone, Copy, Debug)]
enum SegmentQueryKind {
    Unconstrained,
    RangeConstrained,
}

/// Top-k results, aggregations, and per-segment metrics.
pub struct TopKSearch {
    /// Search results.
    pub results: Vec<(SearchIndexScore, DocAddress)>,
    /// Optional aggregations.
    pub aggregation_results: Option<IntermediateAggregationResults>,
    /// Per-segment collector metrics.
    pub segment_info: BTreeMap<SegmentId, serde_json::Value>,
}

impl TopKSearch {
    pub fn new(
        results: Vec<(SearchIndexScore, DocAddress)>,
        aggregation_results: Option<IntermediateAggregationResults>,
    ) -> Self {
        Self {
            results,
            aggregation_results,
            segment_info: BTreeMap::new(),
        }
    }

    pub fn with_segment_info(
        results: Vec<(SearchIndexScore, DocAddress)>,
        aggregation_results: Option<IntermediateAggregationResults>,
        segment_info: BTreeMap<SegmentId, serde_json::Value>,
    ) -> Self {
        Self {
            results,
            aggregation_results,
            segment_info,
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
}

/// Whether this build must rebuild the index before it can read the vectors `error` came from.
pub fn needs_rebuild(error: &tantivy::TantivyError) -> bool {
    matches!(
        error,
        tantivy::TantivyError::IncompatibleIndex(
            Incompatibility::VectorFormatMismatch { .. }
                | Incompatibility::CentroidFormatMismatch { .. }
                | Incompatibility::VectorRouterMismatch { .. },
        )
    )
}

/// Builds the PostgreSQL error for vector storage this build can't read without a rebuild.
pub fn vector_format_error_report(
    index_name: &str,
    error: &tantivy::TantivyError,
) -> Option<pgrx::pg_sys::panic::ErrorReport> {
    needs_rebuild(error).then(|| {
        pgrx::pg_sys::panic::ErrorReport::new(
            pgrx::PgSqlErrorCode::ERRCODE_FEATURE_NOT_SUPPORTED,
            format!("index {index_name:?} has an unsupported vector storage format: {error}"),
            pgrx::function_name!(),
        )
        .set_hint(format!("Rebuild index {index_name:?} with REINDEX."))
    })
}

fn report_vector_error(index_name: &str, error: &tantivy::TantivyError) {
    if let Some(report) = vector_format_error_report(index_name, error) {
        report.report(PgLogLevel::ERROR);
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

/// Base filter applied to a segment during a Random Access (`RA`) point lookup.
pub enum RaBaseFilter {
    /// Base query is `AllQuery` and the segment is unconstrained by partition bounds
    /// (every live document matches with base score `0.0`).
    MatchAll,
    /// Segment requires evaluating the weight as a `0.0`-score filter (a `partially_included`
    /// segment's `partition_bounds` on an `AllQuery` base, a disjunctive scan's base query, an
    /// unscored base query, or their conjunction), so its non-scoring `ConstScorer` is never
    /// added to the document score.
    Filter(Arc<LazyWeight>),
    /// Segment requires evaluating the weight, and the base query itself contributes its BM25
    /// score (`!unconstrained_is_match_all && need_scores`).
    Scored(Arc<LazyWeight>),
}

impl RaBaseFilter {
    fn filter(&self) -> Option<&Arc<LazyWeight>> {
        match self {
            Self::Filter(weight) => Some(weight),
            Self::MatchAll | Self::Scored(_) => None,
        }
    }

    fn scored(&self) -> Option<&Arc<LazyWeight>> {
        match self {
            Self::Scored(weight) => Some(weight),
            Self::MatchAll | Self::Filter(_) => None,
        }
    }
}

/// Reusable base query weights for Random Access (`RA`) point lookups in `lazy_sort` mode.
pub struct RaLookupWeights {
    /// Base filter for unconstrained segments (or all candidate segments when not range-partitioned).
    unconstrained: RaBaseFilter,
    /// Partition segment sets and range-constrained filter for boundary segments when range-partitioned.
    constrained: Option<(PartitionSegments, RaBaseFilter)>,
    /// Optional union of local disjunct queries (`ScanMode::Disjunctive`) used to score
    /// documents fetched via `RA` without materializing boolean match-tag columns.
    disjunct_weight: Option<Arc<LazyWeight>>,
}

/// Lazily initialized per-segment scorer for `ScanMode::Disjunctive` `RA` lookups.
struct DisjunctScorer<'a> {
    segment_reader: &'a SegmentReader,
    weight: Option<&'a Arc<LazyWeight>>,
    scorer: Option<Box<dyn Scorer>>,
}

impl<'a> DisjunctScorer<'a> {
    fn new(segment_reader: &'a SegmentReader, weight: Option<&'a Arc<LazyWeight>>) -> Self {
        Self {
            segment_reader,
            weight,
            scorer: None,
        }
    }

    fn score_for(&mut self, target_doc: DocId) -> Score {
        let Some(weight) = self.weight else {
            return 0.0;
        };
        let scorer = self.scorer.get_or_insert_with(|| {
            weight
                .get()
                .scorer(self.segment_reader, 1.0)
                .expect("disjunct scorer should be constructable")
        });
        let mut cur = scorer.doc();
        if cur < target_doc {
            cur = scorer.seek(target_doc);
        }
        if cur == target_doc {
            scorer.score()
        } else {
            0.0
        }
    }
}

/// Result of probing [`SegmentBaseFilter`] at a candidate `DocId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BaseFilterMatch {
    Matched,
    NextCandidate(DocId),
}

/// Maximum estimated live documents in a postings-backed `base_filter_weight` for which
/// `collect_lookup_only_segment` drives intersection from `base_filter_weight` (probing
/// `lookup_weight` in `O(1)` via `seek_danger`) instead of materializing `lookup_weight.scorer`.
const MAX_BASE_DRIVER_DOCS: u64 = 128;

/// Lazily initialized per-segment base filter for `RA` lookups.
struct SegmentBaseFilter<'a> {
    segment_reader: &'a SegmentReader,
    weight: Option<&'a Arc<LazyWeight>>,
    candidate_size: u32,
    state: Option<(Box<dyn Scorer>, DocId)>,
}

impl<'a> SegmentBaseFilter<'a> {
    fn new(
        segment_reader: &'a SegmentReader,
        weight: Option<&'a Arc<LazyWeight>>,
        candidate_size: u32,
    ) -> Self {
        Self {
            segment_reader,
            weight,
            candidate_size,
            state: None,
        }
    }

    fn seek_match(&mut self, target_doc: DocId) -> BaseFilterMatch {
        let Some(weight) = self.weight else {
            return BaseFilterMatch::Matched;
        };
        // Non-`TermSetQuery` weights (such as `RangeQuery` or `BooleanQuery` from
        // `and_query`) fall back to `Weight::scorer`, which positions the scorer at its first
        // matching document (`scorer.doc()`) and records `last_seek_pos_opt` on any child
        // `RangeDocSet`. Tracking `min_doc` avoids calling `seek_danger` with a `target_doc`
        // below the scorer's current lower bound (which would underflow `new_seek -
        // last_seek_pos` in `RangeDocSet::is_last_seek_distance_large`).
        let (scorer, min_doc) = self.state.get_or_insert_with(|| {
            let scorer = weight
                .get()
                .scorer_for_candidates_unpositioned(
                    self.segment_reader,
                    1.0,
                    self.candidate_size.max(1),
                )
                .expect("base filter scorer should be constructable");
            let first_doc = scorer.doc();
            (scorer, first_doc)
        });
        if target_doc < *min_doc {
            return BaseFilterMatch::NextCandidate(*min_doc);
        }
        match scorer.seek_danger(target_doc) {
            tantivy::SeekDangerResult::Found => {
                *min_doc = target_doc.saturating_add(1);
                BaseFilterMatch::Matched
            }
            tantivy::SeekDangerResult::SeekLowerBound(next_bound) => {
                *min_doc = next_bound;
                BaseFilterMatch::NextCandidate(next_bound)
            }
        }
    }

    fn matches(&mut self, target_doc: DocId) -> bool {
        matches!(self.seek_match(target_doc), BaseFilterMatch::Matched)
    }
}

/// Collects Random Access (`RA`) point-lookup matches from a single segment with an unscored base
/// (`RaBaseFilter::MatchAll` or `RaBaseFilter::Filter`) when `score_threshold` is `None` or
/// `< 0.0` (so `0.0`-scored matches cannot be pruned up front).
///
/// Emits live, not-yet-emitted documents matching `lookup_weight` (and `base_filter_weight`, when
/// present), scoring each match via `disjunct_weight` (`0.0` when absent or non-matching):
/// 1. Skips the segment without opening `lookup_weight` when `base_filter_weight` has an exact
///    zero dictionary estimate.
/// 2. Drives intersection from `base_filter_weight` when it is postings-backed with
///    `<= MAX_BASE_DRIVER_DOCS` estimated live docs, probing `lookup_weight` via
///    [`intersect_scorer_with_lookup`].
/// 3. Otherwise drives from `lookup_weight.scorer`, leapfrogging against [`SegmentBaseFilter`]
///    (`SeekLowerBound`) and lazily scoring matches via [`DisjunctScorer`].
#[allow(clippy::too_many_arguments)]
fn collect_lookup_only_segment(
    segment_ord: SegmentOrdinal,
    segment_reader: &SegmentReader,
    lookup_weight: &dyn Weight,
    base_filter_weight: Option<&Arc<LazyWeight>>,
    disjunct_weight: Option<&Arc<LazyWeight>>,
    score_threshold: Option<Score>,
    estimate_ctx: EstimateContext<'_>,
    emitted_docs: &mut crate::api::HashSet<DocAddress>,
    results: &mut Vec<(SearchIndexScore, DocAddress)>,
) {
    if segment_reader.num_docs() == 0 {
        return;
    }
    // Consult cheap base-filter dictionary statistics before opening `lookup_weight.scorer`.
    if let Some(bf_weight) = base_filter_weight
        && let Ok(Some(est)) = bf_weight
            .query()
            .estimate_docs_with_context(segment_reader, estimate_ctx)
    {
        let est_docs = est.live_docs(segment_reader);
        // Path 1: base filter provably matches zero docs in this segment.
        if est_docs == 0 && is_zero_estimate_exact(bf_weight.query()) {
            return;
        }
        // Path 2: sparse postings-backed base filter drives `seek_danger` probes into `lookup_weight`.
        if est_docs <= MAX_BASE_DRIVER_DOCS && can_drive_lookup_from_postings(bf_weight.query()) {
            let mut base_scorer = bf_weight
                .get()
                .scorer(segment_reader, 1.0)
                .expect("base filter scorer should be constructable");
            intersect_scorer_with_lookup(
                segment_ord,
                segment_reader,
                base_scorer.as_mut(),
                false,
                lookup_weight,
                disjunct_weight,
                score_threshold,
                None,
                emitted_docs,
                results,
            );
            return;
        }
    }

    // Path 3: drive from `lookup_weight`, leapfrogging against `SegmentBaseFilter`.
    let mut lookup_scorer = lookup_weight
        .scorer(segment_reader, 1.0)
        .expect("lookup scorer should be constructable");
    if lookup_scorer.doc() == tantivy::TERMINATED {
        return;
    }

    let alive_bitset = segment_reader.alive_bitset();
    let mut base_filter = SegmentBaseFilter::new(
        segment_reader,
        base_filter_weight,
        lookup_scorer.size_hint(),
    );
    let mut disjunct_scorer = DisjunctScorer::new(segment_reader, disjunct_weight);

    let mut doc_id = lookup_scorer.doc();
    while doc_id != tantivy::TERMINATED {
        let addr = DocAddress::new(segment_ord, doc_id);
        if !alive_bitset.is_none_or(|b| b.is_alive(doc_id)) || emitted_docs.contains(&addr) {
            doc_id = lookup_scorer.advance();
            continue;
        }
        match base_filter.seek_match(doc_id) {
            BaseFilterMatch::Matched => {
                let score = disjunct_scorer.score_for(doc_id);
                if score_threshold.is_none_or(|t| score > t) {
                    emitted_docs.insert(addr);
                    results.push((SearchIndexScore::new(score), addr));
                }
                doc_id = lookup_scorer.advance();
            }
            BaseFilterMatch::NextCandidate(tantivy::TERMINATED) => break,
            BaseFilterMatch::NextCandidate(next_doc) => {
                // Leapfrog `lookup_scorer` forward when the base filter skips past `doc_id + 1`.
                doc_id = if next_doc > doc_id + 1 {
                    lookup_scorer.seek(next_doc)
                } else {
                    lookup_scorer.advance()
                };
            }
        }
    }
}

/// Intersects a driving [`Scorer`] (either a [`tantivy::query::PruningScorer`] or a standard
/// [`Scorer`]) with `lookup_weight`'s candidate-aware [`TermSetQuery`](tantivy::query::TermSetQuery)
/// scorer via `seek_danger`.
#[allow(clippy::too_many_arguments)]
fn intersect_scorer_with_lookup<S: Scorer + ?Sized>(
    segment_ord: SegmentOrdinal,
    segment_reader: &SegmentReader,
    driver: &mut S,
    driver_contributes_score: bool,
    lookup_weight: &dyn Weight,
    disjunct_weight: Option<&Arc<LazyWeight>>,
    score_threshold: Option<Score>,
    base_filter_weight: Option<&Arc<LazyWeight>>,
    emitted_docs: &mut crate::api::HashSet<DocAddress>,
    results: &mut Vec<(SearchIndexScore, DocAddress)>,
) {
    if driver.doc() == tantivy::TERMINATED {
        return;
    }
    let candidate_size = driver.size_hint().max(1);
    let mut lookup_scorer = lookup_weight
        .scorer_for_candidates_unpositioned(segment_reader, 1.0, candidate_size)
        .expect("lookup scorer should be constructable");
    let first_lookup_doc = lookup_scorer.doc();
    if first_lookup_doc == tantivy::TERMINATED {
        return;
    }

    let alive_bitset = segment_reader.alive_bitset();
    let mut base_filter =
        SegmentBaseFilter::new(segment_reader, base_filter_weight, candidate_size);
    let mut disjunct_scorer = DisjunctScorer::new(segment_reader, disjunct_weight);

    let mut doc_id = driver.doc();
    if doc_id < first_lookup_doc {
        doc_id = driver.seek(first_lookup_doc);
    }
    while doc_id != tantivy::TERMINATED {
        match lookup_scorer.seek_danger(doc_id) {
            tantivy::SeekDangerResult::Found => {
                let addr = DocAddress::new(segment_ord, doc_id);
                if alive_bitset.is_none_or(|b| b.is_alive(doc_id))
                    && !emitted_docs.contains(&addr)
                    && base_filter.matches(doc_id)
                {
                    let base_score = if driver_contributes_score {
                        driver.score()
                    } else {
                        0.0
                    };
                    let score = base_score + disjunct_scorer.score_for(doc_id);
                    if score_threshold.is_none_or(|t| score > t) {
                        emitted_docs.insert(addr);
                        results.push((SearchIndexScore::new(score), addr));
                    }
                }
                if lookup_scorer.has_fast_bitset() {
                    let next_target = lookup_scorer.advance();
                    if next_target == tantivy::TERMINATED {
                        break;
                    }
                    doc_id = if next_target > doc_id + 1 {
                        driver.seek(next_target)
                    } else {
                        driver.advance()
                    };
                } else {
                    doc_id = driver.advance();
                }
            }
            tantivy::SeekDangerResult::SeekLowerBound(next_target) => {
                if next_target == tantivy::TERMINATED {
                    break;
                }
                doc_id = if next_target > doc_id + 1 {
                    driver.seek(next_target)
                } else {
                    driver.advance()
                };
            }
        }
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

impl MultiSegmentSearchResults {
    pub fn current_segment(&mut self) -> Option<&mut ScorerIter> {
        if crate::postgres::customscan::mpp::check_for_interrupts_or_cancel() {
            return None;
        }
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

    /// Creates an empty `MultiSegmentSearchResults` retaining `searcher` for segment reader access.
    pub fn empty(searcher: Searcher) -> Self {
        Self {
            searcher,
            iterators: Vec::new(),
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

/// Reader over a ParadeDB Tantivy index snapshot, encapsulating segment checkout,
/// query compilation, segment pruning, and search execution.
///
/// TODO: Decompose `SearchIndexReader` into smaller, focused components (e.g. separating
/// index/segment lifecycle, query compilation & selectivity estimation, and `SA`/`RA`/Top-K
/// search execution).
pub struct SearchIndexReader {
    index_rel: PgSearchRelation,
    searcher: Searcher,
    schema: SearchIndexSchema,
    underlying_reader: IndexReader,
    underlying_index: Index,
    query: Box<dyn Query>,
    estimate_from_statistics: bool,
    segment_stats_snapshot: Arc<SegmentStatsSnapshot>,
    /// The original query, retained for pruning only when a predicate references a field with
    /// segment statistics. This eligibility check needs no segment statistics. Keep the whole
    /// query so mixed Boolean branches retain their original meaning; segment decisions still
    /// happen on demand.
    pruning_query: Option<SearchQueryInput>,
    need_scores: bool,
    total_segment_count: usize,
    total_docs: u64,
    index_created_by_version: Option<Version>,
    /// Query-level reader initialization time.
    scan_init_ns: u64,
    /// The directory `underlying_index` opened over; kept to capture this reader's
    /// [`SegmentView`].
    directory: MVCCDirectory,

    vector_segments_valid: OnceCell<tantivy::Result<()>>,
    pub vector_routing_plan: Option<Arc<SharedVectorRouting>>,
    pruning_estimate: OnceLock<SegmentPruningEstimate>,
    candidate_segments: Arc<[OnceLock<bool>]>,

    // Keep cleanup admission alive across reader clones.
    _cleanup_lock: Arc<CleanupAdmission>,
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
            estimate_from_statistics: self.estimate_from_statistics,
            segment_stats_snapshot: Arc::clone(&self.segment_stats_snapshot),
            pruning_query: self.pruning_query.clone(),
            need_scores: self.need_scores,
            total_segment_count: self.total_segment_count,
            total_docs: self.total_docs,
            index_created_by_version: self.index_created_by_version,
            scan_init_ns: self.scan_init_ns,
            directory: self.directory.clone(),
            vector_segments_valid: self.vector_segments_valid.clone(),
            vector_routing_plan: self.vector_routing_plan.clone(),
            pruning_estimate: self.pruning_estimate.clone(),
            candidate_segments: Arc::clone(&self.candidate_segments),
            _cleanup_lock: self._cleanup_lock.clone(),
        }
    }
}

#[cfg(any(test, feature = "pg_test"))]
pub mod test_support {
    use super::PgSearchRelation;
    use super::SearchIndexReader;
    use crate::api::FieldName;
    use crate::index::mvcc::MvccSatisfies;
    use crate::postgres::pdb_owned_value::PdbOwnedValue;
    use crate::query::SearchQueryInput;
    use crate::query::pdb_query::pdb;
    use pgrx::Spi;
    use std::sync::atomic::AtomicUsize;

    /// How many times the index was actually opened (metadata walk, pins, searcher build).
    /// Tests use it to prove reuse paths perform zero additional opens.
    pub static INDEX_COMPONENT_OPENS: AtomicUsize = AtomicUsize::new(0);

    pub fn range_query(field: &str, lower: i64, upper: i64) -> SearchQueryInput {
        SearchQueryInput::FieldedQuery {
            field: FieldName::from(field),
            query: pdb::Query::Range {
                lower_bound: std::ops::Bound::Included(PdbOwnedValue::I64(lower)),
                upper_bound: std::ops::Bound::Included(PdbOwnedValue::I64(upper)),
            },
        }
    }

    pub fn term_query(field: &str, value: &str) -> SearchQueryInput {
        SearchQueryInput::FieldedQuery {
            field: FieldName::from(field),
            query: pdb::Query::Term {
                value: PdbOwnedValue::Str(value.to_string()),
            },
        }
    }

    /// Opens the index a test just created through SPI.
    pub fn open_index(name: &str) -> PgSearchRelation {
        unsafe { pgrx::pg_sys::CommandCounterIncrement() };
        let oid = Spi::get_one::<pgrx::pg_sys::Oid>(&format!("SELECT '{name}'::regclass::oid"))
            .unwrap()
            .unwrap();
        PgSearchRelation::open(oid)
    }

    pub fn open_snapshot_reader(
        index_rel: &PgSearchRelation,
        query: SearchQueryInput,
        need_scores: bool,
    ) -> SearchIndexReader {
        SearchIndexReader::open(index_rel, query, need_scores, MvccSatisfies::Snapshot).unwrap()
    }

    /// Every entry point of `reader` agrees with Tantivy searching the same query over every
    /// segment, so candidate filtering changed nothing observable.
    pub fn assert_pruning_matches_tantivy(reader: &SearchIndexReader, expected: usize) {
        use tantivy::collector::{Count, TopDocs};

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
    /// Test fixture shared by the reuse/laziness tests: a table + ParadeDB index laid out as
    /// `immutable_batches` frozen segments of 10 rows (batch 0's titles contain "silver dragon",
    /// later batches "quiet river"), plus an optional 5-row mutable segment. Returns the opened
    /// index relation and the heap relation's OID.
    pub fn segmented_index_fixture(
        name: &str,
        immutable_batches: usize,
        with_mutable: bool,
    ) -> (PgSearchRelation, pgrx::pg_sys::Oid) {
        let mut sql = format!(
            "CREATE TABLE {name} (id bigint PRIMARY KEY, title text NOT NULL);
             CREATE INDEX {name}_idx ON {name}
             USING paradedb (id, (title::pdb.unicode_words('columnar=true')))
             WITH (partition_by = 'id', target_segment_count = 8, background_layer_sizes = '0');
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
    cleanup_lock: Arc<CleanupAdmission>,
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
    pub fn index_rel(&self) -> &PgSearchRelation {
        &self.index_rel
    }

    pub fn estimate_vector_match_fraction(&self) -> f64 {
        let candidates: Vec<_> = self.candidate_segment_readers().collect();
        let ctx = EstimateContext::for_candidates(&self.segment_stats_snapshot, candidates.len());
        let (matches, docs) =
            candidates
                .into_iter()
                .fold((0.0, 0u64), |(matches, docs), (_, reader)| {
                    let estimated = self
                        .query
                        .estimate_docs_with_context(reader, ctx)
                        .ok()
                        .flatten()
                        .map_or(
                            f64::from(reader.num_docs()) * DEFAULT_VECTOR_MATCH_FRACTION,
                            |estimate| estimate.live_docs(reader) as f64,
                        );
                    (matches + estimated, docs + u64::from(reader.num_docs()))
                });
        matches / docs.max(1) as f64
    }

    pub fn vector_search_budget(&self, name: &FieldName) -> VectorSearchBudget {
        let field = self
            .schema
            .tantivy_schema()
            .get_field(&name.root())
            .expect("vector field should exist");
        let segments = self
            .candidate_segment_readers()
            .map(VectorSegment::from)
            .collect::<Vec<_>>();
        VectorSearchBudget::for_segments(field, &probe_params(), &segments)
            .inspect_err(|error| report_vector_error(self.index_rel.name(), error))
            .expect("vector budget should be valid")
    }

    pub fn vector_routing_plan(
        &self,
        name: &FieldName,
        query: &[f32],
        match_fraction: f64,
        budget: VectorSearchBudget,
    ) -> VectorRoutingResults {
        let field = self
            .schema
            .tantivy_schema()
            .get_field(&name.root())
            .expect("vector field should exist");
        let segments = self
            .candidate_segment_readers()
            .map(|(ordinal, reader)| {
                let mut segment = VectorSegment::from((ordinal, reader));
                segment.partition_centroid = SegmentStats::of_reader(reader)
                    .expect("segment stats should be readable")
                    .and_then(|stats| {
                        stats
                            .vector_partition(field)
                            .expect("partition centroid should be readable")
                    })
                    .map(Into::into);
                segment
            })
            .collect::<Vec<_>>();
        let router = TopClustersBySimilarityCollector::new(
            field,
            query.to_vec(),
            probe_params(),
            match_fraction,
        )
        .expect("vector routing parameters should be valid")
        .with_budget(budget);
        let candidates = router
            .collect_segments(&segments, |ord| {
                pgrx::check_for_interrupts();
                if let Some(stats) = &self.directory.io_stats {
                    stats.end_segment(self.searcher.segment_reader(ord).segment_id());
                }
            })
            .inspect_err(|error| report_vector_error(self.index_rel.name(), error))
            .expect("vector routing should not fail");
        let mut by_segment = crate::api::HashMap::default();
        for candidate in candidates {
            by_segment
                .entry(candidate.node.0)
                .or_insert_with(Vec::new)
                .push(candidate);
        }
        VectorRoutingResults {
            budget,
            candidates: by_segment,
            statistics: router.take_statistics(),
        }
    }

    /// Validates this reader's visible vector headers once, including an unsuccessful check.
    /// An unsupported format raises a PostgreSQL error and does not return; other errors are returned.
    pub fn validate_vector_segments(&self) -> Result<()> {
        let result = self.vector_segments_valid.get_or_init(|| {
            self.searcher
                .segment_readers()
                .iter()
                .try_for_each(SegmentReader::validate_vector_format)
        });
        match result {
            Ok(()) => Ok(()),
            Err(error) => match vector_format_error_report(self.index_rel.name(), error) {
                Some(report) => {
                    report.report(pgrx::PgLogLevel::ERROR);
                    unreachable!("PostgreSQL ERROR reports do not return")
                }
                None => Err(error.clone().into()),
            },
        }
    }

    /// Returns the minimum and maximum heap block numbers represented in the segment.
    pub fn block_bounds(segment: &SegmentReader) -> Result<Option<RangeInclusive<BlockNumber>>> {
        let field = segment.schema().get_field(CTID_FIELD_NAME)?;
        let stats = SegmentStats::of_reader(segment)?;
        let empirical = stats
            .map(|stats| stats.empirical(field))
            .transpose()?
            .flatten();
        let (min, max) = if let Some(EmpiricalStats {
            min: PdbOwnedValue::U64(min),
            max: PdbOwnedValue::U64(max),
            nullable: false,
        }) = empirical
        {
            (min, max)
        } else {
            let ctids = segment.fast_fields().u64(CTID_FIELD_NAME)?;
            if ctids.get_cardinality() != Cardinality::Full || ctids.num_docs() != segment.max_doc()
            {
                return Ok(None);
            }
            (ctids.min_value(), ctids.max_value())
        };
        let first =
            BlockNumber::try_from(min >> 16).context("heap block number exceeds BlockNumber")?;
        let last =
            BlockNumber::try_from(max >> 16).context("heap block number exceeds BlockNumber")?;
        Ok(Some(first..=last))
    }

    fn open_index_components(
        index_relation: &PgSearchRelation,
        mvcc_style: MvccSatisfies,
        needs_tokenizer_manager: bool,
        io_stats: Option<io_stats::Trace>,
    ) -> Result<IndexComponents> {
        let _metadata = io_stats.as_ref().map(|stats| stats.external("Metadata"));
        #[cfg(any(test, feature = "pg_test"))]
        test_support::INDEX_COMPONENT_OPENS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let cleanup_lock = Arc::new(MetaPage::open(index_relation).cleanup_lock_for_reader());

        let mut directory = mvcc_style.directory(index_relation);
        directory.io_stats = io_stats;
        let mut index = crate::index::open_index(directory.clone())?;
        index.settings_mut().bitmap_postings.use_for_queries = gucs::enable_bitmap_postings();
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
        let segment_stats_snapshot = SegmentStatsSnapshot::capture(&searcher, &directory);

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
            None,
        )
    }

    /// Open a tantivy index with optional expression context for proper postgres expression evaluation
    #[allow(clippy::too_many_arguments)]
    pub fn open_with_context(
        index_relation: &PgSearchRelation,
        search_query_input: SearchQueryInput,
        need_scores: bool,
        mvcc_style: MvccSatisfies,
        expr_context: Option<NonNull<pgrx::pg_sys::ExprContext>>,
        planstate: Option<NonNull<pgrx::pg_sys::PlanState>>,
        needs_tokenizer_manager: bool,
        io_stats: Option<io_stats::Trace>,
    ) -> Result<Self> {
        let scan_init_start = Instant::now();
        let scan_init_io = io_stats.as_ref().map(io_stats::Trace::begin_scan_init);
        // Derive the tokenizer need from the query as well as the caller's flag: a caller
        // passing `false` alongside a query that tokenizes must not silently parse wrong.
        let needs_tokenizer_manager =
            needs_tokenizer_manager || search_query_input.needs_tokenizer();
        let components = Self::open_index_components(
            index_relation,
            mvcc_style,
            needs_tokenizer_manager,
            io_stats,
        )?;
        let mut reader = Self::from_components(
            index_relation,
            components,
            search_query_input,
            need_scores,
            expr_context,
            planstate,
        )?;
        drop(scan_init_io);
        reader.scan_init_ns = scan_init_start.elapsed().as_nanos() as u64;
        Ok(reader)
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
        let scan_init_start = Instant::now();
        let scan_init_io = manifest
            .components()
            .directory
            .io_stats
            .as_ref()
            .map(io_stats::Trace::begin_scan_init);
        if needs_tokenizer_manager || search_query_input.needs_tokenizer() {
            crate::index::search::register_tokenizers(
                index_relation,
                &manifest.components().index,
            )?;
        }
        let mut reader = Self::from_components(
            index_relation,
            manifest.components().clone(),
            search_query_input,
            need_scores,
            expr_context,
            None,
        )?;
        drop(scan_init_io);
        reader.scan_init_ns = scan_init_start.elapsed().as_nanos() as u64;
        Ok(reader)
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

        let index_created_by_version = {
            let _metadata = directory
                .io_stats
                .as_ref()
                .map(|stats| stats.external("Metadata"));
            index_relation.created_by_version()
        };
        let need_scores = need_scores || search_query_input.need_scores();
        let parser = || {
            QueryParser::for_index(
                &index,
                schema.fields().map(|(field, _)| field).collect::<Vec<_>>(),
            )
        };
        let pruning_query = SegmentPruner::has_prunable_predicate(&schema, &search_query_input)
            .then(|| search_query_input.clone());
        let estimate_from_statistics = search_query_input.supports_statistics_estimation();
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

        let candidate_segments = (0..searcher.segment_readers().len())
            .map(|_| OnceLock::new())
            .collect();

        Ok(Self {
            index_rel: index_relation.clone(),
            searcher,
            schema,
            underlying_reader: reader,
            underlying_index: index,
            query,
            estimate_from_statistics,
            segment_stats_snapshot,
            pruning_query,
            need_scores,
            total_segment_count,
            total_docs,
            index_created_by_version,
            scan_init_ns: 0,
            directory,
            _cleanup_lock: cleanup_lock,
            vector_segments_valid: OnceCell::new(),
            vector_routing_plan: None,
            pruning_estimate: OnceLock::new(),
            candidate_segments,
        })
    }

    pub fn segment_ids(&self) -> Vec<SegmentId> {
        self.searcher
            .segment_readers()
            .iter()
            .map(|r| r.segment_id())
            .collect()
    }

    pub fn scan_init_ns(&self) -> u64 {
        self.scan_init_ns
    }

    pub fn add_scan_init_ns(&mut self, elapsed_ns: u64) {
        self.scan_init_ns = self.scan_init_ns.saturating_add(elapsed_ns);
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

    pub fn cleanup_admission(&self) -> Arc<CleanupAdmission> {
        self._cleanup_lock.clone()
    }

    pub fn need_scores(&self) -> bool {
        self.need_scores
    }

    /// The compiled query. Segment pruning happens in the reader's search and collection
    /// methods, not in the query.
    pub fn query(&self) -> &dyn Query {
        self.query.as_ref()
    }

    /// Returns a reader matching the same documents as `self` but scoring each one `score`.
    pub fn with_constant_score(&self, score: Score) -> Self {
        let mut clone = self.clone();
        let existing = std::mem::replace(&mut clone.query, Box::new(tantivy::query::EmptyQuery));
        clone.query = Box::new(ConstScoreQuery::new(existing, score));
        clone
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
        clone.pruning_estimate = OnceLock::new();
        clone.estimate_from_statistics = false;
        clone
    }

    /// Compiles a non-empty slice of `disjunct_queries` into a single `Occur::Should` union query.
    fn make_disjunct_union_query(
        &self,
        disjunct_queries: &[SearchQueryInput],
    ) -> Option<Box<dyn Query>> {
        if disjunct_queries.is_empty() {
            return None;
        }
        let mut should_clauses: Vec<(tantivy::query::Occur, Box<dyn Query>)> = disjunct_queries
            .iter()
            .map(|q| (tantivy::query::Occur::Should, self.make_query(q, None)))
            .collect();
        if should_clauses.len() == 1 {
            Some(should_clauses.pop().unwrap().1)
        } else {
            Some(Box::new(tantivy::query::BooleanQuery::new(should_clauses)))
        }
    }

    /// Build a reader for Sorted Access (`SA`) over a disjunctive scan (`ScanMode::Disjunctive`) by
    /// restricting `SA` to documents that match at least one of `disjunct_queries`.
    /// When `replace_query` is true (i.e. `base_query` is `SearchQueryInput::All` and no
    /// additional filter was pushed into `self`), the union of `disjunct_queries` replaces
    /// the `AllQuery` directly so Tantivy can use Block-Max WAND over the disjunct postings.
    /// Otherwise the union is intersected with the reader's query; when that query has a
    /// constant score, tantivy still prunes the union with Block-Max WAND
    /// (`SpecializedScorer::FilteredTermUnion`).
    pub fn with_disjunctive_sa_queries(
        &self,
        replace_query: bool,
        disjunct_queries: &[SearchQueryInput],
    ) -> Self {
        let Some(disjunct_union) = self.make_disjunct_union_query(disjunct_queries) else {
            return self.clone();
        };
        if replace_query {
            let mut clone = self.clone();
            clone.query = disjunct_union;
            clone.pruning_estimate = OnceLock::new();
            clone.estimate_from_statistics = false;
            clone
        } else {
            self.and_query(disjunct_union)
        }
    }

    /// Compiled without an expression context: only the reader's own query carries resolved
    /// Params, and it is kept as is.
    #[cfg(any(test, feature = "pg_test"))]
    pub fn and_query_input(&self, query: &SearchQueryInput) -> Self {
        self.and_query(self.make_query(query, None))
    }

    fn pruner(&self) -> SegmentPruner<'_> {
        SegmentPruner::new(
            &self.segment_stats_snapshot,
            &self.schema,
            self.index_created_by_version,
        )
    }

    fn is_candidate(&self, ord: SegmentOrdinal) -> bool {
        *self.candidate_segments[ord as usize].get_or_init(|| {
            self.segment_num_docs(self.searcher.segment_reader(ord)) > 0
                && self
                    .pruning_query
                    .as_ref()
                    .is_none_or(|query| self.pruner().can_match(ord, query))
        })
    }

    fn directory_segment_num_docs(directory: &MVCCDirectory, reader: &SegmentReader) -> u64 {
        directory
            .segment_num_docs(&reader.segment_id())
            .map(|n| n as u64)
            .unwrap_or_else(|| u64::from(reader.num_docs()))
    }

    fn segment_num_docs(&self, reader: &SegmentReader) -> u64 {
        Self::directory_segment_num_docs(&self.directory, reader)
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
        let weight = self
            .query
            .weight(enable_scoring(self.need_scores, &self.searcher).with_bitmap_postings(true))?;
        let mut total = 0u64;
        for (_, segment_reader) in self.candidate_segment_readers() {
            total += u64::from(weight.count(segment_reader)?);
        }
        Ok(total)
    }

    pub fn weight(&self) -> Box<dyn Weight> {
        self.query
            .weight(enable_scoring(self.need_scores, &self.searcher))
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

        visibility.set_ffhelper(Arc::new(FFHelper::for_ctid(self)));

        let mut search_results = self.search();
        let mut doc_ids = Vec::with_capacity(VISIBILITY_BATCH_SIZE);
        let mut resolved = Vec::with_capacity(VISIBILITY_BATCH_SIZE);
        let mut visible_ctids = Vec::new().into_iter();

        KeySet::build_from(std::iter::from_fn(move || {
            loop {
                if let Some(ctid) = visible_ctids.next() {
                    return Some(
                        TantivyValue::try_from(ctid)
                            .expect("ctid should convert to a Tantivy value"),
                    );
                }

                let seg = search_results.current_segment()?;
                let seg_ord = seg.segment_ord();

                doc_ids.clear();
                while doc_ids.len() < VISIBILITY_BATCH_SIZE {
                    if let Some((_, doc_addr)) = seg.next() {
                        doc_ids.push(doc_addr.doc_id);
                    } else {
                        break;
                    }
                }

                if doc_ids.is_empty() {
                    search_results.current_segment_pop();
                    continue;
                }

                let ctids = visibility.resolve_segment_docs(seg_ord, &doc_ids, &mut resolved);
                visible_ctids = ctids.iter_visible().collect::<Vec<_>>().into_iter();
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

    /// Selects the MVCC segment-loading strategy for plan-time estimation.
    ///
    /// When the query contains predicates that can prune segments via `.stats`, open all segment
    /// headers (`MvccSatisfies::Snapshot` — which is lazy and does not open segment components)
    /// so `SegmentPruner` can filter to candidate segments before estimating. Otherwise, open only
    /// the largest segment (`MvccSatisfies::LargestSegment`).
    ///
    /// TODO: <https://github.com/paradedb/paradedb/issues/6976> will maintain persistent, index-wide
    /// summary statistics so estimation no longer needs to sample a single largest segment.
    pub fn mvcc_for_estimation(
        index_relation: &PgSearchRelation,
        search_query_input: &SearchQueryInput,
    ) -> MvccSatisfies {
        if index_relation
            .schema()
            .is_ok_and(|schema| SegmentPruner::has_prunable_predicate(&schema, search_query_input))
        {
            MvccSatisfies::Snapshot
        } else {
            MvccSatisfies::LargestSegment
        }
    }

    /// Returns the number of candidate segments after static pruning when all segments are
    /// loaded (`MvccSatisfies::Snapshot`), or `total_segment_count` when opened with
    /// `MvccSatisfies::LargestSegment`.
    pub fn estimated_segment_count(&self) -> usize {
        if self.pruning_query.is_some()
            && self.searcher.segment_readers().len() == self.total_segment_count
        {
            self.segment_pruning_estimate().candidate_segments
        } else {
            self.total_segment_count
        }
    }

    /// Estimates the number of candidate segments for `search_query_input` without building a
    /// Tantivy `Query` or `TokenizerManager`.
    ///
    /// When `search_query_input` contains a prunable predicate, opens `MvccSatisfies::Snapshot`
    /// and counts segments that survive [`SegmentPruner`]. Otherwise, opens
    /// `MvccSatisfies::LargestSegment` and returns `total_segment_count`.
    pub fn estimate_segment_count(
        index_relation: &PgSearchRelation,
        schema: &SearchIndexSchema,
        search_query_input: Option<&SearchQueryInput>,
    ) -> usize {
        if let Some(query) = search_query_input
            && SegmentPruner::has_prunable_predicate(schema, query)
        {
            let index_created_by_version = index_relation.created_by_version();
            let components =
                Self::open_index_components(index_relation, MvccSatisfies::Snapshot, false, None)
                    .expect("custom_scan: should be able to open index");
            let pruner = SegmentPruner::new(
                &components.segment_stats_snapshot,
                &components.schema,
                index_created_by_version,
            );
            components
                .searcher
                .segment_readers()
                .iter()
                .enumerate()
                .filter(|&(ord, reader)| {
                    Self::directory_segment_num_docs(&components.directory, reader) > 0
                        && pruner.can_match(ord as SegmentOrdinal, query)
                })
                .count()
        } else {
            let directory = MvccSatisfies::LargestSegment.directory(index_relation);
            let segment_count = directory.total_segment_count(); // return value only valid after the index has been opened
            crate::index::open_index(directory).expect("custom_scan: should be able to open index");
            segment_count.load(std::sync::atomic::Ordering::Relaxed)
        }
    }

    /// Returns the total number of docs in the index, according to the MVCC directory.
    pub fn total_docs(&self) -> u64 {
        self.total_docs
    }

    fn non_empty_segment_count(&self) -> usize {
        self.searcher
            .segment_readers()
            .iter()
            .filter(|r| self.segment_num_docs(r) > 0)
            .count()
    }

    fn pruning_estimate_from_candidates(
        &self,
        candidates: &[(SegmentOrdinal, &SegmentReader)],
    ) -> SegmentPruningEstimate {
        let non_empty_segments = self.non_empty_segment_count();
        let candidate_segments = candidates.len();
        let candidate_docs = if self.pruning_query.is_none() {
            self.total_docs
        } else {
            candidates
                .iter()
                .map(|(_, r)| self.segment_num_docs(r))
                .sum()
        };
        SegmentPruningEstimate {
            candidate_segments,
            pruned_segments: non_empty_segments.saturating_sub(candidate_segments),
            candidate_docs,
        }
    }

    /// Returns the cached segment pruning estimate if it has already been computed.
    pub fn cached_segment_pruning_estimate(&self) -> Option<SegmentPruningEstimate> {
        self.pruning_estimate.get().copied()
    }

    /// Counts the candidate and pruned segments in this reader's snapshot.
    pub fn segment_pruning_estimate(&self) -> SegmentPruningEstimate {
        *self.pruning_estimate.get_or_init(|| {
            if self.pruning_query.is_none() {
                SegmentPruningEstimate {
                    candidate_segments: self.non_empty_segment_count(),
                    pruned_segments: 0,
                    candidate_docs: self.total_docs,
                }
            } else {
                let candidates: Vec<(SegmentOrdinal, &SegmentReader)> =
                    self.candidate_segment_readers().collect();
                self.pruning_estimate_from_candidates(&candidates)
            }
        })
    }

    pub fn segment_stats_snapshot(&self) -> &Arc<SegmentStatsSnapshot> {
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

    /// The snippet generator for `column`, built over every indexed field the column is stored
    /// under.
    ///
    /// A column can back more than one field: an alias carries its own tokenizer over the same
    /// text, and a column indexed only through an aliased expression has no field of its own.
    /// Which of those fields a query matched cannot be read off the index, because term
    /// lookups go through the segment readers and an empty index reports that nothing is
    /// addressed at all. So the fields come from the index settings, and the generator selects
    /// fragments over the union of their matches.
    pub fn snippet_generator(
        &self,
        field_name: impl AsRef<str> + Display,
        query: &SearchQueryInput,
        expr_context: Option<NonNull<pgrx::pg_sys::ExprContext>>,
    ) -> SnippetGenerator {
        let named = self.schema.search_field(&field_name);
        if let Some(field) = &named
            && !(field.is_text() || field.is_json())
        {
            panic!(
                "failed to create snippet generator for field: {field_name}... can only highlight text fields"
            )
        }

        let mut fields: Vec<_> = named.iter().map(|field| field.field()).collect();
        let siblings: Vec<_> = self
            .fields_backed_by(field_name.as_ref())
            .into_iter()
            .filter(|sibling| sibling.is_text() || sibling.is_json())
            .map(|sibling| sibling.field())
            .filter(|sibling| !fields.contains(sibling))
            .collect();
        fields.extend(siblings);
        if fields.is_empty() {
            panic!(
                "cannot generate snippet for field {field_name} because it was not found in the index"
            )
        }

        SnippetGenerator::create_for_fields(
            &self.searcher,
            &self.make_query(query, expr_context),
            fields,
        )
        .unwrap_or_else(|err| {
            panic!("failed to create snippet generator for field: {field_name}... {err}")
        })
    }

    /// Every indexed field whose text comes from the heap column `column`.
    ///
    /// Two spellings reach the same column: a field configured with the column as its source,
    /// which the schema resolves on its own, and an indexed expression carrying an alias,
    /// whose source column is only recoverable by looking at the expression's `Var`. A column
    /// indexed solely through the second spelling has no field of its own, so this is also
    /// what lets highlighting resolve it at all.
    fn fields_backed_by(&self, column: &str) -> Vec<crate::schema::SearchField> {
        let mut fields = self.schema.fields_sourced_from(column);
        let Some(heaprel) = self.index_rel.heap_relation() else {
            return fields;
        };
        let expressions = self.index_rel.index_expressions();
        for (field, data) in self.schema.categorized_fields().iter() {
            let crate::postgres::utils::FieldSource::Expression { att_idx, .. } = data.source
            else {
                continue;
            };
            if fields.iter().any(|existing| existing == field) {
                continue;
            }
            let matches_column = unsafe {
                expressions
                    .get_ptr(att_idx)
                    .map(|expr| crate::postgres::utils::strip_tokenizer_cast(expr.cast()))
                    .and_then(|node| crate::nodecast!(Var, node))
                    .and_then(|var| crate::api::operator::attname_from_var(&heaprel, var))
                    .is_some_and(|attname| attname.as_ref() == column)
            };
            if matches_column {
                fields.push(field.clone());
            }
        }
        fields
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

    /// Search specific index segments with different queries for fully included vs partially included segments.
    ///
    /// Fully included segments evaluate against `self` (the unconstrained base query),
    /// while partially included segments also evaluate `partition_bounds`. Both variants share
    /// the base query weight, so scored text preparation runs only once per task.
    pub fn search_segments_with_range_filter(
        &self,
        segments: &PartitionSegments,
        partition_bounds: &SearchQueryInput,
    ) -> MultiSegmentSearchResults {
        let included = segments.included.iter().copied();
        let partially_included = segments.partially_included.iter().copied();
        let unconstrained_weight = Arc::new(LazyWeight::new(
            self.query().box_clone(),
            self.need_scores,
            self.searcher.clone(),
        ));
        let range_query = Box::new(ConstScoreQuery::new(
            self.make_query(partition_bounds, None),
            0.0,
        ));
        let constrained_weight = Arc::new(unconstrained_weight.and_query(range_query));
        let mut iterators = Vec::new();
        for (segment_ord, segment_reader) in self.segment_readers_in_segments(included) {
            iterators.push(ScorerIter::new(
                DeferredScorer::new(Arc::clone(&unconstrained_weight), segment_reader.clone()),
                segment_ord,
                segment_reader.clone(),
            ));
        }
        for (segment_ord, segment_reader) in self.segment_readers_in_segments(partially_included) {
            iterators.push(ScorerIter::new(
                DeferredScorer::new(Arc::clone(&constrained_weight), segment_reader.clone()),
                segment_ord,
                segment_reader.clone(),
            ));
        }
        MultiSegmentSearchResults {
            searcher: self.searcher.clone(),
            iterators,
            lazy_iterators: None,
            lazy_estimated_rows: None,
        }
    }

    /// Prepare reusable base query weights for Random Access (`RA`) point lookups in `lazy_sort` mode.
    ///
    /// When `disjunct_queries` is non-empty (`ScanMode::Disjunctive` scans), also prepares an
    /// optional `disjunct_weight` over the union of `disjunct_queries` so `search_ra_lookup` can
    /// accumulate BM25 scores for documents fetched via `RA` without materializing boolean
    /// match-tag columns in `Scanner`.
    pub fn prepare_ra_lookup_weights(
        &self,
        range_target: Option<&(PartitionSegments, SearchQueryInput)>,
        unconstrained_is_match_all: bool,
        disjunct_queries: &[SearchQueryInput],
    ) -> RaLookupWeights {
        let unconstrained = if unconstrained_is_match_all {
            RaBaseFilter::MatchAll
        } else {
            let contributes_score = self.need_scores && disjunct_queries.is_empty();
            let weight = Arc::new(LazyWeight::new(
                self.query().box_clone(),
                contributes_score,
                self.searcher.clone(),
            ));
            if contributes_score {
                RaBaseFilter::Scored(weight)
            } else {
                RaBaseFilter::Filter(weight)
            }
        };
        let constrained = range_target.map(|(segments, partition_bounds)| {
            let range_query = Box::new(ConstScoreQuery::new(
                self.make_query(partition_bounds, None),
                0.0,
            ));
            let filter = match &unconstrained {
                RaBaseFilter::MatchAll => RaBaseFilter::Filter(Arc::new(LazyWeight::new(
                    range_query,
                    false,
                    self.searcher.clone(),
                ))),
                RaBaseFilter::Filter(base) => {
                    RaBaseFilter::Filter(Arc::new(base.and_query(range_query)))
                }
                RaBaseFilter::Scored(base) => {
                    RaBaseFilter::Scored(Arc::new(base.and_query(range_query)))
                }
            };
            (segments.clone(), filter)
        });
        let disjunct_weight = self
            .need_scores
            .then(|| self.make_disjunct_union_query(disjunct_queries))
            .flatten()
            .map(|union_query| Arc::new(LazyWeight::new(union_query, true, self.searcher.clone())));

        RaLookupWeights {
            unconstrained,
            constrained,
            disjunct_weight,
        }
    }

    /// Execute a Random Access (`RA`) point lookup against the index using pre-prepared
    /// [`RaLookupWeights`], skipping segments that contain no un-emitted matching documents.
    ///
    /// When `score_threshold` is `Some(threshold)` (using strict `score > threshold` semantics),
    /// pushes the threshold down into Tantivy's [`tantivy::query::PruningScorer`] so non-matching or low-scoring
    /// segments and 128-doc blocks are skipped via Block-Max WAND before fast-field / CTID reads.
    pub fn search_ra_lookup(
        &self,
        weights: &RaLookupWeights,
        lookup_query: Box<dyn Query>,
        score_threshold: Option<Score>,
        emitted_docs: &mut crate::api::HashSet<DocAddress>,
    ) -> Vec<(SearchIndexScore, DocAddress)> {
        let lookup_weight = lookup_query
            .weight(EnableScoring::disabled_from_searcher(&self.searcher))
            .expect("lookup weight should be constructable");
        let mut results = Vec::new();

        if let Some((ref segments, ref constrained_filter)) = weights.constrained {
            for (segment_ord, segment_reader) in
                self.segment_readers_in_segments(segments.included.iter().copied())
            {
                self.search_ra_segment(
                    segment_ord,
                    segment_reader,
                    &weights.unconstrained,
                    weights.disjunct_weight.as_ref(),
                    lookup_weight.as_ref(),
                    score_threshold,
                    emitted_docs,
                    &mut results,
                );
            }
            for (segment_ord, segment_reader) in
                self.segment_readers_in_segments(segments.partially_included.iter().copied())
            {
                self.search_ra_segment(
                    segment_ord,
                    segment_reader,
                    constrained_filter,
                    weights.disjunct_weight.as_ref(),
                    lookup_weight.as_ref(),
                    score_threshold,
                    emitted_docs,
                    &mut results,
                );
            }
        } else {
            for (segment_ord, segment_reader) in self.candidate_segment_readers() {
                self.search_ra_segment(
                    segment_ord,
                    segment_reader,
                    &weights.unconstrained,
                    weights.disjunct_weight.as_ref(),
                    lookup_weight.as_ref(),
                    score_threshold,
                    emitted_docs,
                    &mut results,
                );
            }
        }

        results
    }

    /// Evaluates a single segment for [`Self::search_ra_lookup`] according to its [`RaBaseFilter`].
    #[allow(clippy::too_many_arguments)]
    fn search_ra_segment(
        &self,
        segment_ord: SegmentOrdinal,
        segment_reader: &SegmentReader,
        base_filter: &RaBaseFilter,
        disjunct_weight: Option<&Arc<LazyWeight>>,
        lookup_weight: &dyn Weight,
        score_threshold: Option<Score>,
        emitted_docs: &mut crate::api::HashSet<DocAddress>,
        results: &mut Vec<(SearchIndexScore, DocAddress)>,
    ) {
        if let Some(base_weight) = base_filter.scored() {
            // When `base_weight` is the sole score source, push `threshold` directly into its
            // `PruningScorer` (Block-Max WAND).
            if disjunct_weight.is_none()
                && let Some(threshold) = score_threshold
            {
                let mut pruning_scorer = base_weight
                    .get()
                    .pruning_scorer(segment_reader, 1.0, threshold)
                    .expect("pruning scorer should be constructable");
                intersect_scorer_with_lookup(
                    segment_ord,
                    segment_reader,
                    pruning_scorer.as_mut(),
                    true,
                    lookup_weight,
                    None,
                    Some(threshold),
                    None,
                    emitted_docs,
                    results,
                );
            } else {
                let mut base_scorer = base_weight
                    .get()
                    .scorer(segment_reader, 1.0)
                    .expect("base scorer should be constructable");
                intersect_scorer_with_lookup(
                    segment_ord,
                    segment_reader,
                    base_scorer.as_mut(),
                    true,
                    lookup_weight,
                    disjunct_weight,
                    score_threshold,
                    None,
                    emitted_docs,
                    results,
                );
            }
            return;
        }

        // Base score is 0.0, so a non-negative strict `score > threshold` can only be
        // satisfied by documents matching `disjunct_weight` above `threshold`. Driving from the
        // base filter instead would probe the lookup once per base document, which a dense
        // filter makes unaffordable.
        if let Some(threshold) = score_threshold
            && threshold >= 0.0
        {
            if let Some(dw) = disjunct_weight {
                let mut pruning_scorer = dw
                    .get()
                    .pruning_scorer(segment_reader, 1.0, threshold)
                    .expect("pruning scorer should be constructable");
                intersect_scorer_with_lookup(
                    segment_ord,
                    segment_reader,
                    pruning_scorer.as_mut(),
                    true,
                    lookup_weight,
                    None,
                    Some(threshold),
                    base_filter.filter(),
                    emitted_docs,
                    results,
                );
            }
            return;
        }

        collect_lookup_only_segment(
            segment_ord,
            segment_reader,
            lookup_weight,
            base_filter.filter(),
            disjunct_weight,
            score_threshold,
            EstimateContext::for_single_segment(&self.segment_stats_snapshot),
            emitted_docs,
            results,
        );
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
        // holds `Rc` schema state that is only ever touched on the Postgres main thread via
        // `run_on_main_thread` and dropped via `MainThread`. `Send`/`Sync` are required so
        // `SearchResults::new_lazy` can store the iterator as `Box<dyn Iterator<...> + Send + Sync>`.
        unsafe impl Send for ParallelCandidateIterator {}
        unsafe impl Sync for ParallelCandidateIterator {}
        impl Iterator for ParallelCandidateIterator {
            type Item = SegmentOrdinal;
            fn next(&mut self) -> Option<Self::Item> {
                loop {
                    if crate::postgres::customscan::mpp::check_for_interrupts_or_cancel() {
                        return None;
                    }
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
    ) -> TopKSearch {
        // Do an un-ordered search.
        TopKSearch::new(
            self.search_segments(segment_ids)
                .skip(offset)
                .take(n)
                .collect(),
            None,
        )
    }

    /// Search specific range-partitioned segments for an unordered window of matching documents.
    ///
    /// Fully included segments evaluate against the unconstrained base query, while partially
    /// included segments also evaluate `partition_bounds`.
    pub fn search_top_k_unordered_segments_with_range_filter(
        &self,
        segments: &PartitionSegments,
        partition_bounds: &SearchQueryInput,
        window: TopKWindow,
    ) -> TopKSearch {
        TopKSearch::new(
            self.search_segments_with_range_filter(segments, partition_bounds)
                .skip(window.offset.0)
                .take(window.limit.0)
                .collect(),
            None,
        )
    }

    /// Mirrors the sort-shape branch in `search_top_k_in_segments`.
    pub fn orderby_uses_score_desc_topk_collector(orderby_info: &[OrderByInfo]) -> bool {
        matches!(
            orderby_info.first(),
            Some(OrderByInfo {
                feature: OrderByFeature::Score { .. },
                direction,
            }) if !direction.is_asc()
        )
    }

    /// Search the Tantivy index for the Top K matching documents in specific segments.
    /// Segments are consumed lazily, except when local vector routing collects all segments together.
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
    /// per-segment JSON in [`TopKSearch::segment_info`].
    #[allow(clippy::too_many_arguments)]
    pub fn search_top_k_in_segments(
        &self,
        segments: impl Iterator<Item = SegmentId>,
        orderby_info: &[OrderByInfo],
        n: usize,
        offset: usize,
        aux_collector: Option<TopKAuxiliaryCollector>,
        parallel_state_holding_shared_threshold: Option<*mut crate::postgres::ParallelScanState>,
        routing: &VectorRouting,
    ) -> TopKSearch {
        self.search_top_k_in_segments_internal(
            segments.map(|id| (id, SegmentQueryKind::Unconstrained)),
            None,
            orderby_info,
            TopKWindow {
                limit: TopKLimit(n),
                offset: TopKOffset(offset),
            },
            aux_collector,
            parallel_state_holding_shared_threshold,
            routing,
        )
    }

    /// Search specific range-partitioned segments for the Top-K matching documents.
    ///
    /// Fully included segments evaluate against the unconstrained base query, while partially
    /// included segments also evaluate `partition_bounds`.
    pub fn search_top_k_segments_with_range_filter(
        &self,
        segments: &PartitionSegments,
        partition_bounds: &SearchQueryInput,
        orderby_info: &[OrderByInfo],
        window: TopKWindow,
        routing: &VectorRouting,
    ) -> TopKSearch {
        let segment_iter = segments
            .included
            .iter()
            .copied()
            .map(|id| (id, SegmentQueryKind::Unconstrained))
            .chain(
                segments
                    .partially_included
                    .iter()
                    .copied()
                    .map(|id| (id, SegmentQueryKind::RangeConstrained)),
            );
        self.search_top_k_in_segments_internal(
            segment_iter,
            Some(partition_bounds),
            orderby_info,
            window,
            None,
            None,
            routing,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn search_top_k_in_segments_internal(
        &self,
        segments: impl Iterator<Item = (SegmentId, SegmentQueryKind)>,
        partition_bounds: Option<&SearchQueryInput>,
        orderby_info: &[OrderByInfo],
        window: TopKWindow,
        aux_collector: Option<TopKAuxiliaryCollector>,
        parallel_state_holding_shared_threshold: Option<*mut crate::postgres::ParallelScanState>,
        routing: &VectorRouting,
    ) -> TopKSearch {
        let n = window.limit.0;
        let offset = window.offset.0;
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
                        TopKSearch::new_for_discarded_field(self.top_in_segments(
                            segments,
                            partition_bounds,
                            (computer, order),
                            erased_features,
                            window,
                            aux_collector,
                        ))
                    }};
                }

                // A range is indexed as a tantivy JSON object, so `value_type()` below reports
                // `Type::Json`, which no arm handles — it would reach the catch-all `panic!`.
                // Dispatch on the Postgres type instead: tantivy cannot distinguish a range's
                // JSON from a user-supplied JSON column, and only the former is sortable (see
                // `SearchField::is_sortable`). `SortByRange` compares the bound sub-columns the
                // way Postgres' `range_cmp` does.
                if matches!(field.field_type(), SearchFieldType::Range(_)) {
                    return TopKSearch::new_for_discarded_field(self.top_in_segments(
                        segments,
                        partition_bounds,
                        (SortByRange::for_field(sort_field), order),
                        erased_features,
                        window,
                        aux_collector,
                    ));
                }

                match field.field_entry().field_type().value_type() {
                    tantivy::schema::Type::Str => {
                        TopKSearch::new_for_discarded_field(self.top_in_segments(
                            segments,
                            partition_bounds,
                            (SortByString::for_field(sort_field), order),
                            erased_features,
                            window,
                            aux_collector,
                        ))
                    }
                    tantivy::schema::Type::U64 => sort_fast_value!(u64),
                    tantivy::schema::Type::I64 => sort_fast_value!(i64),
                    tantivy::schema::Type::F64 => sort_fast_value!(f64),
                    tantivy::schema::Type::Bool => sort_fast_value!(bool),
                    tantivy::schema::Type::Date => sort_fast_value!(DateTime),
                    tantivy::schema::Type::Bytes => {
                        TopKSearch::new_for_discarded_field(self.top_in_segments(
                            segments,
                            partition_bounds,
                            (SortByBytes::for_field(sort_field), order),
                            erased_features,
                            window,
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
                    segments,
                    partition_bounds,
                    (computer, order),
                    erased_features,
                    window,
                    aux_collector,
                );
                TopKSearch::new_for_score(
                    top_docs.into_iter().map(|((f, _), doc)| (f, doc)),
                    aggregation_results,
                )
            }
            OrderByInfo {
                feature: OrderByFeature::Score { .. },
                direction,
            } => self.top_by_score_in_segments(
                segments,
                partition_bounds,
                *direction,
                window,
                aux_collector,
                parallel_state_holding_shared_threshold,
            ),
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
                // Only the field ID is needed; constructing a SearchField loads PostgreSQL
                // metadata and tokenizer settings for every indexed field.
                let tantivy_field = self
                    .schema
                    .tantivy_schema()
                    .get_field(&name.root())
                    .expect("vector field should exist in index schema");
                let query_vector = query_vector
                    .resolved()
                    .expect("vector ORDER BY query vector was never resolved")
                    .to_vec();
                let max_scan_levels = gucs::vector_max_scan_levels();
                let mut collector = TopDocs::with_limit(n)
                    .and_offset(offset)
                    .order_by_similarity(tantivy_field, query_vector)
                    .with_probe_params(probe_params())
                    .with_max_scan_levels(max_scan_levels);
                if let Some(state) = parallel_state_holding_shared_threshold {
                    collector = collector.with_shared_threshold(std::sync::Arc::new(
                        crate::postgres::shared_threshold::new_score_threshold(state),
                    ));
                }

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
                    self.segment_readers_with_query_kind(segments)
                        .inspect(|(_, reader, _)| {
                            collected_ids.borrow_mut().push(reader.segment_id());
                        });
                let batches: Box<dyn Iterator<Item = Vec<_>>> = match routing {
                    VectorRouting::Shared(..) => Box::new(readers.map(|reader| vec![reader])),
                    VectorRouting::Local => Box::new(std::iter::once(readers.collect())),
                };
                // Fruit is `VectorSimilarityFruit` — hits plus per-segment
                // ProbeStats — for every tie-break shape.
                let tie_break_count = tie_breaks.len();
                let mut tie_breaks = tie_breaks.into_iter();
                let mut next = || tie_breaks.next().expect("tie-break feature should exist");
                let (fruit, aggregation_results) = match tie_break_count {
                    0 => self.collect_vector_maybe_auxiliary(
                        batches,
                        tantivy_field,
                        partition_bounds,
                        collector,
                        aux_collector,
                        routing,
                    ),
                    1 => self.collect_vector_maybe_auxiliary(
                        batches,
                        tantivy_field,
                        partition_bounds,
                        collector.with_tie_break(next()),
                        aux_collector,
                        routing,
                    ),
                    2 => self.collect_vector_maybe_auxiliary(
                        batches,
                        tantivy_field,
                        partition_bounds,
                        collector.with_tie_break((next(), next())),
                        aux_collector,
                        routing,
                    ),
                    3 => self.collect_vector_maybe_auxiliary(
                        batches,
                        tantivy_field,
                        partition_bounds,
                        collector.with_tie_break((next(), next(), next())),
                        aux_collector,
                        routing,
                    ),
                    4 => self.collect_vector_maybe_auxiliary(
                        batches,
                        tantivy_field,
                        partition_bounds,
                        collector.with_tie_break((next(), next(), next(), next())),
                        aux_collector,
                        routing,
                    ),
                    x => panic!(
                        "Unsupported sort-field count: {}. At most {MAX_TOPK_FEATURES} are supported.",
                        x + 1
                    ),
                };
                let segment_ids = collected_ids.into_inner();
                let mut segment_info = probe_stats_to_segment_info(&segment_ids, &fruit.stats);
                if let Some(first_segment) = segment_ids.first()
                    && let Some(serde_json::Value::Object(stats)) =
                        segment_info.get_mut(first_segment)
                {
                    let segment_scan_init = stats
                        .get("scan_init_ns")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    stats.insert(
                        "scan_init_ns".to_string(),
                        segment_scan_init.saturating_add(self.scan_init_ns).into(),
                    );
                }
                if let Some(stats) = &self.directory.io_stats {
                    stats.attach(&mut segment_info);
                }
                let scored_results: Vec<(SearchIndexScore, DocAddress)> = fruit
                    .results
                    .into_iter()
                    .map(|(score, doc_address)| (SearchIndexScore { bm25: score }, doc_address))
                    .collect();
                TopKSearch::with_segment_info(scored_results, aggregation_results, segment_info)
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
        segments: impl Iterator<Item = (SegmentId, SegmentQueryKind)>,
        partition_bounds: Option<&SearchQueryInput>,
        first_feature: S,
        mut erased_features: ErasedFeatures,
        window: TopKWindow,
        aux_collector: Option<TopKAuxiliaryCollector>,
    ) -> TopKWithAggregate<S::SortKey>
    where
        S: SortKeyComputer + Clone + Send + 'static,
    {
        let n = window.limit.0;
        let offset = window.offset.0;
        let readers = self.segment_readers_with_query_kind(segments);
        // if last erased feature is score, then we need to return the score
        match erased_features.len() {
            0 => {
                let top_docs_collector = TopDocs::with_limit(n)
                    .and_offset(offset)
                    .order_by::<S::SortKey>(first_feature);

                let (top_docs, aggregation_results) = self.collect_maybe_auxiliary(
                    readers,
                    partition_bounds,
                    top_docs_collector,
                    aux_collector,
                );

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

                let (top_docs, aggregation_results) = self.collect_maybe_auxiliary(
                    readers,
                    partition_bounds,
                    top_docs_collector,
                    aux_collector,
                );

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

                let (top_docs, aggregation_results) = self.collect_maybe_auxiliary(
                    readers,
                    partition_bounds,
                    top_docs_collector,
                    aux_collector,
                );

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

                let (top_docs, aggregation_results) = self.collect_maybe_auxiliary(
                    readers,
                    partition_bounds,
                    top_docs_collector,
                    aux_collector,
                );

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

                let (top_docs, aggregation_results) = self.collect_maybe_auxiliary(
                    readers,
                    partition_bounds,
                    top_docs_collector,
                    aux_collector,
                );

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
        segments: impl Iterator<Item = (SegmentId, SegmentQueryKind)>,
        partition_bounds: Option<&SearchQueryInput>,
        sortdir: SortDirection,
        window: TopKWindow,
        aux_collector: Option<TopKAuxiliaryCollector>,
        parallel_state_holding_shared_threshold: Option<*mut crate::postgres::ParallelScanState>,
    ) -> TopKSearch {
        let n = window.limit.0;
        let offset = window.offset.0;
        // NOTE: which `sortdir` arm uses the Block-WAND pruning collector below
        // (only Desc, via `order_by::<Score>`) defines
        // `orderby_uses_score_desc_topk_collector` -- the plan-time gate that costs
        // ordered TopK as serial-vs-parallel (#4664). If you change which direction
        // prunes here, update that predicate to match, or the planner will mis-cost.
        let readers = self.segment_readers_with_query_kind(segments);
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

                let (top_docs, aggregation_results) = self.collect_maybe_auxiliary(
                    readers,
                    partition_bounds,
                    top_docs_collector,
                    aux_collector,
                );

                TopKSearch::new_for_score(
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

                let (top_docs, aggregation_results) = self.collect_maybe_auxiliary(
                    readers,
                    partition_bounds,
                    top_docs_collector,
                    aux_collector,
                );

                TopKSearch::new_for_score(top_docs, aggregation_results)
            }
        }
    }

    /// Whether [`Self::estimate_docs`] can use segment metadata and term-dictionary statistics
    /// instead of opening postings lists.
    pub fn supports_statistics_estimation(&self) -> bool {
        self.estimate_from_statistics
    }

    /// Given an estimate of the total number of rows in the relation, return estimates of:
    /// 1. The number of rows which will be matched by the configured query.
    /// 2. The total number of rows in the index (estimated if total_docs is Unknown).
    /// 3. Tantivy's relative cost to drive the configured query's docset.
    ///
    /// Prunes segments using `.stats` first, sums dictionary-backed estimates across all
    /// surviving candidate segments when [`Self::supports_statistics_estimation`] holds and every
    /// candidate segment supports [`QueryEstimate`], and otherwise samples the largest surviving
    /// candidate segment and scales its estimates to the candidate portion of the relation.
    ///
    /// TODO: <https://github.com/paradedb/paradedb/issues/6976> will replace per-segment sampling
    /// with persistent index-wide summary statistics, avoiding both the assumption that the
    /// largest surviving segment is representative of the rest and the plan-time cost of opening
    /// term dictionaries across every surviving segment.
    pub fn estimate_docs(&self, total_docs: RowEstimate) -> DocsEstimate {
        if self.segment_readers().is_empty() {
            return DocsEstimate {
                matching_docs: 0,
                total_docs: 0,
                query_cost: 0,
            };
        }

        // Fall back to the index's own doc count when the caller's heap estimate is unknown or 0;
        // sampled candidate segments are then scaled relative to their share of `total_docs`.
        let total_docs = match total_docs {
            RowEstimate::Known(total_docs) if total_docs > 0 => total_docs,
            _ => self.total_docs(),
        };

        let Some(sample) = self.estimation_sample(total_docs as f64) else {
            return DocsEstimate {
                matching_docs: 0,
                total_docs,
                query_cost: 0,
            };
        };

        if let Some(docs_estimate) = self.estimate_self_across_candidates(&sample, total_docs) {
            return docs_estimate;
        }

        let largest_reader = sample.largest_reader;
        let single_ctx = EstimateContext::for_single_segment(&self.segment_stats_snapshot);
        let (count, mut cost) = self
            .estimate_from_statistics
            .then(|| {
                self.query
                    .estimate_docs_with_context(largest_reader, single_ctx)
                    .ok()
                    .flatten()
            })
            .flatten()
            .map(|estimate| (estimate.live_docs(largest_reader), estimate.cost))
            .unwrap_or_else(|| scorer_estimate(self.weight().as_ref(), largest_reader));
        if let Some(shortest_posting_list) = self.shortest_posting_list(largest_reader) {
            cost = cost.max(shortest_posting_list);
        }

        DocsEstimate {
            matching_docs: scale_largest_segment_estimate(count, sample.largest_doc_proportion)
                as usize,
            total_docs,
            query_cost: scale_largest_segment_estimate(cost, sample.largest_doc_proportion),
        }
    }

    /// Sums dictionary-backed [`QueryEstimate`] live-doc and cost estimates for `self.query`
    /// across `sample.candidates` and scales them to the candidate share of `total_docs`,
    /// returning `None` if statistics estimation is disabled or unsupported on any candidate.
    fn estimate_self_across_candidates(
        &self,
        sample: &EstimationSample<'_>,
        total_docs: u64,
    ) -> Option<DocsEstimate> {
        if !self.estimate_from_statistics {
            return None;
        }
        let ctx =
            EstimateContext::for_candidates(&self.segment_stats_snapshot, sample.candidates.len());
        let mut matching_docs = 0u64;
        let mut query_cost = 0u64;
        for &reader in &sample.candidates {
            let estimate = self
                .query
                .estimate_docs_with_context(reader, ctx)
                .ok()
                .flatten()?;
            let mut seg_cost = estimate.cost;
            if let Some(shortest_posting_list) = self.shortest_posting_list(reader) {
                seg_cost = seg_cost.max(shortest_posting_list);
            }
            matching_docs = matching_docs.saturating_add(estimate.live_docs(reader));
            query_cost = query_cost.saturating_add(seg_cost);
        }
        Some(DocsEstimate {
            matching_docs: scale_largest_segment_estimate(
                matching_docs,
                sample.candidates_doc_proportion,
            ) as usize,
            total_docs,
            query_cost: scale_largest_segment_estimate(
                query_cost,
                sample.candidates_doc_proportion,
            ),
        })
    }

    /// Sums unscaled dictionary-backed [`QueryEstimate`] live-doc counts for `query` across
    /// `candidates`, returning `None` if any candidate segment does not support [`QueryEstimate`].
    fn estimate_live_docs_across_candidates(
        &self,
        query: &dyn Query,
        candidates: &[&SegmentReader],
    ) -> Option<u64> {
        let ctx = EstimateContext::for_candidates(&self.segment_stats_snapshot, candidates.len());
        let mut matching_docs = 0u64;
        for &reader in candidates {
            let estimate = query
                .estimate_docs_with_context(reader, ctx)
                .ok()
                .flatten()?;
            matching_docs = matching_docs.saturating_add(estimate.live_docs(reader));
        }
        Some(matching_docs)
    }

    /// Returns the candidate segment readers and scaling proportions for plan-time estimation.
    ///
    /// Populates `self.pruning_estimate` over all candidate segments (including mutable segments),
    /// then excludes mutable segments from the estimation sample whenever at least one persisted
    /// segment exists in the reader so plan-time estimation never materializes mutable segments
    /// from the heap.
    ///
    /// TODO: <https://github.com/paradedb/paradedb/issues/6976> will replace this largest-segment
    /// sampling heuristic with index-wide summary statistics.
    fn estimation_sample(&self, total_docs: f64) -> Option<EstimationSample<'_>> {
        let mut candidates: Vec<(SegmentOrdinal, &SegmentReader)> =
            self.candidate_segment_readers().collect();
        let all_candidate_docs: u64 = candidates
            .iter()
            .map(|(_, r)| self.segment_num_docs(r))
            .sum();
        let _ = self
            .pruning_estimate
            .set(self.pruning_estimate_from_candidates(&candidates));

        let persisted_reader_docs: u64 = self
            .segment_readers()
            .iter()
            .filter(|r| !self.directory.is_mutable(&r.segment_id()))
            .map(|r| self.segment_num_docs(r))
            .sum();
        let (candidate_docs, reader_docs) = if persisted_reader_docs > 0 {
            candidates.retain(|(_, r)| !self.directory.is_mutable(&r.segment_id()));
            let candidate_docs = candidates
                .iter()
                .map(|(_, r)| self.segment_num_docs(r))
                .sum();
            (candidate_docs, persisted_reader_docs)
        } else {
            let reader_docs = self
                .segment_readers()
                .iter()
                .map(|r| self.segment_num_docs(r))
                .sum();
            (all_candidate_docs, reader_docs)
        };

        if candidate_docs == 0 {
            return None;
        }
        let (_, largest_reader) = candidates
            .iter()
            .copied()
            .max_by_key(|&(ord, r)| (self.segment_num_docs(r), ord))?;
        let reader_proportion = reader_docs.max(1) as f64 / total_docs.max(1.0);
        let largest_doc_proportion = (self.segment_num_docs(largest_reader) as f64
            / candidate_docs as f64)
            * reader_proportion;
        let candidates = candidates.into_iter().map(|(_, r)| r).collect();
        Some(EstimationSample {
            candidates,
            candidate_docs,
            candidates_doc_proportion: reader_proportion,
            largest_reader,
            largest_doc_proportion,
        })
    }

    /// Estimates the fraction `[0.0, 1.0]` of documents in this index matched by `query_input`.
    pub fn estimate_query_selectivity(&self, query_input: &SearchQueryInput) -> f64 {
        let heuristic = gucs::enable_heuristic_selectivity()
            .then(|| query_input.expensive_selectivity_heuristic())
            .flatten();
        if let Some(selectivity) = heuristic {
            return selectivity.clamp(0.0, 1.0);
        }
        if query_input.has_postgres_expressions() || query_input.has_parameters() {
            return query_input.selectivity_heuristic().clamp(0.0, 1.0);
        }
        let Some(sample) = self.estimation_sample(self.total_docs() as f64) else {
            return 0.0;
        };
        let query = self.make_query(query_input, None);
        if query_input.supports_statistics_estimation()
            && sample.candidate_docs > 0
            && let Some(count) =
                self.estimate_live_docs_across_candidates(query.as_ref(), &sample.candidates)
        {
            return (count as f64 / sample.candidate_docs as f64).clamp(0.0, 1.0);
        }
        let largest_reader = sample.largest_reader;
        let seg_docs = self.segment_num_docs(largest_reader);
        if seg_docs == 0 {
            return 0.0;
        }
        let single_ctx = EstimateContext::for_single_segment(&self.segment_stats_snapshot);
        let count = query_input
            .supports_statistics_estimation()
            .then(|| {
                query
                    .estimate_docs_with_context(largest_reader, single_ctx)
                    .ok()
                    .flatten()
            })
            .flatten()
            .map(|estimate| estimate.live_docs(largest_reader))
            .unwrap_or_else(|| {
                let weight = query
                    .weight(EnableScoring::disabled_from_searcher(&self.searcher))
                    .expect("weight should be constructable");
                weight
                    .scorer(largest_reader, 1.0)
                    .expect("creating scorer for estimation should not fail")
                    .size_hint() as u64
            });
        (count as f64 / seg_docs as f64).clamp(0.0, 1.0)
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
        let Some(sample) = self.estimation_sample(total_docs) else {
            query_tree.traverse_mut(0, &mut |node, _depth| {
                node.estimated_docs = Some(0);
            });
            return;
        };

        // Keep EXPLAIN's child estimates on the same path as the whole query.
        let estimate_from_statistics = query_tree.query.supports_statistics_estimation();
        self.estimate_node_recursive(query_tree, &sample, parser, estimate_from_statistics);
    }

    fn estimate_node_recursive<QueryParserCtor: Fn() -> QueryParser>(
        &self,
        node: &mut QueryWithEstimates,
        sample: &EstimationSample<'_>,
        parser: &QueryParserCtor,
        estimate_from_statistics: bool,
    ) {
        // First, recursively estimate all children
        for child in node.children_mut() {
            self.estimate_node_recursive(child, sample, parser, estimate_from_statistics);
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

        if estimate_from_statistics
            && let Some(count) = self
                .estimate_live_docs_across_candidates(tantivy_query.as_ref(), &sample.candidates)
        {
            node.set_estimate(scale_largest_segment_estimate(
                count,
                sample.candidates_doc_proportion,
            ) as usize);
            return;
        }
        let largest_reader = sample.largest_reader;
        let single_ctx = EstimateContext::for_single_segment(&self.segment_stats_snapshot);
        if estimate_from_statistics
            && let Ok(Some(estimate)) =
                tantivy_query.estimate_docs_with_context(largest_reader, single_ctx)
        {
            node.set_estimate(scale_largest_segment_estimate(
                estimate.live_docs(largest_reader),
                sample.largest_doc_proportion,
            ) as usize);
            return;
        }

        let weight = tantivy_query
            .weight(enable_scoring(node.query.need_scores(), &self.searcher))
            .expect("creating weight for estimation should not fail");

        let (count, _) = scorer_estimate(weight.as_ref(), largest_reader);
        let estimated =
            scale_largest_segment_estimate(count, sample.largest_doc_proportion) as usize;

        node.set_estimate(estimated);
    }

    pub fn collect<C: Collector>(&self, collector: C) -> C::Fruit {
        self.collect_readers(self.candidate_segment_readers(), collector)
    }

    /// [`Self::collect`] over the given segments only, consumed lazily like
    /// [`Self::search_segments`]: a parallel participant collects over the segments
    /// it has claimed and merges with the other participants afterwards.
    pub fn collect_segments<C: Collector>(
        &self,
        segment_ids: impl Iterator<Item = SegmentId>,
        collector: C,
    ) -> C::Fruit {
        self.collect_readers(self.segment_readers_in_segments(segment_ids), collector)
    }

    fn collect_readers<'a, C: Collector>(
        &'a self,
        readers: impl Iterator<Item = (SegmentOrdinal, &'a SegmentReader)>,
        collector: C,
    ) -> C::Fruit {
        // Tantivy's `search` validates the collector before any segment is visited; do the same,
        // so a collector over an unknown field fails even when every segment is pruned.
        collector
            .check_schema(self.searcher.schema())
            .expect("search should not fail");
        let weight = self
            .query
            .weight(
                enable_scoring(self.need_scores, &self.searcher)
                    .with_bitmap_postings(collector.supports_bitmap_collection()),
            )
            .expect("creating a Weight from a Query should not fail");
        let fruits = self.collect_segment_readers(readers, &collector, weight.as_ref());
        collector
            .merge_fruits(fruits)
            .expect("search should not fail")
    }

    fn collect_vector_maybe_auxiliary<'a, S>(
        &self,
        reader_batches: impl Iterator<Item = Vec<(SegmentOrdinal, &'a SegmentReader, SegmentQueryKind)>>,
        vector_field: Field,
        partition_bounds: Option<&SearchQueryInput>,
        builder: TopDocsByVectorSimilarityBuilder<f32, S>,
        aux_collector: Option<TopKAuxiliaryCollector>,
        routing: &VectorRouting,
    ) -> (
        VectorSimilarityFruit,
        Option<IntermediateAggregationResults>,
    )
    where
        S: SortKeyComputer + Clone + Send + Sync + 'static,
    {
        let base = Arc::new(LazyWeight::new(
            self.query().box_clone(),
            false,
            self.searcher.clone(),
        ));
        let constrained = partition_bounds.map(|bounds| {
            Arc::new(base.and_query(Box::new(ConstScoreQuery::new(
                self.make_query(bounds, None),
                0.0,
            ))))
        });
        let (aggregation, visibility) = match aux_collector {
            Some(TopKAuxiliaryCollector {
                aggregation_collector,
                vischeck,
            }) => (
                Some(aggregation_collector),
                vischeck.map(|checker| {
                    MVCCFilterCollector::new(tantivy::collector::DocSetCollector, checker)
                }),
            ),
            None => (None, None),
        };
        if let Some(stats) = &self.directory.io_stats {
            stats.reset();
        }
        let on_segment = |ord: SegmentOrdinal| {
            pgrx::check_for_interrupts();
            if let Some(stats) = &self.directory.io_stats {
                stats.end_segment(self.searcher.segment_reader(ord).segment_id());
            }
        };
        let shared_work = routing.work_for_pull();
        let mut collector: Option<TopDocsByVectorSimilarity<f32, S>> = None;
        let mut fruits = Vec::new();
        let mut aggregation_fruits = Vec::new();
        for readers in reader_batches {
            pgrx::check_for_interrupts();
            let mut weight = VectorFilterWeight {
                base: Arc::clone(&base),
                visible_docs: Default::default(),
                constrained: constrained.clone(),
                constrained_segments: readers
                    .iter()
                    .filter_map(|(_, reader, kind)| {
                        matches!(kind, SegmentQueryKind::RangeConstrained)
                            .then_some(reader.segment_id())
                    })
                    .collect(),
            };
            if let Some(visibility) = &visibility {
                for &(ordinal, reader, _) in &readers {
                    let (_, docs) = visibility
                        .collect_segment(&weight, ordinal, reader)
                        .expect("vector visibility should collect");
                    let mut visible = tantivy_common::BitSet::with_max_value(reader.max_doc());
                    for doc in docs {
                        visible.insert(doc);
                    }
                    weight.visible_docs.insert(reader.segment_id(), visible);
                    on_segment(ordinal);
                }
            }
            let use_centroids = matches!(routing, VectorRouting::Local) && readers.len() > 1;
            let segments = readers
                .iter()
                .map(|&(ordinal, reader, _)| {
                    let mut segment = VectorSegment::from((ordinal, reader));
                    if use_centroids {
                        segment.partition_centroid = SegmentStats::of_reader(reader)
                            .expect("segment stats should be readable")
                            .and_then(|stats| {
                                stats
                                    .vector_partition(vector_field)
                                    .expect("partition centroid should be readable")
                            })
                            .map(Into::into);
                        on_segment(ordinal);
                    }
                    segment
                })
                .collect::<Vec<_>>();
            match routing {
                VectorRouting::Local => {
                    collector = Some(
                        builder
                            .clone()
                            .build_for_segments(&weight, &segments, on_segment)
                            .inspect_err(|error| report_vector_error(self.index_rel.name(), error))
                            .expect("vector scan preparation should not fail"),
                    );
                }
                VectorRouting::Shared(shared, _) => {
                    let prepared = builder
                        .prepare(&weight, &segments, shared.budget, on_segment)
                        .expect("vector filters should prepare");
                    let routing_segments = prepared.routing_segments();
                    let inputs = if routing_segments.is_empty() {
                        prepared.with_candidates(Vec::new(), shared_work.clone())
                    } else {
                        let results = shared
                            .get()
                            .expect("vector routing results should be available");
                        let candidates = routing_segments
                            .iter()
                            .flat_map(|segment| {
                                results
                                    .candidates
                                    .get(&segment.ordinal)
                                    .into_iter()
                                    .flatten()
                                    .copied()
                            })
                            .collect();
                        prepared
                            .with_candidates(candidates, shared_work.clone())
                            .map(|inputs| {
                                inputs.with_routing_statistics(
                                    results
                                        .statistics
                                        .iter()
                                        .filter(|stat| {
                                            segments.iter().any(|segment| {
                                                segment.ordinal == stat.segment_ordinal
                                            })
                                        })
                                        .cloned()
                                        .collect(),
                                )
                            })
                    }
                    .expect("vector candidates should match prepared segments");
                    if let Some(collector) = &mut collector {
                        collector
                            .set_inputs(inputs)
                            .expect("vector assignment should match this query");
                    } else {
                        collector = Some(builder.clone().build(inputs));
                    }
                }
            }
            let collector = collector.as_ref().unwrap();
            fruits.extend(
                collector
                    .collect_segments(&weight, &segments, on_segment)
                    .inspect_err(|error| report_vector_error(self.index_rel.name(), error))
                    .expect("vector collection should not fail"),
            );
            for &(ordinal, reader, _) in &readers {
                if let Some(aggregation) = &aggregation {
                    aggregation_fruits.push(
                        aggregation
                            .collect_segment(&weight, ordinal, reader)
                            .expect("aggregation should collect"),
                    );
                }
                on_segment(ordinal);
            }
        }
        let collector = collector.unwrap_or_else(|| {
            builder
                .build_for_segments(base.get(), &[], on_segment)
                .expect("empty vector scan should prepare")
        });
        collector
            .check_schema(self.searcher.schema())
            .expect("vector schema should be valid");
        let aggregation = aggregation.map(|aggregation| {
            aggregation
                .merge_fruits(aggregation_fruits)
                .expect("aggregation should merge")
        });
        (
            collector
                .merge_fruits(fruits)
                .expect("vector results should merge"),
            aggregation,
        )
    }

    /// Collect for the given Collector, optionally paired with / wrapped with the given auxiliary
    /// Collector(s).
    fn collect_maybe_auxiliary<'a, C: Collector>(
        &self,
        readers: impl Iterator<Item = (SegmentOrdinal, &'a SegmentReader, SegmentQueryKind)>,
        partition_bounds: Option<&SearchQueryInput>,
        top_docs_collector: C,
        aux_collector: Option<TopKAuxiliaryCollector>,
    ) -> (C::Fruit, Option<IntermediateAggregationResults>) {
        let Some(aux_collector) = aux_collector else {
            // No auxiliary collector.
            let fruits = self.collect_segment_readers_by_query_kind(
                readers,
                partition_bounds,
                &top_docs_collector,
            );
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
            let fruits =
                self.collect_segment_readers_by_query_kind(readers, partition_bounds, &collector);
            let (top_docs, aggregation_results) = collector
                .merge_fruits(fruits)
                .expect("should be able to merge Top K in segment");
            (top_docs, Some(aggregation_results))
        } else {
            let fruits = self.collect_segment_readers_by_query_kind(
                readers,
                partition_bounds,
                &compound_collector,
            );
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
        self.segment_stats_snapshot.segment_ordinal(*segment_id)
    }

    fn candidates(
        &self,
        ordinals: impl Iterator<Item = SegmentOrdinal>,
    ) -> impl Iterator<Item = (SegmentOrdinal, &SegmentReader)> {
        ordinals
            .filter(move |ord| self.is_candidate(*ord))
            .map(move |ord| (ord, self.searcher.segment_reader(ord)))
    }

    pub fn candidate_segment_readers(
        &self,
    ) -> impl Iterator<Item = (SegmentOrdinal, &SegmentReader)> {
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

    /// NOTE: Like `segment_readers_in_segments`, this method must consume `segments` lazily.
    fn segment_readers_with_query_kind(
        &self,
        segments: impl Iterator<Item = (SegmentId, SegmentQueryKind)>,
    ) -> impl Iterator<Item = (SegmentOrdinal, &SegmentReader, SegmentQueryKind)> {
        segments.filter_map(move |(segment_id, query_kind)| {
            let ord = self.segment_ordinal_by_id(&segment_id).unwrap_or_else(|| {
                panic!(
                    "segment {segment_id} should exist in this {} reader's {} segments",
                    self.directory.mvcc_style_name(),
                    self.searcher.segment_readers().len()
                )
            });
            self.is_candidate(ord)
                .then(|| (ord, self.searcher.segment_reader(ord), query_kind))
        })
    }

    fn collect_segment_readers<'a, C: Collector>(
        &self,
        readers: impl Iterator<Item = (SegmentOrdinal, &'a SegmentReader)>,
        collector: &C,
        weight: &dyn Weight,
    ) -> Vec<<<C as Collector>::Child as SegmentCollector>::Fruit> {
        if let Some(stats) = &self.directory.io_stats {
            stats.reset();
        }
        readers
            .take_while(|_| !crate::postgres::customscan::mpp::check_for_interrupts_or_cancel())
            .map(|(segment_ord, segment_reader)| {
                let fruit = collector
                    .collect_segment(weight, segment_ord, segment_reader)
                    .inspect_err(|error| report_vector_error(self.index_rel.name(), error))
                    .unwrap_or_else(|error| report_segment_collection_error(error));
                if let Some(stats) = &self.directory.io_stats {
                    stats.end_segment(segment_reader.segment_id());
                }
                fruit
            })
            .collect()
    }

    fn collect_segment_readers_by_query_kind<'a, C: Collector>(
        &self,
        readers: impl Iterator<Item = (SegmentOrdinal, &'a SegmentReader, SegmentQueryKind)>,
        partition_bounds: Option<&SearchQueryInput>,
        collector: &C,
    ) -> Vec<<<C as Collector>::Child as SegmentCollector>::Fruit> {
        let unconstrained_weight = Arc::new(LazyWeight::new(
            self.query().box_clone(),
            self.need_scores,
            self.searcher.clone(),
        ));
        let _ = unconstrained_weight.get();
        let constrained_weight = partition_bounds.map(|bounds| {
            let range_query = Box::new(ConstScoreQuery::new(self.make_query(bounds, None), 0.0));
            unconstrained_weight.and_query(range_query)
        });
        if let Some(stats) = &self.directory.io_stats {
            stats.reset();
        }
        readers
            .take_while(|_| !crate::postgres::customscan::mpp::check_for_interrupts_or_cancel())
            .map(|(segment_ord, segment_reader, query_kind)| {
                let weight = match query_kind {
                    SegmentQueryKind::Unconstrained => unconstrained_weight.get(),
                    SegmentQueryKind::RangeConstrained => constrained_weight
                        .as_ref()
                        .expect("RangeConstrained segment requires partition_bounds")
                        .get(),
                };
                let fruit = collector
                    .collect_segment(weight, segment_ord, segment_reader)
                    .inspect_err(|error| report_vector_error(self.index_rel.name(), error))
                    .unwrap_or_else(|error| report_segment_collection_error(error));
                if let Some(stats) = &self.directory.io_stats {
                    stats.end_segment(segment_reader.segment_id());
                }
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
            SearchIndexReader::open_index_components(index_relation, mvcc_style, false, None)?;
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
            .with_disjunction_pruning(gucs::disjunction_pruning())
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
    use crate::query::pdb_query::pdb;
    use crate::query::pdb_query::to_search_query_input;
    use crate::scan::range_partitioning::RangePartitioning;
    use pgrx::prelude::*;
    use std::ops::Bound;
    use tantivy::index::SegmentComponent;

    use super::test_support::{
        assert_pruning_matches_tantivy, open_index, open_snapshot_reader, range_query, term_query,
    };

    #[pg_test]
    fn bitmap_postings_are_requested_only_for_counts() {
        use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
        use tantivy::collector::{Count, DocSetCollector};

        #[derive(Debug)]
        struct CheckBitmapOptIn {
            query: Box<dyn Query>,
            expected: bool,
            calls: Arc<AtomicUsize>,
        }

        impl Clone for CheckBitmapOptIn {
            fn clone(&self) -> Self {
                Self {
                    query: self.query.box_clone(),
                    expected: self.expected,
                    calls: Arc::clone(&self.calls),
                }
            }
        }

        impl Query for CheckBitmapOptIn {
            fn weight(&self, scoring: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
                assert_eq!(scoring.bitmap_postings_enabled(), self.expected);
                self.calls.fetch_add(1, Relaxed);
                self.query.weight(scoring)
            }
        }

        let (index, _) = segmented_index_fixture("bitmap_opt_in", 2, false);
        for expected in [false, true] {
            let mut reader = open_snapshot_reader(&index, term_query("title", "silver"), false);
            let calls = Arc::new(AtomicUsize::new(0));
            reader.query = Box::new(CheckBitmapOptIn {
                query: reader.query.box_clone(),
                expected,
                calls: Arc::clone(&calls),
            });
            if expected {
                assert_eq!(reader.collect(Count), 10);
                assert_eq!(reader.count_matched_docs().unwrap(), 10);
            } else {
                assert_eq!(reader.search().count(), 10);
                assert_eq!(reader.collect(DocSetCollector).len(), 10);
            }
            assert_eq!(calls.load(Relaxed), 2);
        }
    }

    #[pg_test]
    fn search_query_input_pruning_and_estimation_properties() {
        use crate::index::segment_pruning::EMPIRICAL_READS;
        use crate::query::TermInput;
        use proptest::prelude::*;
        use std::sync::atomic::Ordering::Relaxed;
        use tantivy::collector::Count;

        #[derive(Clone, Copy, Debug)]
        enum ScoringMode {
            Unscored,
            Scored,
        }

        #[derive(Clone, Copy, Debug)]
        enum StatsSource {
            StatsComponent,
            FastHeaderFallback,
        }

        fn is_exact_pdb_leaf(field: &FieldName, query: &pdb::Query) -> bool {
            match query {
                pdb::Query::All | pdb::Query::Empty | pdb::Query::Term { .. } => true,
                pdb::Query::Range { .. } => !matches!(field.root().as_str(), "title" | "u"),
                pdb::Query::ScoreAdjusted { query, .. } => is_exact_pdb_leaf(field, query),
                _ => false,
            }
        }

        fn is_exact_single_leaf(query: &SearchQueryInput) -> bool {
            match query {
                SearchQueryInput::All | SearchQueryInput::Empty => true,
                SearchQueryInput::FieldedQuery { field, query } => is_exact_pdb_leaf(field, query),
                SearchQueryInput::Parse { query_string, .. } => {
                    matches!(query_string.as_str(), "id:[1 TO 5]" | "id: IN [1 3 15]")
                }
                SearchQueryInput::Boost { query, .. }
                | SearchQueryInput::ConstScore { query, .. }
                | SearchQueryInput::WithIndex { query, .. } => is_exact_single_leaf(query),
                _ => false,
            }
        }

        fn arb_search_query_input(
            index_oid: pg_sys::Oid,
        ) -> impl Strategy<Value = SearchQueryInput> {
            let term_input = prop_oneof![
                prop::sample::select(vec![1i64, 10, 15, 99]).prop_map(|v| TermInput {
                    field: FieldName::from("id"),
                    value: PdbOwnedValue::I64(v),
                }),
                prop::sample::select(vec![1i64, 2, 99]).prop_map(|v| TermInput {
                    field: FieldName::from("bucket"),
                    value: PdbOwnedValue::I64(v),
                }),
                prop::sample::select(vec![i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX])
                    .prop_map(|v| TermInput {
                        field: FieldName::from("signed_extreme"),
                        value: PdbOwnedValue::U64(v),
                    }),
                Just(TermInput {
                    field: FieldName::from("title"),
                    value: PdbOwnedValue::Str("silver".into()),
                }),
                Just(TermInput {
                    field: FieldName::from("u"),
                    value: PdbOwnedValue::Str("e29b".into()),
                }),
            ];

            let int_bounds = prop::sample::select(vec![0i64, 1, 5, 10, 11, 20, 100]);
            let base_pdb_leaf = prop_oneof![
                Just(("id", pdb::Query::All)),
                Just(("title", pdb::Query::All)),
                prop::sample::select(vec!["id", "nullable_num", "title"])
                    .prop_map(|f| (f, pdb::Query::Exists)),
                prop::sample::select(vec![1i64, 10, 11, 20, 99]).prop_map(|v| (
                    "id",
                    pdb::Query::Term {
                        value: PdbOwnedValue::I64(v),
                    }
                )),
                prop::sample::select(vec![1i64, 2, 3]).prop_map(|v| (
                    "bucket",
                    pdb::Query::Term {
                        value: PdbOwnedValue::I64(v),
                    }
                )),
                prop::sample::select(vec![1i64, 2]).prop_map(|v| (
                    "nullable_num",
                    pdb::Query::Term {
                        value: PdbOwnedValue::I64(v),
                    }
                )),
                prop::sample::select(vec![
                    PdbOwnedValue::I64(i64::MIN),
                    PdbOwnedValue::I64(-2),
                    PdbOwnedValue::I64(-1),
                    PdbOwnedValue::U64(i64::MAX as u64),
                    PdbOwnedValue::U64(i64::MAX as u64 + 1),
                    PdbOwnedValue::U64(u64::MAX),
                ])
                .prop_map(|value| ("signed_extreme", pdb::Query::Term { value })),
                any::<bool>().prop_map(|b| (
                    "active",
                    pdb::Query::Term {
                        value: PdbOwnedValue::Bool(b),
                    }
                )),
                prop::sample::select(vec!["silver_1", "quiet_11", "absent"]).prop_map(|s| (
                    "lit",
                    pdb::Query::Term {
                        value: PdbOwnedValue::Str(s.into()),
                    }
                )),
                prop::sample::select(vec!["silver", "quiet", "absent"]).prop_map(|s| (
                    "title",
                    pdb::Query::Term {
                        value: PdbOwnedValue::Str(s.into()),
                    }
                )),
                prop::sample::select(vec!["e29b", "550e8400", "absent"]).prop_map(|s| (
                    "u",
                    pdb::Query::Term {
                        value: PdbOwnedValue::Str(s.into()),
                    }
                )),
                prop::collection::vec(
                    prop::sample::select(vec![
                        PdbOwnedValue::U64(i64::MAX as u64),
                        PdbOwnedValue::U64(i64::MAX as u64 + 1),
                        PdbOwnedValue::U64(u64::MAX),
                    ]),
                    1..=2,
                )
                .prop_map(|terms| ("signed_extreme", pdb::Query::TermSet { terms })),
                (
                    prop::sample::select(vec!["id", "bucket", "nullable_num"]),
                    int_bounds.clone(),
                    int_bounds,
                )
                    .prop_map(|(f, a, b)| (
                        f,
                        pdb::Query::Range {
                            lower_bound: Bound::Included(PdbOwnedValue::I64(a.min(b))),
                            upper_bound: Bound::Included(PdbOwnedValue::I64(a.max(b))),
                        }
                    )),
                Just((
                    "signed_extreme",
                    pdb::Query::Range {
                        lower_bound: Bound::Included(PdbOwnedValue::U64(u64::MAX - 1)),
                        upper_bound: Bound::Excluded(PdbOwnedValue::U64(u64::MAX)),
                    }
                )),
                prop::sample::select(vec![
                    ("2024-01-01T00:00:00Z", "2024-12-31T23:59:59Z"),
                    ("2025-01-01T00:00:00Z", "2025-12-31T23:59:59Z"),
                    ("2026-01-01T00:00:00Z", "2026-12-31T23:59:59Z"),
                ])
                .prop_map(|(lo, hi)| (
                    "created_at",
                    pdb::Query::Range {
                        lower_bound: Bound::Included(PdbOwnedValue::Str(lo.into())),
                        upper_bound: Bound::Included(PdbOwnedValue::Str(hi.into())),
                    }
                )),
                prop::sample::select(vec![("title", "silver"), ("u", "e29b")]).prop_map(
                    |(f, tok)| (
                        f,
                        pdb::Query::Range {
                            lower_bound: Bound::Included(PdbOwnedValue::Str(tok.into())),
                            upper_bound: Bound::Included(PdbOwnedValue::Str(tok.into())),
                        }
                    )
                ),
                (
                    prop::option::of(Just(1u8)),
                    any::<bool>(),
                    prop::option::of(Just(pdb::SynonymRules::Rules(vec![
                        "dragon => quiet".into(),
                    ]))),
                )
                    .prop_map(|(distance, conj, synonyms)| (
                        "title",
                        pdb::Query::Match {
                            value: "SILVER dragon".into(),
                            tokenizer: None,
                            distance,
                            transposition_cost_one: None,
                            prefix: None,
                            conjunction_mode: Some(conj),
                            synonyms,
                        }
                    )),
                prop::sample::select(vec!["silver dragon", "silver quiet"]).prop_map(|p| (
                    "title",
                    pdb::Query::TokenizedPhrase {
                        phrase: p.into(),
                        slop: None,
                    }
                )),
                prop::sample::select(vec!["sil.*", "zzz.*"]).prop_map(|pat| (
                    "title",
                    pdb::Query::Regex {
                        pattern: pat.into(),
                    }
                )),
            ];

            let leaf = prop_oneof![
                2 => Just(SearchQueryInput::All),
                1 => Just(SearchQueryInput::Empty),
                1 => Just(SearchQueryInput::from_unfielded(pdb::Query::All)),
                3 => prop::collection::vec(term_input, 0..=16)
                    .prop_map(|terms| SearchQueryInput::TermSet { terms }),
                2 => prop::sample::select(vec![
                    "id:[1 TO 5]",
                    "id: IN [1 3 15]",
                    "title:silver",
                    "id:[1 TO 5] AND title:silver",
                ])
                .prop_map(|s| SearchQueryInput::Parse {
                    query_string: s.into(),
                    lenient: None,
                    conjunction_mode: None,
                }),
                8 => (
                    base_pdb_leaf,
                    prop_oneof![
                        3 => Just(None),
                        1 => Just(Some((pdb::ScoreAdjustStyle::Boost(2.0), false))),
                        1 => Just(Some((pdb::ScoreAdjustStyle::Const(0.0), true))),
                    ],
                )
                    .prop_map(|((field, q), adj)| {
                        let field = FieldName::from(field);
                        match adj {
                            None => SearchQueryInput::FieldedQuery { field, query: q },
                            Some((style, hoist)) => {
                                let adjusted = pdb::Query::ScoreAdjusted {
                                    query: Box::new(q),
                                    score: Some(style),
                                };
                                if hoist {
                                    to_search_query_input(field, adjusted)
                                } else {
                                    SearchQueryInput::FieldedQuery {
                                        field,
                                        query: adjusted,
                                    }
                                }
                            }
                        }
                    }),
            ];

            leaf.prop_recursive(2, 10, 3, move |inner| {
                prop_oneof![
                    4 => (
                        prop::collection::vec(inner.clone(), 0..=2),
                        prop::collection::vec(inner.clone(), 0..=2),
                        prop::collection::vec(inner.clone(), 0..=2),
                        prop::sample::select(vec![None, Some(0), Some(1), Some(2), Some(-1)]),
                    )
                        .prop_map(|(must, should, must_not, minimum_should_match)| {
                            SearchQueryInput::Boolean {
                                must,
                                should,
                                must_not,
                                minimum_should_match,
                            }
                        }),
                    1 => inner.clone().prop_map(|q| SearchQueryInput::Boost {
                        query: Box::new(q),
                        factor: 2.0,
                    }),
                    1 => inner.clone().prop_map(|q| SearchQueryInput::ConstScore {
                        query: Box::new(q),
                        score: 0.0,
                    }),
                    1 => inner.clone().prop_map(move |q| SearchQueryInput::WithIndex {
                        oid: index_oid,
                        query: Box::new(q),
                    }),
                    1 => prop::collection::vec(inner, 1..=2).prop_map(|disjuncts| {
                        SearchQueryInput::DisjunctionMax {
                            disjuncts,
                            tie_breaker: None,
                        }
                    }),
                ]
            })
        }

        Spi::run(
            "CREATE TABLE query_properties (
                 id bigint PRIMARY KEY,
                 signed_extreme bigint NOT NULL,
                 bucket bigint NOT NULL,
                 active boolean NOT NULL,
                 created_at timestamp NOT NULL,
                 nullable_num bigint,
                 title text NOT NULL,
                 lit text NOT NULL,
                 u uuid NOT NULL
             );
             CREATE INDEX query_properties_idx ON query_properties
             USING paradedb (
                 id,
                 signed_extreme,
                 bucket,
                 active,
                 created_at,
                 nullable_num,
                 (title::pdb.unicode_words('columnar=true')),
                 (lit::pdb.literal),
                 u
             )
             WITH (
                 partition_by = 'id',
                 target_segment_count = 8,
                 background_layer_sizes = '0',
                 text_fields = '{\"u\": {\"tokenizer\": {\"type\": \"default\"}, \"fast\": true}}'
             );
             SET paradedb.global_mutable_segment_rows = 0;
             INSERT INTO query_properties
             SELECT g,
                    CASE WHEN g = 1 THEN '-9223372036854775808'::bigint ELSE -1 END,
                    1,
                    false,
                    TIMESTAMP '2024-01-01 00:00:00' + (g || ' days')::interval,
                    CASE WHEN g = 1 THEN NULL ELSE 1 END,
                    'silver dragon ' || g,
                    'silver_' || g,
                    '550e8400-e29b-41d4-a716-446655440000'::uuid
             FROM generate_series(1, 10) g;
             INSERT INTO query_properties
             SELECT g,
                    CASE WHEN g = 20 THEN '9223372036854775807'::bigint ELSE -2 END,
                    2,
                    true,
                    TIMESTAMP '2025-01-01 00:00:00' + (g || ' days')::interval,
                    1,
                    'quiet river ' || g,
                    'quiet_' || g,
                    '650e8400-e29b-41d4-a716-446655440000'::uuid
             FROM generate_series(11, 20) g;
             RESET paradedb.global_mutable_segment_rows;",
        )
        .unwrap();

        let index_rel = open_index("query_properties_idx");
        let probe = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        let segment_ids = probe.segment_ids();
        assert_eq!(segment_ids.len(), 2);
        for name in [
            "id",
            "signed_extreme",
            "bucket",
            "active",
            "created_at",
            "nullable_num",
            "title",
            "lit",
            "u",
        ] {
            let field = probe.schema().search_field(name).unwrap();
            for ord in 0..2 {
                assert!(
                    probe
                        .segment_stats_snapshot()
                        .empirical(ord, &field)
                        .is_some(),
                    "fixture must populate .stats for {name}"
                );
            }
        }

        let config = ProptestConfig::with_cases(64);
        proptest!(config, |(
            query in arb_search_query_input(index_rel.oid()),
            scoring_mode in prop_oneof![Just(ScoringMode::Unscored), Just(ScoringMode::Scored)],
            stats_source in prop_oneof![
                Just(StatsSource::StatsComponent),
                Just(StatsSource::FastHeaderFallback),
            ],
        )| {
            let guards: Vec<_> = match stats_source {
                StatsSource::StatsComponent => Vec::new(),
                StatsSource::FastHeaderFallback => segment_ids
                    .iter()
                    .map(|&seg| inject_stats_failure(seg, InjectedStatsFailure::Missing))
                    .collect(),
            };
            let need_scores = matches!(scoring_mode, ScoringMode::Scored);

            STATS_OPENS.store(0, Relaxed);
            EMPIRICAL_READS.store(0, Relaxed);
            let reader = open_snapshot_reader(&index_rel, query.clone(), need_scores);
            prop_assert_eq!(STATS_OPENS.load(Relaxed), 0);
            prop_assert_eq!(EMPIRICAL_READS.load(Relaxed), 0);

            let unpruned_count = reader.searcher().search(reader.query(), &Count).unwrap();
            let pruning = reader.segment_pruning_estimate();
            prop_assert_eq!(pruning.candidate_segments + pruning.pruned_segments, 2);
            assert_pruning_matches_tantivy(&reader, unpruned_count);
            let top = reader.search_top_k_unordered_in_segments(
                reader.segment_ids().into_iter(),
                30,
                0,
            );
            prop_assert_eq!(top.results.len(), unpruned_count);

            let stats_opens = STATS_OPENS.load(Relaxed);
            let empirical_reads = EMPIRICAL_READS.load(Relaxed);
            prop_assert!(stats_opens <= 2);
            if !SegmentPruner::has_prunable_predicate(reader.schema(), &query) {
                prop_assert_eq!(pruning.candidate_segments, 2);
                prop_assert_eq!(stats_opens, 0);
                prop_assert_eq!(empirical_reads, 0);
            }
            if let SearchQueryInput::TermSet { terms } = &query {
                let distinct_fields: crate::api::HashSet<_> =
                    terms.iter().map(|t| &t.field).collect();
                prop_assert!(empirical_reads <= distinct_fields.len() * 2);
            }
            if matches!(stats_source, StatsSource::FastHeaderFallback) {
                let hidden_hits: usize = guards.iter().map(|g| g.hits()).sum();
                prop_assert_eq!(hidden_hits, stats_opens);
            }

            let uses_stats = query.supports_statistics_estimation();
            prop_assert_eq!(reader.supports_statistics_estimation(), uses_stats);
            let est = reader.estimate_docs(RowEstimate::Unknown);
            prop_assert_eq!(est.total_docs, 20);
            let scaled = reader.estimate_docs(RowEstimate::Known(160));
            prop_assert_eq!(scaled.matching_docs, est.matching_docs * 8);
            let vec_frac = reader.estimate_vector_match_fraction();
            prop_assert!((0.0..=1.0).contains(&vec_frac));
            if pruning.candidate_segments == 0 {
                prop_assert_eq!(est.matching_docs, 0);
                prop_assert_eq!(est.query_cost, 0);
                prop_assert_eq!(vec_frac, 0.0);
            }
            if uses_stats {
                prop_assert!(est.matching_docs <= 20);
                if is_exact_single_leaf(&query) {
                    prop_assert_eq!(est.matching_docs, unpruned_count);
                    if pruning.candidate_segments > 0 {
                        let candidate_docs: u64 = reader
                            .candidate_segment_readers()
                            .map(|(_, r)| u64::from(r.num_docs()))
                            .sum();
                        prop_assert!(
                            (vec_frac - (unpruned_count as f64 / candidate_docs as f64)).abs()
                                <= 1e-9
                        );
                    }
                }
            }
            if !need_scores || uses_stats {
                let mut tree = reader
                    .build_query_tree_with_estimates(query.clone())
                    .unwrap();
                prop_assert_eq!(tree.estimated_docs, Some(est.matching_docs));
                let mut all_nodes_valid = true;
                tree.traverse_mut(0, &mut |node, _| {
                    all_nodes_valid &= node
                        .estimated_docs
                        .is_some_and(|n| !uses_stats || n <= 20);
                });
                prop_assert!(all_nodes_valid);
            }
        });

        for scoring in [false, true] {
            let single_must_with_min_should = open_snapshot_reader(
                &index_rel,
                SearchQueryInput::Boolean {
                    must: vec![SearchQueryInput::ConstScore {
                        query: Box::new(SearchQueryInput::All),
                        score: 1.0,
                    }],
                    should: vec![],
                    must_not: vec![],
                    minimum_should_match: Some(1),
                },
                scoring,
            );
            assert_eq!(
                single_must_with_min_should
                    .segment_pruning_estimate()
                    .candidate_segments,
                0
            );
            assert_pruning_matches_tantivy(&single_must_with_min_should, 0);
        }

        let regex_reader = open_snapshot_reader(
            &index_rel,
            SearchQueryInput::FieldedQuery {
                field: "title".into(),
                query: pdb::Query::Regex {
                    pattern: "sil.*".into(),
                },
            },
            false,
        );
        assert_eq!(
            regex_reader.estimate_vector_match_fraction(),
            DEFAULT_VECTOR_MATCH_FRACTION
        );

        Spi::run("SET paradedb.enable_segment_pruning = off;").unwrap();
        STATS_OPENS.store(0, Relaxed);
        let disabled = open_snapshot_reader(&index_rel, range_query("id", 100, 200), false);
        assert_eq!(disabled.segment_pruning_estimate().candidate_segments, 2);
        assert_eq!(STATS_OPENS.load(Relaxed), 0);
        assert_pruning_matches_tantivy(&disabled, 0);
        Spi::run("RESET paradedb.enable_segment_pruning;").unwrap();

        for (name, mutable, expected) in [
            ("statistics_empty", false, 0),
            ("statistics_mutable", true, 5),
        ] {
            let (index, _) = segmented_index_fixture(name, 0, mutable);
            unsafe { pg_sys::PushActiveSnapshot(pg_sys::GetTransactionSnapshot()) };
            let reader = SearchIndexReader::open(
                &index,
                term_query("title", "mutable"),
                false,
                MvccSatisfies::LargestSegment,
            )
            .unwrap();
            let estimate = reader.estimate_docs(RowEstimate::Unknown);
            assert_eq!(estimate.matching_docs, expected);
            assert_eq!(estimate.total_docs, expected as u64);
            unsafe { pg_sys::PopActiveSnapshot() };
        }
    }

    #[pg_test]
    fn collect_ctidset_rebinds_visibility_to_each_reader() {
        let (index_rel, heap_oid) = segmented_index_fixture("ctidset_reader_reuse", 2, false);
        let heap_rel = PgSearchRelation::open(heap_oid);
        unsafe { pg_sys::PushActiveSnapshot(pg_sys::GetTransactionSnapshot()) };
        let mut visibility =
            VisibilityChecker::with_rel_and_snap(&heap_rel, unsafe { pg_sys::GetActiveSnapshot() });
        let largest = SearchIndexReader::open(
            &index_rel,
            SearchQueryInput::All,
            false,
            MvccSatisfies::LargestSegment,
        )
        .unwrap();
        let KeySet::InMemory(largest_ctids) = largest.collect_ctidset(&mut visibility) else {
            panic!("expected an in-memory CTID set");
        };
        assert_eq!(largest_ctids.len(), 10);

        let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        let KeySet::InMemory(all_ctids) = reader.collect_ctidset(&mut visibility) else {
            panic!("expected an in-memory CTID set");
        };
        assert_eq!(all_ctids.len(), 20);
        assert!(largest_ctids.is_subset(&all_ctids));
        unsafe { pg_sys::PopActiveSnapshot() };
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

    /// One partition's scan omits the range filter on fully included segments and keeps it on
    /// partially included ones. The exact query over every segment is the oracle for rows (and
    /// for scores, on `All`); the two-reader form is the oracle for scores on every query, and
    /// the base weight is prepared once, lazily, for both variants.
    #[pg_test]
    fn partition_filter_is_omitted_only_when_redundant() {
        use crate::index::stats::segments_for_partition;
        use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

        #[derive(Debug)]
        struct CountPreparation {
            query: Box<dyn Query>,
            calls: Arc<AtomicUsize>,
        }

        impl Clone for CountPreparation {
            fn clone(&self) -> Self {
                Self {
                    query: self.query.box_clone(),
                    calls: Arc::clone(&self.calls),
                }
            }
        }

        impl Query for CountPreparation {
            fn weight(&self, scoring: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
                self.calls.fetch_add(1, Relaxed);
                self.query.weight(scoring)
            }
        }

        fn scored_hits(
            results: impl Iterator<Item = (SearchIndexScore, DocAddress)>,
        ) -> BTreeMap<(SegmentOrdinal, DocId), f32> {
            results
                .map(|(score, doc)| ((doc.segment_ord, doc.doc_id), score.bm25))
                .collect()
        }
        let check = |index_rel: &PgSearchRelation,
                     query: &SearchQueryInput,
                     scoring: bool,
                     partitioning: &RangePartitioning,
                     partition: usize,
                     expected_all: usize,
                     expected_included: usize,
                     expected_partial: usize| {
            let mut reader = open_snapshot_reader(index_rel, query.clone(), scoring);
            let bounds = partitioning.partition_bounds(partition);
            let segments = segments_for_partition(&reader, partitioning, partition);
            assert_eq!(
                (segments.included.len(), segments.partially_included.len()),
                (expected_included, expected_partial),
                "{query:?}, {bounds:?}"
            );

            let make_bounds_query =
                || Box::new(ConstScoreQuery::new(reader.make_query(&bounds, None), 0.0));
            let exact = scored_hits(reader.and_query(make_bounds_query()).search());
            let is_all = matches!(query, SearchQueryInput::All);
            if is_all {
                assert_eq!(exact.len(), expected_all, "{bounds:?}");
            }
            // Partition bounds are wrapped in ConstScoreQuery(..., 0.0) so they do not
            // alter scores. Applying the range across all segments or per group matches
            // the per-partition search.
            let per_group = scored_hits(
                reader
                    .search_segments(segments.included.iter().copied())
                    .chain(
                        reader
                            .and_query(make_bounds_query())
                            .search_segments(segments.partially_included.iter().copied()),
                    ),
            );

            let calls = Arc::new(AtomicUsize::new(0));
            reader.query = Box::new(CountPreparation {
                query: reader.query.box_clone(),
                calls: Arc::clone(&calls),
            });
            let scan = reader.search_segments_with_range_filter(&segments, &bounds);
            assert_eq!(calls.load(Relaxed), 0, "preparation must remain lazy");
            let actual = scored_hits(scan);
            assert_eq!(actual.len(), exact.len(), "{query:?}, {bounds:?}");
            assert_eq!(actual, exact, "{query:?}, scoring={scoring}, {bounds:?}");
            assert_eq!(
                actual, per_group,
                "{query:?}, scoring={scoring}, {bounds:?}"
            );
            assert_eq!(
                calls.load(Relaxed),
                usize::from(expected_included > 0) + usize::from(expected_partial > 0)
            );
        };

        // Fixture ids 1..=20 over two NOT NULL segments holding 1..=10 and 11..=20.
        let (index_rel, _heap) = segmented_index_fixture("contained_range_filter_test", 2, false);
        let text = SearchQueryInput::Boolean {
            must: vec![],
            should: vec![term_query("title", "silver"), term_query("title", "quiet")],
            must_not: vec![],
            minimum_should_match: Some(1),
        };
        for query in [
            SearchQueryInput::All,
            term_query("title", "silver"),
            term_query("title", "absent"),
            text.clone(),
            SearchQueryInput::Boost {
                query: Box::new(text.clone()),
                factor: 2.5,
            },
            SearchQueryInput::ConstScore {
                query: Box::new(text),
                score: 3.0,
            },
        ] {
            for scoring in [false, true] {
                for (split, partition, expected_all, included, partial) in [
                    (11, 0, 10, 1, 0),
                    (11, 1, 10, 1, 0),
                    (5, 0, 4, 0, 1),
                    (15, 0, 14, 1, 1),
                    (15, 1, 6, 0, 1),
                    (100, 1, 0, 0, 0),
                ] {
                    let partitioning = RangePartitioning {
                        partition_by: FieldName::from("id"),
                        split_points: vec![PdbOwnedValue::I64(split)],
                    };
                    check(
                        &index_rel,
                        &query,
                        scoring,
                        &partitioning,
                        partition,
                        expected_all,
                        included,
                        partial,
                    );
                }
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
        let index_rel = open_index("nullable_partition_filter_idx");
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
        // Partition 0 owns the NULL rows, so a nullable segment inside its value range is fully
        // included and searches without the partition filter: all three rows belong to it.
        // With the edge inside the segment, partition 0 keeps the NULL row and drops the value
        // above the edge. Partition 1 does not own NULLs, so the same segment stays partially
        // included and the filter excludes the NULL row.
        for (split, partition, expected_all, included, partial) in
            [(21, 0, 3, 1, 0), (15, 0, 2, 0, 1), (5, 1, 2, 0, 1)]
        {
            let partitioning = RangePartitioning {
                partition_by: FieldName::from("value"),
                split_points: vec![PdbOwnedValue::I64(split)],
            };
            for scoring in [false, true] {
                check(
                    &index_rel,
                    &SearchQueryInput::All,
                    scoring,
                    &partitioning,
                    partition,
                    expected_all,
                    included,
                    partial,
                );
            }
        }
    }

    #[pg_test]
    fn statistics_are_read_only_when_a_decision_needs_them() {
        use crate::index::segment_pruning::EMPIRICAL_READS;
        use std::sync::atomic::Ordering::Relaxed;

        let (index_rel, _heap) = segmented_index_fixture("direct_segment_checks", 4, true);

        STATS_OPENS.store(0, Relaxed);
        let unprovable = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        assert_eq!(unprovable.search().count(), 45);
        assert_eq!(
            STATS_OPENS.load(Relaxed),
            0,
            "a query without a provable predicate must not open any .stats component"
        );
        // Planning estimates prune candidate segments using .stats before estimating.
        let estimating = SearchIndexReader::open(
            &index_rel,
            range_query("id", 100, 200),
            false,
            MvccSatisfies::LargestSegment,
        )
        .unwrap();
        assert_eq!(
            STATS_OPENS.load(Relaxed),
            0,
            "opening a reader must not open statistics"
        );
        let estimate = estimating.estimate_docs(RowEstimate::Known(40));
        assert_eq!(estimate.matching_docs, 0);
        assert_eq!(estimate.query_cost, 0);
        assert_eq!(
            STATS_OPENS.load(Relaxed),
            1,
            "estimating docs prunes using the segment's statistics"
        );
        assert_eq!(estimating.segment_pruning_estimate().candidate_segments, 0);
        assert_eq!(
            STATS_OPENS.load(Relaxed),
            1,
            "subsequent pruning checks reuse the opened statistics"
        );

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
        assert_eq!(
            EMPIRICAL_READS.load(Relaxed),
            7,
            "subsequent searches and derived readers reuse cached per-segment candidacy"
        );
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
        assert_eq!(
            STATS_OPENS.load(Relaxed),
            5,
            "derived readers and manifest siblings reuse opened components, including absence"
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
    fn vector_parallelism_uses_work_only_for_filters() {
        Spi::run(
            "SET max_parallel_workers_per_gather = 2;
             SET parallel_setup_cost = 0;
             SET parallel_tuple_cost = 0;
             SET paradedb.min_rows_per_worker = 0;
             CREATE TABLE vector_parallelism (id bigint PRIMARY KEY, category text, embedding vector(3), heap_value bigint DEFAULT 1);
             CREATE INDEX vector_parallelism_idx ON vector_parallelism
             USING paradedb (id, (category::pdb.literal), embedding vector_l2_ops)
             WITH (background_layer_sizes = '0', layer_sizes = '0');
             SET paradedb.global_mutable_segment_rows = 0;
             INSERT INTO vector_parallelism (id, category, embedding) SELECT g, CASE WHEN g % 2 = 0 THEN 'keep' ELSE 'other' END, ARRAY[g::real,1,0]::vector FROM generate_series(1,100) g;
             INSERT INTO vector_parallelism (id, category, embedding) SELECT g, CASE WHEN g % 2 = 0 THEN 'keep' ELSE 'other' END, ARRAY[g::real,1,0]::vector FROM generate_series(101,200) g;
             SET vector_parallelism.filter = 'keep OR other';
             SET vector_parallelism.range = '[2 TO 100]';
             RESET paradedb.global_mutable_segment_rows;"
        ).unwrap();
        for (predicate, workers, parallel) in [
            ("", 2, false),
            ("AND category = 'keep'", 2, false),
            (
                "AND id @@@ paradedb.parse_with_field('category', current_setting('vector_parallelism.filter'))",
                2,
                false,
            ),
            ("AND id > 1", 2, true),
            (
                "AND id @@@ paradedb.parse_with_field('id', current_setting('vector_parallelism.range'))",
                2,
                true,
            ),
            ("AND heap_value = 1", 2, true),
            ("AND category = 'keep' AND heap_value = 1", 2, true),
            ("AND category = 'keep'", 0, false),
            ("AND id > 1", 0, false),
            ("AND heap_value = 1", 0, false),
        ] {
            Spi::run(&format!("SET max_parallel_workers_per_gather = {workers}")).unwrap();
            let plan = Spi::get_one::<pgrx::Json>(&format!(
                "EXPLAIN (VERBOSE, COSTS OFF, FORMAT JSON) SELECT id FROM vector_parallelism WHERE id @@@ pdb.all() {predicate} ORDER BY embedding <-> '[1,1,0]'::vector LIMIT 5"
            )).unwrap().unwrap().0;
            let mut nodes = vec![&plan[0]["Plan"]];
            let mut found = false;
            let mut workers = false;
            while let Some(node) = nodes.pop() {
                workers |= node["Workers Planned"].as_u64().unwrap_or(0) > 0;
                if node["Index"] == "vector_parallelism_idx" {
                    found = true;
                    assert_eq!(node["Parallel Aware"], parallel, "{plan}");
                    assert_eq!(node["Worker Selection"], "Vector work", "{plan}");
                    if predicate.contains("heap_value") {
                        assert!(
                            node["Tantivy Query"].to_string().contains("heap_filter"),
                            "{plan}"
                        );
                    }
                    if predicate.contains("category") {
                        assert!(
                            node["Tantivy Query"].to_string().contains("category"),
                            "{plan}"
                        );
                        assert!(node.get("Filter").is_none(), "{plan}");
                    }
                }
                if let Some(children) = node["Plans"].as_array() {
                    nodes.extend(children);
                }
            }
            assert!(found, "{plan}");
            assert_eq!(workers, parallel, "{plan}");
        }
        Spi::run(
            "ANALYZE vector_parallelism;
             UPDATE pg_class SET reltuples = 10000000 WHERE oid = 'vector_parallelism'::regclass;
             SET max_parallel_workers_per_gather = 2;
             SET paradedb.vector_cluster_max_probe = 65536;",
        )
        .unwrap();
        for (predicate, parallel) in [("", false), ("AND category = 'keep'", true)] {
            let plan = Spi::get_one::<pgrx::Json>(&format!(
                "EXPLAIN (VERBOSE, COSTS OFF, FORMAT JSON) SELECT id FROM vector_parallelism WHERE id @@@ pdb.all() {predicate} ORDER BY embedding <-> '[1,1,0]'::vector LIMIT 5"
            )).unwrap().unwrap().0;
            let mut nodes = vec![&plan[0]["Plan"]];
            let mut found = false;
            while let Some(node) = nodes.pop() {
                if node["Index"] == "vector_parallelism_idx" {
                    found = true;
                    assert_eq!(node["Parallel Aware"], parallel, "{plan}");
                    assert_eq!(node["Worker Selection"], "Vector work", "{plan}");
                }
                if let Some(children) = node["Plans"].as_array() {
                    nodes.extend(children);
                }
            }
            assert!(found, "{plan}");
        }
    }

    #[pg_test]
    fn non_vector_topk_keeps_segment_collection_lazy() {
        let (index_rel, _) = segmented_index_fixture("lazy_sorted_segments", 3, false);
        let query = SearchQueryInput::Boolean {
            must: vec![],
            should: vec![term_query("title", "silver"), term_query("title", "quiet")],
            must_not: vec![],
            minimum_should_match: None,
        };
        let field_order = OrderByInfo {
            feature: OrderByFeature::Field {
                name: FieldName::from("id"),
                rti: 0,
            },
            direction: SortDirection::AscNullsLast,
        };
        let score_order = OrderByInfo {
            feature: OrderByFeature::Score { rti: 0 },
            direction: SortDirection::DescNullsLast,
        };
        for order in [
            vec![field_order.clone()],
            vec![score_order.clone()],
            vec![score_order, field_order],
        ] {
            let trace = io_stats::Trace::default();
            let reader = SearchIndexReader::open_with_context(
                &index_rel,
                query.clone(),
                true,
                MvccSatisfies::Snapshot,
                None,
                None,
                false,
                Some(trace.clone()),
            )
            .unwrap();
            let ids = reader.segment_ids();
            assert_eq!(ids.len(), 3);
            let segments = ids.iter().copied().enumerate().map(|(i, id)| {
                if i > 0 {
                    let mut info = BTreeMap::from([(ids[i - 1], serde_json::json!({}))]);
                    trace.attach(&mut info);
                    assert!(
                        info[&ids[i - 1]]["blocks_fetched"].as_u64().unwrap_or(0) > 0,
                        "collection must finish before claiming the next segment: {order:?}"
                    );
                }
                id
            });
            let top = reader.search_top_k_in_segments(
                segments,
                &order,
                50,
                0,
                None,
                None,
                &VectorRouting::Local,
            );
            assert_eq!(top.results.len(), 30);
        }
    }

    #[pg_test(error = "canceling statement due to user request")]
    fn global_vector_topk_checks_for_interrupts() {
        Spi::run(
            "CREATE EXTENSION IF NOT EXISTS vector;
             CREATE TABLE cancel_vectors (id bigint PRIMARY KEY, embedding vector(3));
             CREATE INDEX cancel_vectors_idx ON cancel_vectors
             USING paradedb (id, embedding vector_l2_ops)
             WITH (background_layer_sizes = '0');
             SET paradedb.global_mutable_segment_rows = 0;
             INSERT INTO cancel_vectors SELECT g, ARRAY[g::real,1,0]::vector FROM generate_series(1,10) g;
             INSERT INTO cancel_vectors SELECT g, ARRAY[g::real,1,0]::vector FROM generate_series(11,20) g;
             RESET paradedb.global_mutable_segment_rows;",
        )
        .unwrap();
        let index_rel = open_index("cancel_vectors_idx");
        let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        let ids = reader.segment_ids();
        assert_eq!(ids.len(), 2);
        let order = [OrderByInfo {
            feature: OrderByFeature::VectorDistance {
                name: FieldName::from("embedding"),
                rti: 0,
                query_vector: crate::api::QueryVector::Resolved(vec![1.0, 0.0, 0.0]),
                metric: crate::vector::metric::VectorMetric::L2,
            },
            direction: SortDirection::AscNullsLast,
        }];
        let segments = ids.into_iter().inspect(|_| unsafe {
            pg_sys::QueryCancelPending = 1;
            pg_sys::InterruptPending = 1;
        });
        reader.search_top_k_in_segments(segments, &order, 10, 0, None, None, &VectorRouting::Local);
        panic!("global vector collection ignored cancellation");
    }

    #[pg_test]
    fn vector_topk_with_aggregation_and_visibility() {
        use crate::api::QueryVector;
        use crate::vector::metric::VectorMetric;
        use tantivy::aggregation::{AggContextParams, AggregationLimitsGuard};
        Spi::run(
            "CREATE TABLE vector_window (id bigint PRIMARY KEY, embedding vector(3));
             INSERT INTO vector_window SELECT g, ARRAY[g::real,1,0]::vector FROM generate_series(1,20) g;
             CREATE INDEX vector_window_idx ON vector_window USING paradedb (id, embedding vector_l2_ops)
             WITH (background_layer_sizes = '0');
             DELETE FROM vector_window WHERE id = 1;"
        ).unwrap();
        unsafe { pg_sys::PushActiveSnapshot(pg_sys::GetTransactionSnapshot()) };
        let index_rel = open_index("vector_window_idx");
        let heap = PgSearchRelation::open(
            Spi::get_one::<pg_sys::Oid>("SELECT 'vector_window'::regclass::oid")
                .unwrap()
                .unwrap(),
        );
        let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        let order = vec![OrderByInfo {
            feature: OrderByFeature::VectorDistance {
                name: FieldName::from("embedding"),
                rti: 0,
                query_vector: QueryVector::Resolved(vec![0.0, 1.0, 0.0]),
                metric: VectorMetric::L2,
            },
            direction: SortDirection::AscNullsLast,
        }];
        for visible in [false, true] {
            let request: tantivy::aggregation::agg_req::Aggregations =
                serde_json::from_value(serde_json::json!({"total":{"value_count":{"field":"id"}}}))
                    .unwrap();
            let aggregation_collector = DistributedAggregationCollector::from_aggs(
                request.clone(),
                AggContextParams::new(
                    Default::default(),
                    reader.searcher().index().tokenizers().clone(),
                ),
            );
            let vischeck = visible.then(|| {
                let mut check = VisibilityChecker::with_rel_and_snap(&heap, unsafe {
                    pg_sys::GetActiveSnapshot()
                });
                check.set_ffhelper(Arc::new(FFHelper::for_ctid(&reader)));
                check
            });
            let top = reader.search_top_k_in_segments(
                reader.segment_ids().into_iter(),
                &order,
                3,
                0,
                Some(TopKAuxiliaryCollector {
                    aggregation_collector,
                    vischeck,
                }),
                None,
                &VectorRouting::Local,
            );
            let ids = top
                .results
                .iter()
                .map(|(_, doc)| {
                    reader
                        .searcher()
                        .segment_reader(doc.segment_ord)
                        .fast_fields()
                        .i64("id")
                        .unwrap()
                        .first(doc.doc_id)
                        .unwrap()
                })
                .collect::<Vec<_>>();
            assert_eq!(
                ids,
                if visible {
                    vec![2, 3, 4]
                } else {
                    vec![1, 2, 3]
                }
            );
            let results = top
                .aggregation_results
                .unwrap()
                .into_final_result(request, AggregationLimitsGuard::default())
                .unwrap();
            let results = serde_json::to_value(results).unwrap();
            assert_eq!(
                results["total"]["value"].as_f64().unwrap(),
                if visible { 19.0 } else { 20.0 }
            );
        }
        unsafe { pg_sys::PopActiveSnapshot() };
    }

    #[pg_test]
    fn vector_topk_reports_only_collected_candidates() {
        use crate::api::QueryVector;
        use crate::vector::metric::VectorMetric;
        Spi::run(
            "CREATE TABLE candidate_vectors (id bigint PRIMARY KEY, embedding vector(3));
             CREATE INDEX candidate_vectors_idx ON candidate_vectors
             USING paradedb (id, embedding vector_l2_ops)
             WITH (partition_by = 'id', target_segment_count = 8, background_layer_sizes = '0');
             SET paradedb.global_mutable_segment_rows = 0;
             INSERT INTO candidate_vectors SELECT g, ARRAY[g::real,1,0]::vector FROM generate_series(1,10) g;
             INSERT INTO candidate_vectors SELECT g, ARRAY[g::real,1,0]::vector FROM generate_series(11,20) g;
             RESET paradedb.global_mutable_segment_rows;",
        ).unwrap();
        let index_rel = open_index("candidate_vectors_idx");
        let base = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        assert_eq!(base.segment_ids().len(), 2);
        for (lo, hi, expected_rows, expected_segments) in [
            (1, 10, 10, 1),
            (11, 20, 10, 1),
            (1, 20, 20, 2),
            (99, 100, 0, 0),
        ] {
            for reverse in [false, true] {
                for (tie_break, individual_segments) in
                    [(false, false), (true, false), (false, true), (true, true)]
                {
                    let trace = io_stats::Trace::default();
                    let reader = SearchIndexReader::open_with_context(
                        &index_rel,
                        range_query("id", lo, hi),
                        false,
                        MvccSatisfies::Snapshot,
                        None,
                        None,
                        false,
                        Some(trace.clone()),
                    )
                    .unwrap();
                    let mut ids = reader.segment_ids();
                    if reverse {
                        ids.reverse();
                    }
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
                    let routing = if individual_segments {
                        let results = reader.vector_routing_plan(
                            &FieldName::from("embedding"),
                            &[1.0, 0.0, 0.0],
                            reader.estimate_vector_match_fraction(),
                            reader.vector_search_budget(&FieldName::from("embedding")),
                        );
                        let bytes = serde_cbor::to_vec(&results).unwrap();
                        let results =
                            serde_cbor::from_slice::<VectorRoutingResults>(&bytes).unwrap();
                        let shared = if reverse {
                            SharedVectorRouting {
                                budget: results.budget,
                                results: OnceLock::new(),
                                work: OnceLock::new(),
                                shared: None,
                            }
                        } else {
                            SharedVectorRouting::from(results)
                        };
                        VectorRouting::Shared(Arc::new(shared), Default::default())
                    } else {
                        VectorRouting::Local
                    };
                    let checked_out = std::cell::Cell::new(0);
                    let first_id = ids[0];
                    let ids = ids.into_iter().inspect(|_| {
                        if individual_segments && expected_segments == 2 && checked_out.get() == 1 {
                            let mut info = BTreeMap::from([(first_id, serde_json::json!({}))]);
                            trace.attach(&mut info);
                            assert!(
                                info[&first_id]["blocks_fetched"].as_u64().unwrap_or(0) > 0,
                                "collection must finish before claiming another segment"
                            );
                        }
                        checked_out.set(checked_out.get() + 1);
                    });
                    let top =
                        reader.search_top_k_in_segments(ids, &order, 50, 0, None, None, &routing);
                    let docs = top
                        .results
                        .into_iter()
                        .map(|(_, doc)| doc)
                        .collect::<Vec<_>>();
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

    #[pg_test]
    fn vector_topk_retry_replays_a_spent_shared_budget() {
        use crate::api::QueryVector;
        use crate::vector::metric::VectorMetric;
        use tantivy::vector::SharedVectorWork;
        Spi::run(
            "SET paradedb.vector_clustering_threshold = 1;
             SET paradedb.global_mutable_segment_rows = 0;
             SET paradedb.vector_cluster_max_probe = 1;
             CREATE TABLE retry_vectors (id bigint PRIMARY KEY, embedding vector(3));
             CREATE INDEX retry_vectors_idx ON retry_vectors
             USING paradedb (id, embedding vector_l2_ops)
             WITH (partition_by = 'id', target_segment_count = 1, background_layer_sizes = '0');
             INSERT INTO retry_vectors SELECT g, ARRAY[g::real,1,0]::vector FROM generate_series(1,10) g;
             INSERT INTO retry_vectors SELECT g, ARRAY[g::real,1,0]::vector FROM generate_series(11,20) g;
             REINDEX INDEX retry_vectors_idx;",
        ).unwrap();
        let index_rel = open_index("retry_vectors_idx");
        let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        let field = FieldName::from("embedding");
        let query = vec![1.0, 0.0, 0.0];
        let plan = Arc::new(SharedVectorRouting::from(reader.vector_routing_plan(
            &field,
            &query,
            1.0,
            reader.vector_search_budget(&field),
        )));
        let spent = Arc::new(tantivy::vector::AtomicVectorWork::default());
        let routing = VectorRouting::Shared(Arc::clone(&plan), Arc::clone(&spent));
        let order = [OrderByInfo {
            feature: OrderByFeature::VectorDistance {
                name: field,
                rti: 0,
                query_vector: QueryVector::Resolved(query),
                metric: VectorMetric::L2,
            },
            direction: SortDirection::AscNullsLast,
        }];
        let search = |limit, offset, routing: &VectorRouting| {
            reader
                .search_top_k_in_segments(
                    reader.segment_ids().into_iter(),
                    &order,
                    limit,
                    offset,
                    None,
                    None,
                    routing,
                )
                .results
                .into_iter()
                .map(|(_, doc)| doc)
                .collect::<Vec<_>>()
        };
        let expected = search(10, 0, &VectorRouting::Local);
        assert_eq!(expected.len(), 10);
        let mut actual = search(5, 0, &routing);
        assert_eq!(actual.len(), 5);
        assert!(spent.charged() > 0.0);
        // A peer can finish its last cluster after the shared ceiling is reached.
        plan.work().charge(1.0);
        let charged = plan.work().charged();
        actual.extend(search(5, 5, &routing));
        assert_eq!(actual, expected);
        assert_eq!(plan.work().charged(), charged);
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
                InjectedStatsFailure::Missing => unreachable!(),
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
        let index = crate::index::open_index(directory.clone()).unwrap();
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
        let impossible_query = range_query("id", 100, 200);
        let impossible = open_snapshot_reader(&index_rel, impossible_query.clone(), false);
        assert_eq!(impossible.segment_pruning_estimate().candidate_segments, 1);
        assert_eq!(
            impossible.estimate_docs(RowEstimate::Unknown).matching_docs,
            0
        );
        assert_eq!(
            impossible
                .build_query_tree_with_estimates(impossible_query)
                .unwrap()
                .estimated_docs,
            Some(0)
        );
        assert_eq!(
            impossible
                .directory
                .mutable_segment_materialized(&mutable_id),
            Some(false),
            "estimation must not materialize a mutable segment when persisted segments are pruned"
        );
        assert_pruning_matches_tantivy(&impossible, 0);
        // The immutable segment fits this range; the mutable one cannot prove coverage.
        use crate::index::stats::segments_for_partition;
        let partitioning = RangePartitioning {
            partition_by: FieldName::from("id"),
            split_points: vec![PdbOwnedValue::I64(11)],
        };
        let partition_segments = segments_for_partition(&reader, &partitioning, 0);
        assert!(!partition_segments.partially_included.is_empty());
        let bounds = range_query("id", 1, 10);
        let matching = open_snapshot_reader(&index_rel, bounds.clone(), false);
        assert_eq!(
            matching.estimate_docs(RowEstimate::Unknown).matching_docs,
            15
        );
        assert_eq!(
            matching
                .build_query_tree_with_estimates(bounds.clone())
                .unwrap()
                .estimated_docs,
            Some(15)
        );
        assert_eq!(
            matching.directory.mutable_segment_materialized(&mutable_id),
            Some(false),
            "estimation must scale persisted segments without materializing the mutable segment"
        );
        let results = reader.search_segments_with_range_filter(&partition_segments, &bounds);
        assert_eq!(results.count(), 10);
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

#[cfg(test)]
mod vector_probe_stats_tests {
    use super::{ProbeStats, SegmentId, probe_stats_to_segment_info};
    use tantivy::vector::VectorIoStats;

    // Exact-scoring I/O is exposed as additive scalar counters in per-segment EXPLAIN data.
    #[test]
    fn exact_io_counters_are_flat_segment_fields() {
        let id = SegmentId::from_bytes([7; 16]);
        let mut stats = ProbeStats::default();
        stats.exact_io = VectorIoStats {
            reads: 3,
            bytes_read: 64,
            storage_blocks: 5,
        };
        let info = probe_stats_to_segment_info(&[id], &[stats]);
        assert_eq!(info[&id]["exact_reads"], 3);
        assert_eq!(info[&id]["exact_bytes_read"], 64);
        assert_eq!(info[&id]["exact_storage_blocks"], 5);
        assert!(info[&id].get("exact_io").is_none());
    }
}

#[cfg(test)]
mod lookup_only_segment_tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::Bound;
    use std::ops::RangeInclusive;
    use tantivy::query::{BooleanQuery, EnableScoring, Query, RangeQuery, TermQuery, TermSetQuery};
    use tantivy::schema::{FAST, Field, INDEXED, IndexRecordOption, Schema, TEXT};
    use tantivy::{DocSet, Index, Score, TantivyDocument, Term, doc};

    #[derive(Clone, Debug)]
    enum BaseFilterSpec {
        MatchAll,
        Term(i64),
        TermSet(Vec<i64>),
        Range(RangeInclusive<i64>),
        Union(Box<Self>, Box<Self>),
        Intersection(Box<Self>, Box<Self>),
    }

    impl BaseFilterSpec {
        fn matches(&self, div: i64) -> bool {
            match self {
                Self::MatchAll => true,
                Self::Term(v) => div == *v,
                Self::TermSet(vs) => vs.contains(&div),
                Self::Range(r) => r.contains(&div),
                Self::Union(a, b) => a.matches(div) || b.matches(div),
                Self::Intersection(a, b) => a.matches(div) && b.matches(div),
            }
        }

        fn to_query(&self, div_field: Field) -> Option<Box<dyn Query>> {
            match self {
                Self::MatchAll => None,
                Self::Term(v) => Some(Box::new(TermQuery::new(
                    Term::from_field_i64(div_field, *v),
                    IndexRecordOption::Basic,
                ))),
                Self::TermSet(vs) => Some(Box::new(TermSetQuery::new(
                    vs.iter().map(|&v| Term::from_field_i64(div_field, v)),
                ))),
                Self::Range(r) => Some(Box::new(RangeQuery::new(
                    Bound::Included(Term::from_field_i64(div_field, *r.start())),
                    Bound::Included(Term::from_field_i64(div_field, *r.end())),
                ))),
                Self::Union(a, b) => Some(Box::new(BooleanQuery::union(vec![
                    a.to_query(div_field).unwrap(),
                    b.to_query(div_field).unwrap(),
                ]))),
                Self::Intersection(a, b) => Some(Box::new(BooleanQuery::intersection(vec![
                    a.to_query(div_field).unwrap(),
                    b.to_query(div_field).unwrap(),
                ]))),
            }
        }
    }

    fn arb_base_filter() -> impl Strategy<Value = BaseFilterSpec> {
        let divs = prop::sample::select(vec![0i64, 1, 10, 20, 25, 30, 50, 99, 100]);
        let leaf = prop_oneof![
            divs.clone().prop_map(BaseFilterSpec::Term),
            prop::collection::vec(divs.clone(), 0..=4).prop_map(BaseFilterSpec::TermSet),
            (divs.clone(), divs).prop_map(|(a, b)| BaseFilterSpec::Range(a.min(b)..=a.max(b))),
        ];
        let compound = leaf.prop_recursive(2, 8, 2, |inner| {
            prop_oneof![
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| BaseFilterSpec::Union(Box::new(a), Box::new(b))),
                (inner.clone(), inner)
                    .prop_map(|(a, b)| BaseFilterSpec::Intersection(Box::new(a), Box::new(b))),
            ]
        });
        prop_oneof![1 => Just(BaseFilterSpec::MatchAll), 5 => compound]
    }

    #[test]
    fn collect_lookup_only_segment_matches_ground_truth() {
        let mut sb = Schema::builder();
        let id_field = sb.add_i64_field("id", INDEXED | FAST);
        let div_field = sb.add_i64_field("division", INDEXED | FAST);
        let body_field = sb.add_text_field("body", TEXT);
        let index = Index::create_in_ram(sb.build());
        let mut writer = index
            .writer_with_num_threads::<TantivyDocument>(1, 50_000_000)
            .unwrap();

        // 250 docs: division 1 (> MAX_BASE_DRIVER_DOCS = 128), divisions 20 and 30 (<= 128).
        for id in 0i64..250 {
            let div = if id < 180 {
                1i64
            } else if id < 200 {
                20
            } else {
                30
            };
            let body = match id % 3 {
                0 => "zebra zebra",
                1 => "zebra",
                _ => "noise",
            };
            writer
                .add_document(doc!(id_field => id, div_field => div, body_field => body))
                .unwrap();
        }
        writer.commit().unwrap();
        for deleted in [5i64, 183, 210] {
            writer.delete_term(Term::from_field_i64(id_field, deleted));
        }
        writer.commit().unwrap();

        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        assert_eq!(searcher.segment_readers().len(), 1);
        let segment = searcher.segment_reader(0);
        let id_col = segment.fast_fields().i64("id").unwrap();
        let div_col = segment.fast_fields().i64("division").unwrap();
        let disjunct_query = TermQuery::new(
            Term::from_field_text(body_field, "zebra"),
            IndexRecordOption::WithFreqs,
        );
        let mut zebra_scores = vec![0.0; segment.max_doc() as usize];
        let mut scorer = disjunct_query
            .weight(EnableScoring::enabled_from_searcher(&searcher))
            .unwrap()
            .scorer(segment, 1.0)
            .unwrap();
        while scorer.doc() != tantivy::TERMINATED {
            zebra_scores[scorer.doc() as usize] = scorer.score();
            scorer.advance();
        }
        let disjunct_weight = Arc::new(LazyWeight::new(
            Box::new(disjunct_query),
            true,
            searcher.clone(),
        ));

        proptest!(|(
            lookup_ids in prop::collection::btree_set(-5i64..260, 0..=80),
            base_filter in arb_base_filter(),
            use_disjunct in any::<bool>(),
            score_threshold in prop_oneof![
                Just(None),
                Just(Some(-0.1f32)),
                Just(Some(0.0f32)),
                (0.01f32..3.0).prop_map(Some),
            ],
            pre_emitted_docs in prop::collection::btree_set(0u32..250, 0..=10),
        )| {
            let segment_ord = 3u32;
            let lookup_query = TermSetQuery::new(
                lookup_ids.iter().map(|&id| Term::from_field_i64(id_field, id)),
            );
            let lookup_weight = lookup_query
                .weight(EnableScoring::disabled_from_searcher(&searcher))
                .unwrap();
            let bf_weight = base_filter
                .to_query(div_field)
                .map(|q| Arc::new(LazyWeight::new(q, false, searcher.clone())));

            let mut emitted: crate::api::HashSet<DocAddress> = pre_emitted_docs
                .iter()
                .map(|&doc_id| DocAddress::new(segment_ord, doc_id))
                .collect();
            let initial_emitted = emitted.clone();
            let mut results = Vec::new();
            collect_lookup_only_segment(
                segment_ord,
                segment,
                lookup_weight.as_ref(),
                bf_weight.as_ref(),
                use_disjunct.then_some(&disjunct_weight),
                score_threshold,
                EstimateContext::default(),
                &mut emitted,
                &mut results,
            );

            let mut expected_results = Vec::new();
            let mut expected_emitted = initial_emitted;
            for doc_id in 0..segment.max_doc() {
                let addr = DocAddress::new(segment_ord, doc_id);
                if !segment.alive_bitset().is_none_or(|b| b.is_alive(doc_id))
                    || expected_emitted.contains(&addr)
                    || !lookup_ids.contains(&id_col.first(doc_id).unwrap())
                    || !base_filter.matches(div_col.first(doc_id).unwrap())
                {
                    continue;
                }
                let score: Score = if use_disjunct {
                    zebra_scores[doc_id as usize]
                } else {
                    0.0
                };
                if score_threshold.is_none_or(|t| score > t) {
                    expected_emitted.insert(addr);
                    expected_results.push((SearchIndexScore::new(score), addr));
                }
            }
            prop_assert_eq!(results, expected_results);
            prop_assert_eq!(emitted, expected_emitted);
        });
    }
}

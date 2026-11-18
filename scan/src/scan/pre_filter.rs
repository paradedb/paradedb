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

//! Pre-materialization dynamic filter support.
//!
//! See the [JoinScan README](../../postgres/customscan/joinscan/README.md) for
//! how dynamic filters fit into the overall pruning pipeline.
//!
//! Dynamic filters allow parent operators (e.g. `SortExec(TopK)` or `HashJoinExec`)
//! to push filters down into scan nodes so that rows failing the filter are pruned
//! before column materialization.
//!
//! There are two distinct mechanisms for dynamic filter pushdown:
//!
//! 1. **Query-Time Pushdown (Index):** Filters that are known before the scan begins
//!    (such as `InList` and `HashTableLookupExpr` predicates from a completed HashJoin
//!    build-side) are intercepted during the first `poll_next` of the scan stream. They
//!    are converted into native Tantivy queries (`TermSetQuery` / `TermPredicateQuery`)
//!    and `AND`ed into the main search query. This filters documents *while* executing
//!    the search, leveraging inverted index postings or dictionary/columnar fast fields.
//!
//! 2. **Pre-Filter Pushdown (Fast Fields):** Evolving thresholds (such as the rolling
//!    Top K threshold from `SortExec`) or filters that cannot be (or should not be on
//!    standard unsorted scans, e.g. high-density `InList`s that would fall back to Tantivy
//!    `LinearScan`; see [`InListLinearFallback`]) mapped to the inverted index are applied as
//!    `PreFilter`s *after* the search but *before* heap visibility (`fetch_ctids`) and Arrow
//!    column materialization. These evaluate directly against Tantivy fast fields (using
//!    term-ordinal bounds/sets for strings or direct numeric comparisons).
//!
//! # Data Flow
//!
//! ```text
//! HashJoinExec / SortExec(TopK)
//!   creates DynamicFilterPhysicalExpr (e.g. "col IN (...)" or "val < current_threshold")
//!        │
//!        │  FilterPushdown pass
//!        ▼
//! PgSearchScanPlan                   ← handle_child_pushdown_result stores
//!   .dynamic_filters                   the DynamicFilterPhysicalExpr.
//!        │
//!        │  at first poll_next
//!        ▼
//! try_dynamic_filter_pushdown()      ← Converts eligible InList and HashTableLookupExpr
//!                                      filters into Tantivy Queries (TermSetQuery /
//!                                      TermPredicateQuery) on the SearchIndexReader.
//!                                      Rewrites pushed-down DataFusion exprs to lit(true).
//!        │
//!        │  on first batch or when DynamicFilterTracker::changed() fires
//!        ▼
//! build_filters()                    ← calls DynamicFilterPhysicalExpr::current()
//!   → collect_filters()                to get the latest expression for remaining filters,
//!   → Vec<PreFilter>                   skips lit(true), and decomposes into PreFilter(s).
//!        │
//!        ▼
//! Scanner::next()                    ← applies PreFilters via apply_arrow()
//!   prunes doc IDs in-place            before materializing Arrow columns.
//! ```
//!
//! # Native DataFusion Evaluation
//!
//! `PreFilter`s do not execute custom matching logic. Instead, they leverage native DataFusion
//! `PhysicalExpr` evaluation over a mock `RecordBatch` containing the fetched fast-field columns at
//! their original schema indices (with zero-allocation `NullArray` placeholders in unreferenced slots).
//! For string columns, to avoid expensive materialization, the `PreFilter` rewrites the expression once
//! per segment (cached by `SegmentOrdinal`): translating string literals into local `UInt64` ordinal
//! bounds or `InList` ordinal hash sets and evaluating directly against the fetched term ordinals.
//! This allows complex expressions (e.g. `IS NULL OR col < 'abc'`) to be seamlessly evaluated by
//! Arrow's vectorized compute kernels.
//!
//! # Observability
//!
//! `EXPLAIN (ANALYZE)` on a `PgSearchScan` node reports query-time pushdown via
//! `dynamic_filter_pushdown_<strategy>` (`InList`) and
//! `dynamic_filter_pushdown_hash_<strategy>` (`HashTableLookupExpr`), and batch-level
//! pre-filtering via `rows_pruned` and `rows_scanned` (`rows_pruned > 0` confirms
//! pre-filtering ran). The `dynamic_filters=N` annotation in the non-ANALYZE plan shows
//! how many dynamic filters were attached to the scan.

use std::ops::Bound;
use std::sync::Arc;

use arrow_schema::SchemaRef;
use datafusion::arrow::array::{
    Array, ArrayRef, BinaryViewArray, BooleanArray, Float32Array, Float64Array, Int8Array,
    Int16Array, Int32Array, Int64Array, NullArray, StringViewArray, TimestampMicrosecondArray,
    UInt8Array, UInt16Array, UInt32Array, UInt64Array,
};
use datafusion::arrow::compute::cast;
use datafusion::arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use datafusion::arrow::record_batch::{RecordBatch, RecordBatchOptions};
use datafusion::common::ScalarValue;
use datafusion::common::tree_node::{Transformed, TransformedResult, TreeNode, TreeNodeRecursion};
use datafusion::logical_expr::Operator;
use datafusion::physical_expr::PhysicalExpr;
use datafusion::physical_expr::expressions::{
    BinaryExpr, CastExpr, Column, DynamicFilterPhysicalExpr, IsNullExpr, Literal, NotExpr,
};
use datafusion::physical_plan::expressions::InListExpr;
use datafusion::physical_plan::joins::HashTableLookupExpr;
use parking_lot::Mutex;
use tantivy::columnar::{ColumnType, MonotonicallyMappableToU64};
use tantivy::{Score, SegmentOrdinal};

use crate::api::HashSet;
use crate::index::fast_fields_helper::{FFHelper, FFType, NULL_TERM_ORDINAL};
use crate::postgres::pdb_owned_value::{PdbOwnedValue, PdbOwnedValueScalarExt};
use crate::postgres::types_arrow::datetime_to_pg_micros;
use crate::query::value_to_term;
use crate::scan::deferred_encode::is_deferred_field;
use tantivy::Term;
use tantivy::query::{
    BooleanQuery, BoundsRange, ConstScoreQuery, ExactPostings, Occur, Query, TermPredicate,
    TermPredicateQuery, TermQuery, TermSetQuery, TermSetStrategyConfig, TinySet,
};
use tantivy::schema::{DATE_TIME_PRECISION_INDEXED, FieldType, IndexRecordOption};

/// Point-lookup request payload pushed from [`crate::scan::rank_join_exec::RankJoinExec`] down to
/// [`crate::scan::execution_plan::PgSearchScanPlan`] (and across nested `RankJoinExec` nodes) to
/// trigger on-demand Random Access (`RA`) fetches for a batch of unresolved join keys.
///
/// # How `RankLookupPhysicalExpr` differs from a normal dynamic filter
///
/// If a rank join only needed Sorted Access (`SA`), its child inputs would simply be standard
/// sorted `SendableRecordBatchStream` iterators with no special side-channel. Random Access (`RA`),
/// however, acts like an extension trait on `Stream` that adds point-lookup methods and richer
/// exhaustion states. Because DataFusion's `ExecutionPlan::execute` returns an opaque
/// `SendableRecordBatchStream` wrapped by intermediate unary operators (`ProjectionExec`,
/// `FilterPassthroughExec`, `TantivyFetchExec`, `TantivyDecodeExec`, `VisibilityFilterExec`),
/// `RankJoinExec` cannot call custom methods on the leaf stream directly. Instead, it tunnels a
/// synchronous, bidirectional RPC protocol through DataFusion's [`DynamicFilterPhysicalExpr`]:
///
/// 1. **One-shot RPC request/reset lifecycle (not a persistent monotonic filter):**
///    Normal dynamic filters (`HashJoinExec`'s build-side `InListExpr` / `HashTableLookupExpr`, or
///    `SortExec` / `SegmentedTopKExec`'s rolling threshold) are published once or tightened
///    monotonically and restrict every subsequent batch. By contrast, `RankJoinExec` writes a
///    `RankLookupPhysicalExpr` into a side's `DynamicFilterPhysicalExpr` immediately before polling
///    `child_stream.next().await`
///    for an `RA` probe. At the top of `poll_next` in `lazy_sort.rs`,
///    `LazySortStreamState::take_pending_ra_lookup` immediately consumes the request by resetting
///    `dyn_filter.update(lit(true))` and executes a single out-of-band `reader.search_ra_lookup`
///    batch without advancing the `SA` window cursor (yielding an empty `RecordBatch` if 0 rows
///    matched, since returning `None` would close the stream).
///
/// 2. **Reverse (child `->` parent) exhaustion signaling:**
///    When a leaf scan exhausts its `SA` stream, `LazySortStreamState::signal_parent_sa_exhausted`
///    writes a sentinel literal back into the same `DynamicFilterPhysicalExpr` and yields an empty
///    `RecordBatch` so `RankJoinExec` can distinguish why the batch was empty while keeping the
///    stream open for future `RA` probes:
///    - `lit(false)` (`ExhaustionBehavior::KeepAliveForRa`): `SA` is exhausted, but `RA` remains
///      capable of probing the unvisited tail of the index.
///    - `lit(ScalarValue::Boolean(None))` (`ExhaustionBehavior::SignalBothExhausted`): both `SA`
///      and `RA` are exhausted because `SA` has already visited every matching document.
///
/// 3. **Automatic column-index remapping & cascading (`N >= 3`) interception:**
///    Because `RankLookupPhysicalExpr` implements [`PhysicalExpr`] (`children()` returning `cols`
///    and `with_new_children()` rebuilding `cols`), DataFusion's `FilterPushdown` pass and
///    intermediate `ProjectionExec` nodes automatically remap its `Column` indices as it travels
///    down the physical plan. In a cascading multi-way rank join, an inner `RankJoinExec`
///    intercepts its parent's `DynamicFilterPhysicalExpr`, translates `score_threshold` via
///    `ScoreCombiner::threshold_for`, and forwards a remapped `RankLookupPhysicalExpr` to its own
///    children.
///
/// 4. **Dual execution mode (leaf Tantivy query vs intermediate in-memory `evaluate`):**
///    - At a leaf `PgSearchScanPlan`, `PhysicalExpr::evaluate` is never called on
///      `RankLookupPhysicalExpr`. Instead, [`try_convert_rank_lookup_to_query`] converts `cols`
///      and `keys` into a Tantivy `TermSetQuery` (or `BooleanQuery` for composite keys), and
///      `score_threshold` (`score > threshold`) is passed directly into
///      `SearchIndexReader::search_ra_lookup` to drive Tantivy's `PruningScorer` (Block-Max WAND).
///    - At an intermediate `RankJoinExec` (`find_matching_buffered_rows`),
///      `RankLookupPhysicalExpr::evaluate(&batch)` *is* called as an Arrow [`PhysicalExpr`] to
///      filter batches that the inner join already buffered in memory during prior `SA` pulls.
#[derive(Debug, Clone)]
pub struct RankLookupPhysicalExpr {
    cols: Vec<Arc<dyn PhysicalExpr>>,
    keys: Vec<Vec<ScalarValue>>,
    /// Optional minimum score threshold using strict greater-than (`score > threshold`) semantics,
    /// matching Tantivy's `PruningScorer`.
    score_threshold: Option<Score>,
}

impl RankLookupPhysicalExpr {
    pub fn new(cols: Vec<Arc<dyn PhysicalExpr>>, keys: Vec<Vec<ScalarValue>>) -> Self {
        Self {
            cols,
            keys,
            score_threshold: None,
        }
    }

    pub fn with_score_threshold(mut self, score_threshold: Option<Score>) -> Self {
        self.score_threshold = score_threshold;
        self
    }

    pub fn with_cols(mut self, cols: Vec<Arc<dyn PhysicalExpr>>) -> Self {
        self.cols = cols;
        self
    }

    pub fn with_keys_and_threshold(
        &self,
        keys: Vec<Vec<ScalarValue>>,
        score_threshold: Option<Score>,
    ) -> Self {
        Self {
            cols: self.cols.clone(),
            keys,
            score_threshold,
        }
    }

    pub fn keys(&self) -> &[Vec<ScalarValue>] {
        &self.keys
    }

    pub fn score_threshold(&self) -> Option<Score> {
        self.score_threshold
    }
}

impl std::fmt::Display for RankLookupPhysicalExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RankLookup([")?;
        for (i, c) in self.cols.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{c}")?;
        }
        if let Some(t) = self.score_threshold {
            write!(f, "], keys={}, score_threshold={t})", self.keys.len())
        } else {
            write!(f, "], keys={})", self.keys.len())
        }
    }
}

impl PartialEq for RankLookupPhysicalExpr {
    fn eq(&self, other: &Self) -> bool {
        self.cols.len() == other.cols.len()
            && self
                .cols
                .iter()
                .zip(other.cols.iter())
                .all(|(a, b)| a.as_ref() == b.as_ref())
            && self.keys == other.keys
            && self.score_threshold.map(f32::to_bits) == other.score_threshold.map(f32::to_bits)
    }
}

impl Eq for RankLookupPhysicalExpr {}

impl std::hash::Hash for RankLookupPhysicalExpr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.cols.len().hash(state);
        for c in &self.cols {
            c.as_ref().hash(state);
        }
        self.keys.hash(state);
        self.score_threshold.map(f32::to_bits).hash(state);
    }
}

impl PhysicalExpr for RankLookupPhysicalExpr {
    fn data_type(&self, _input_schema: &Schema) -> datafusion::common::Result<DataType> {
        Ok(DataType::Boolean)
    }

    fn nullable(&self, _input_schema: &Schema) -> datafusion::common::Result<bool> {
        Ok(false)
    }

    fn evaluate(
        &self,
        batch: &RecordBatch,
    ) -> datafusion::common::Result<datafusion::physical_plan::ColumnarValue> {
        let num_rows = batch.num_rows();
        if self.cols.is_empty() || self.keys.is_empty() || num_rows == 0 {
            return Ok(datafusion::physical_plan::ColumnarValue::Array(Arc::new(
                BooleanArray::from(vec![false; num_rows]),
            )));
        }

        let mut col_arrays = Vec::with_capacity(self.cols.len());
        for col_expr in &self.cols {
            let val = col_expr.evaluate(batch)?;
            col_arrays.push(val.into_array(num_rows)?);
        }

        let mut matches = Vec::with_capacity(num_rows);
        if self.cols.len() == 1 {
            let mut key_set = HashSet::default();
            for k in &self.keys {
                if let Some(v) = k.first() {
                    key_set.insert(v.clone());
                }
            }
            let arr = &col_arrays[0];
            for row_idx in 0..num_rows {
                let sv = ScalarValue::try_from_array(arr.as_ref(), row_idx)?;
                matches.push(!sv.is_null() && key_set.contains(&sv));
            }
        } else {
            let mut key_set: HashSet<Vec<ScalarValue>> = HashSet::default();
            for k in &self.keys {
                key_set.insert(k.clone());
            }
            let mut row_key = Vec::with_capacity(self.cols.len());
            for row_idx in 0..num_rows {
                row_key.clear();
                let mut any_null = false;
                for arr in &col_arrays {
                    let sv = ScalarValue::try_from_array(arr.as_ref(), row_idx)?;
                    if sv.is_null() {
                        any_null = true;
                        break;
                    }
                    row_key.push(sv);
                }
                matches.push(!any_null && key_set.contains(&row_key));
            }
        }

        Ok(datafusion::physical_plan::ColumnarValue::Array(Arc::new(
            BooleanArray::from(matches),
        )))
    }

    fn children(&self) -> Vec<&Arc<dyn PhysicalExpr>> {
        self.cols.iter().collect()
    }

    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn PhysicalExpr>>,
    ) -> datafusion::common::Result<Arc<dyn PhysicalExpr>> {
        Ok(Arc::new(Self {
            cols: children,
            keys: self.keys.clone(),
            score_threshold: self.score_threshold,
        }))
    }

    fn fmt_sql(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

/// Returns `true` if `col_name` with Arrow `data_type` can be converted into a Tantivy query
/// by [`try_convert_rank_lookup_to_query`].
pub fn supports_ra_lookup_column(
    col_name: &str,
    data_type: &DataType,
    schema: &crate::schema::SearchIndexSchema,
) -> bool {
    let Some(field) = schema.search_field(col_name) else {
        return false;
    };
    if field.is_text() && !field.is_keyword() {
        return false;
    }
    let tantivy_schema = schema.tantivy_schema();
    let Ok(tantivy_field) = tantivy_schema.get_field(col_name) else {
        return false;
    };
    let tantivy_field_type = tantivy_schema.get_field_entry(tantivy_field).field_type();
    if !tantivy_field_type.is_indexed()
        || matches!(
            tantivy_field_type,
            tantivy::schema::FieldType::JsonObject(_)
        )
    {
        return false;
    }
    PdbOwnedValue::supports_execution_data_type(data_type, &field.field_type())
}

/// Returns `true` if `col_name` with Arrow `data_type` can push down a single-column dynamic join
/// filter (`InListExpr` and/or `HashTableLookupExpr`) into Tantivy via [`try_dynamic_filter_pushdown`]
/// regardless of build-side cardinality.
pub fn supports_probe_dynamic_filter_column(
    col_name: &str,
    data_type: &DataType,
    schema: &crate::schema::SearchIndexSchema,
) -> bool {
    let Some(search_field) = schema.search_field(col_name) else {
        return false;
    };
    if search_field.is_text()
        && !search_field.is_keyword()
        && crate::gucs::hash_join_inlist_pushdown_max_distinct_values() > 0
    {
        return false;
    }
    let tantivy_schema = schema.tantivy_schema();
    let Ok(tantivy_field) = tantivy_schema.get_field(col_name) else {
        return false;
    };
    let tantivy_field_type = tantivy_schema.get_field_entry(tantivy_field).field_type();
    resolve_hash_lookup_exact_postings(data_type, &search_field, tantivy_field_type).is_some()
}

/// Convert a [`RankLookupPhysicalExpr`] into a zero-scoring Tantivy query (`ConstScoreQuery(..., 0.0)`)
/// suitable for `reader.and_query(...)` during a Random Access (`RA`) point lookup.
pub fn try_convert_rank_lookup_to_query(
    lookup: &RankLookupPhysicalExpr,
    output_schema: &Schema,
    schema: &crate::schema::SearchIndexSchema,
    index_created_by_version: Option<crate::api::version::Version>,
    strategy_sink: Option<Arc<std::sync::atomic::AtomicU8>>,
) -> Option<Box<dyn Query>> {
    if lookup.cols.is_empty() || lookup.keys.is_empty() {
        return None;
    }

    let tantivy_schema = schema.tantivy_schema();
    if lookup.cols.len() == 1 {
        let col = lookup.cols[0].downcast_ref::<Column>()?;
        let output_field = output_schema.fields().get(col.index())?;
        if output_field.name() != col.name() {
            return None;
        }
        let field = schema.search_field(output_field.name())?;
        if field.is_text() && !field.is_keyword() {
            return None;
        }
        let field_type = field.field_type();
        let tantivy_field = tantivy_schema.get_field(output_field.name()).ok()?;
        let tantivy_field_type = tantivy_schema.get_field_entry(tantivy_field).field_type();

        let mut terms = Vec::with_capacity(lookup.keys.len());
        for key_tuple in &lookup.keys {
            let scalar = key_tuple.first()?;
            if scalar.is_null() {
                continue;
            }
            let owned_value = PdbOwnedValue::from_execution_scalar(scalar, &field_type)?;
            let term = value_to_term(
                tantivy_field,
                &owned_value,
                tantivy_field_type,
                None,
                index_created_by_version,
            )
            .ok()?;
            terms.push(term);
        }
        if terms.is_empty() {
            return None;
        }
        let cfg = TermSetStrategyConfig {
            gallop_enabled: crate::gucs::term_set_gallop_enabled(),
            bitset_max_density_unique: crate::gucs::term_set_bitset_max_density_unique()
                .max(crate::gucs::term_set_bitset_max_density_multi()),
            bitset_max_density_multi: crate::gucs::term_set_bitset_max_density_multi(),
            strategy_sink,
            ..TermSetStrategyConfig::default()
        };
        let term_set_query = TermSetQuery::new(terms).with_strategy_config(cfg);
        return Some(Box::new(ConstScoreQuery::new(term_set_query, 0.0)));
    }

    // Composite key: OR of ANDed TermQuerys
    let mut col_infos = Vec::with_capacity(lookup.cols.len());
    for col_expr in &lookup.cols {
        let col = col_expr.downcast_ref::<Column>()?;
        let output_field = output_schema.fields().get(col.index())?;
        if output_field.name() != col.name() {
            return None;
        }
        let field = schema.search_field(output_field.name())?;
        if field.is_text() && !field.is_keyword() {
            return None;
        }
        let tantivy_field = tantivy_schema.get_field(output_field.name()).ok()?;
        let tantivy_field_type = tantivy_schema.get_field_entry(tantivy_field).field_type();
        col_infos.push((field.field_type(), tantivy_field, tantivy_field_type));
    }

    let mut tuple_queries: Vec<(Occur, Box<dyn Query>)> = Vec::with_capacity(lookup.keys.len());
    for key_tuple in &lookup.keys {
        if key_tuple.len() != col_infos.len() {
            return None;
        }
        let mut conj: Vec<(Occur, Box<dyn Query>)> = Vec::with_capacity(col_infos.len());
        let mut any_null = false;
        for (scalar, (field_type, tantivy_field, tantivy_field_type)) in
            key_tuple.iter().zip(col_infos.iter())
        {
            if scalar.is_null() {
                any_null = true;
                break;
            }
            let owned_value = PdbOwnedValue::from_execution_scalar(scalar, field_type)?;
            let term = value_to_term(
                *tantivy_field,
                &owned_value,
                tantivy_field_type,
                None,
                index_created_by_version,
            )
            .ok()?;
            conj.push((
                Occur::Must,
                Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
            ));
        }
        if !any_null && !conj.is_empty() {
            tuple_queries.push((Occur::Should, Box::new(BooleanQuery::new(conj))));
        }
    }

    if tuple_queries.is_empty() {
        return None;
    }
    Some(Box::new(ConstScoreQuery::new(
        BooleanQuery::new(tuple_queries),
        0.0,
    )))
}

/// A pre-materialization filter applied inside `Scanner::next()`.
///
/// Wraps a DataFusion `PhysicalExpr` that has been validated to only contain
/// operations we can evaluate early (e.g. before fetching expensive string dictionaries).
pub struct PreFilter {
    /// The validated DataFusion physical expression.
    pub expr: Arc<dyn PhysicalExpr>,
    /// The indices of the fast fields this expression requires.
    pub required_columns: Vec<usize>,
    /// Cached segment-rewritten expression `(segment_ord, rewritten_expr)` so
    /// `apply_arrow` only runs the AST rewrite once per segment rather than
    /// on every batch.
    cached_segment_expr: Mutex<Option<(SegmentOrdinal, Arc<dyn PhysicalExpr>)>>,
}

/// A wrapper bundling a list of `PreFilter`s with the schema they apply to.
pub struct PreFilters<'a> {
    pub filters: &'a [PreFilter],
    pub schema: &'a SchemaRef,
}

impl PreFilter {
    /// Evaluate the pre-filter against a batch of memoized fast-field columns.
    /// Returns a boolean mask of rows that pass the filter.
    pub fn apply_arrow(
        &self,
        ffhelper: &FFHelper,
        segment_ord: SegmentOrdinal,
        memoized_columns: &[Option<ArrayRef>],
        schema: &SchemaRef,
        num_rows: usize,
    ) -> Result<BooleanArray, String> {
        // 1. Rewrite the expression for the current segment (cached per segment_ord).
        // String literal comparisons and `InList`s are rewritten to segment-local ordinal
        // comparisons/sets. Caching by `segment_ord` is safe because `build_filters` unwraps
        // top-level `DynamicFilterPhysicalExpr::current()`, `is_supported` rejects nested
        // `DynamicFilterPhysicalExpr`s, and `PgSearchScanPlan` rebuilds `PreFilter`s whenever
        // `DynamicFilterTracker::changed()` reports an update — so `self.expr` is static for the
        // lifetime of this `PreFilter`.
        let rewritten_expr = {
            let mut cache = self.cached_segment_expr.lock();
            if let Some((cached_ord, cached_expr)) = cache.as_ref()
                && *cached_ord == segment_ord
            {
                Arc::clone(cached_expr)
            } else {
                let rewritten = self
                    .expr
                    .clone()
                    .transform_down(|node| {
                        if let Some(binary) = node.downcast_ref::<BinaryExpr>() {
                            if let Some(rewritten) =
                                try_rewrite_binary(binary, ffhelper, segment_ord, schema)?
                            {
                                // Leaf `Column op Literal` has already been rewritten; jump
                                // past its children.
                                return Ok(Transformed::new(
                                    rewritten,
                                    true,
                                    TreeNodeRecursion::Jump,
                                ));
                            }
                        } else if let Some(in_list) = node.downcast_ref::<InListExpr>() {
                            if let Some(rewritten) =
                                try_rewrite_in_list(in_list, ffhelper, segment_ord, schema)?
                            {
                                return Ok(Transformed::new(
                                    rewritten,
                                    true,
                                    TreeNodeRecursion::Jump,
                                ));
                            }
                            if in_list.expr().is::<Column>() {
                                // Skip `InListExpr::map_children`, which calls `children()` and
                                // allocates/clones a `Vec` of every literal in `in_list.list()`.
                                return Ok(Transformed::new(node, false, TreeNodeRecursion::Jump));
                            }
                        } else if node.is::<HashTableLookupExpr>() {
                            return Ok(Transformed::new(node, false, TreeNodeRecursion::Jump));
                        }
                        Ok(Transformed::no(node))
                    })
                    .data()
                    .map_err(|e| format!("Failed to rewrite string expr: {}", e))?;
                *cache = Some((segment_ord, Arc::clone(&rewritten)));
                rewritten
            }
        };

        // 2. Build a RecordBatch from memoized_columns preserving original schema column indices.
        // Non-required columns are filled with zero-buffer `NullArray` placeholders so we don't
        // need to remap `Column` indices. Even inside the per-segment cache, remapping `col` in a
        // numeric `InListExpr` would have to go through `InListExpr::with_new_children` (since
        // `InListExpr::new` is private in DataFusion), allocating and cloning the entire literal
        // list twice per segment.
        let null_array: ArrayRef = Arc::new(NullArray::new(num_rows));
        let mut fields: Vec<Field> = schema
            .fields()
            .iter()
            .map(|f| Field::new(f.name(), DataType::Null, true))
            .collect();
        let mut arrays: Vec<ArrayRef> = vec![null_array; schema.fields().len()];

        for &ff_index in &self.required_columns {
            let mut array = memoized_columns[ff_index]
                .as_ref()
                .ok_or_else(|| format!("Column {} not fetched", ff_index))?
                .clone();

            let schema_field = schema.field(ff_index);
            let schema_type = schema_field.data_type();

            // Cast numeric fast fields to match the expected DataFusion schema type
            if !is_string_like_field(schema_field) && array.data_type() != schema_type {
                array = cast(&array, schema_type).map_err(|e| {
                    format!(
                        "Failed to cast columnar field from {:?} to DataFusion schema type {:?}: {}",
                        array.data_type(), schema_type, e
                    )
                })?;
            }
            // Note: The schema of the array might differ from the global schema
            // (e.g. UInt64 ordinals instead of Utf8). DataFusion `Column` exprs just extract by name/index,
            // so we must build the batch schema to match the *actual* array types we pass in.
            fields[ff_index] = Field::new(schema_field.name(), array.data_type().clone(), true);
            arrays[ff_index] = array;
        }

        let batch_schema = Arc::new(Schema::new(fields));
        let options = RecordBatchOptions::new().with_row_count(Some(num_rows));
        let batch = RecordBatch::try_new_with_options(batch_schema, arrays, &options)
            .map_err(|e| format!("Failed to build RecordBatch: {}", e))?;

        // 3. Evaluate the rewritten expression natively via DataFusion.
        let columnar_value = rewritten_expr
            .evaluate(&batch)
            .map_err(|e| format!("Failed to evaluate expr: {}", e))?;

        let array = columnar_value
            .into_array(num_rows)
            .map_err(|e| format!("Failed to convert into array: {}", e))?;

        let bool_array = array
            .as_any()
            .downcast_ref::<BooleanArray>()
            .ok_or_else(|| "Result is not a BooleanArray".to_string())?
            .clone();

        Ok(bool_array)
    }
}

/// Recursively decomposes and validates a `PhysicalExpr` into `PreFilter`s.
///
/// Top-level `AND` operations are split into separate `PreFilter`s to allow early
/// short-circuiting in the scanner. Expressions containing unsupported nodes
/// (e.g. non-comparison operators, functions) are safely skipped.
///
/// Note: [`RankLookupPhysicalExpr`] is intentionally not handled here because
/// `LazySortStreamState::take_pending_ra_lookup` consumes any pending `RA` request and resets its
/// [`DynamicFilterPhysicalExpr`] to `lit(true)` at the top of `poll_next`, before `SA` window
/// pre-filter collection runs.
pub fn collect_filters(
    expr: &Arc<dyn PhysicalExpr>,
    schema: &SchemaRef,
    out: &mut Vec<PreFilter>,
    score_col_schema_idx: Option<usize>,
    score_threshold: &mut Option<Score>,
) -> datafusion::error::Result<()> {
    // Split top-level ANDs to maximize early pruning
    if let Some(binary) = expr.downcast_ref::<BinaryExpr>()
        && matches!(binary.op(), Operator::And)
    {
        collect_filters(
            binary.left(),
            schema,
            out,
            score_col_schema_idx,
            score_threshold,
        )?;
        collect_filters(
            binary.right(),
            schema,
            out,
            score_col_schema_idx,
            score_threshold,
        )?;
        return Ok(());
    }

    // Ignore trivial `true` literals (e.g. uninitialized TopK dynamic filters or
    // `InList` / `HashTableLookupExpr` nodes rewritten to `lit(true)` after index pushdown).
    if let Some(lit) = expr.downcast_ref::<Literal>()
        && matches!(lit.value(), ScalarValue::Boolean(Some(true)))
    {
        return Ok(());
    }

    let threshold = match (
        try_extract_score_threshold(expr, score_col_schema_idx),
        &score_threshold,
    ) {
        (Some(new), Some(existing)) => Some(new.min(*existing)),
        (Some(new), None) => Some(new),
        (None, Some(existing)) => Some(*existing),
        (None, None) => None,
    };
    *score_threshold = threshold;

    // Check if the expression is supported for pre-filtering
    let mut required_columns = Vec::new();
    if is_supported(expr, schema, &mut required_columns)? {
        required_columns.sort_unstable();
        required_columns.dedup();
        out.push(PreFilter {
            expr: Arc::clone(expr),
            required_columns,
            cached_segment_expr: Mutex::new(None),
        });
    }
    Ok(())
}

/// Check for expressions that we know always evaluate to false
fn expr_always_false(expr: &Arc<dyn PhysicalExpr>, score_col_schema_idx: usize) -> bool {
    // FALSE literals are obviously always false
    if let Some(lit) = expr.downcast_ref::<Literal>() {
        return matches!(lit.value(), ScalarValue::Boolean(Some(false)));
    }
    // score is never null, so 'score IS NULL' is always false
    if let Some(is_null_expr) = expr.downcast_ref::<IsNullExpr>()
        && let Some(col) = is_null_expr.arg().downcast_ref::<Column>()
    {
        return col.index() == score_col_schema_idx;
    }
    false
}

/// Attempt to extract a minimum score threshold from the expression. Bounds propagate as:
/// - `score > t`  => `t`
/// - `score = t`  => `t.next_down()` (rows at exactly `t` must survive)
/// - `AND`        => the minimum of the bounds from either side (so it holds for the whole conjunction)
/// - `OR`  
///     - If both sides contain a bound, keep the minimum. If one side has a bound, use
///       it only in the case the other side always evaluates to false. Any OR with a
///       possibly-true non-score expression cannot provide a threshold, as the non-score
///       arm may be true
///
/// The returned threshold value assumes the threshold check uses > (greater-than) semantics.
/// ASSUMPTION: We intentionally don't check the Operator::Lt variant as DataFusion always puts the column
/// on the left.
///
/// This is necessary for using the blockmax-wand optimization in joins
fn try_extract_score_threshold(
    expr: &Arc<dyn PhysicalExpr>,
    score_col_schema_idx: Option<usize>,
) -> Option<Score> {
    let score_col_schema_idx = score_col_schema_idx?;
    let binary_expr = expr.downcast_ref::<BinaryExpr>()?;
    match binary_expr.op() {
        Operator::Gt => {
            // Look for a binary expr that looks like: score > f32
            let col = binary_expr.left().downcast_ref::<Column>()?;
            if col.index() != score_col_schema_idx {
                return None;
            }
            match binary_expr.right().downcast_ref::<Literal>()?.value() {
                ScalarValue::Float32(Some(t)) => Some(*t),
                _ => None,
            }
        }
        Operator::Eq | Operator::GtEq => {
            // Look for a binary expr that looks like: score = f32 or score >= f32
            let col = binary_expr.left().downcast_ref::<Column>()?;
            if col.index() != score_col_schema_idx {
                return None;
            }
            match binary_expr.right().downcast_ref::<Literal>()?.value() {
                // PruningScorer's assume the threshold has greater-than semantics, so
                // take the next representable value below the eq/gte check to keep the threshold valid
                ScalarValue::Float32(Some(t)) => Some(t.next_down()),
                _ => None,
            }
        }
        Operator::And => {
            match (
                try_extract_score_threshold(binary_expr.left(), Some(score_col_schema_idx)),
                try_extract_score_threshold(binary_expr.right(), Some(score_col_schema_idx)),
            ) {
                (Some(left), Some(right)) => Some(left.min(right)),
                (Some(v), None) | (None, Some(v)) => Some(v),
                (None, None) => None,
            }
        }
        Operator::Or => {
            // Members of an OR expression that are always false can be safely ignored, so we can
            // try to pull the score threshold from the other side.
            if expr_always_false(binary_expr.left(), score_col_schema_idx) {
                try_extract_score_threshold(binary_expr.right(), Some(score_col_schema_idx))
            } else if expr_always_false(binary_expr.right(), score_col_schema_idx) {
                try_extract_score_threshold(binary_expr.left(), Some(score_col_schema_idx))
            }
            // If both sides have a threshold-containing part, take the lower threshold: then any matching
            // row satisfies one of the arms and therefore exceeds the smaller bound, so pruning by it is safe
            else if let (Some(left), Some(right)) = (
                try_extract_score_threshold(binary_expr.left(), Some(score_col_schema_idx)),
                try_extract_score_threshold(binary_expr.right(), Some(score_col_schema_idx)),
            ) {
                Some(left.min(right))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Helper to centrally identify string, bytes, dictionary, and deferred string columns. A
/// deferred column is a `UInt64` of term ordinals that string literals compare against once
/// `try_rewrite_binary` has translated them, so it counts as a string here.
fn is_string_like_field(field: &Field) -> bool {
    is_deferred_field(field)
        || matches!(
            field.data_type(),
            DataType::Utf8
                | DataType::LargeUtf8
                | DataType::Utf8View
                | DataType::Binary
                | DataType::LargeBinary
                | DataType::BinaryView
                | DataType::Dictionary(_, _)
        )
}
/// Validates that a physical `Column`'s index is within `schema`.
///
/// Physical `Column` indices are remapped to the scan schema during `FilterPushdown`; an
/// out-of-bounds index indicates a plan remapping bug rather than an unsupported predicate, so we
/// surface it as an internal error rather than silently leaving pre-filter performance on the
/// table in release/benchmark builds.
fn check_column_index(col: &Column, schema: &Schema) -> datafusion::error::Result<usize> {
    let idx = col.index();
    if idx < schema.fields().len() {
        Ok(idx)
    } else {
        Err(datafusion::common::internal_datafusion_err!(
            "pre_filter: column '{}' has physical index {} which is out of bounds \
             for schema with {} fields",
            col.name(),
            idx,
            schema.fields().len()
        ))
    }
}

/// Validates that an expression only contains nodes we can evaluate during pre-filtering.
///
/// NOTE: When this function returns `TreeNodeRecursion::Stop`, it correctly halts *all*
/// traversal across the entire expression tree. If an OR branch contains an unsupported
/// child, the entire expression is rejected.
fn is_supported(
    expr: &Arc<dyn PhysicalExpr>,
    schema: &SchemaRef,
    required_columns: &mut Vec<usize>,
) -> datafusion::error::Result<bool> {
    let mut supported = true;
    expr.apply(|node| {
        if let Some(col) = node.downcast_ref::<Column>() {
            required_columns.push(check_column_index(col, schema)?);
        } else if node.is::<Literal>() {
            // Allowed
        } else if let Some(binary) = node.downcast_ref::<BinaryExpr>() {
            // Only logical and simple comparison operators are allowed
            match binary.op() {
                Operator::Eq
                | Operator::NotEq
                | Operator::Lt
                | Operator::LtEq
                | Operator::Gt
                | Operator::GtEq
                | Operator::And
                | Operator::Or => {}
                _ => {
                    supported = false;
                    return Ok(TreeNodeRecursion::Stop);
                }
            }
        } else if node.is::<IsNullExpr>() || node.is::<NotExpr>() {
            // Allowed
        } else if let Some(in_list) = node.downcast_ref::<InListExpr>() {
            if !is_supported(in_list.expr(), schema, required_columns)?
                || !in_list.list().iter().all(|e| e.is::<Literal>())
            {
                supported = false;
                return Ok(TreeNodeRecursion::Stop);
            }
            // All items in `in_list.list()` are verified `Literal`s. Skip visiting
            // `InListExpr::children()`, which allocates a `Vec` of `1 + list.len()` refs.
            return Ok(TreeNodeRecursion::Jump);
        } else if node.is::<HashTableLookupExpr>() {
            // Eligible single-column `HashTableLookupExpr`s are already converted to
            // `TermPredicateQuery` and replaced with `lit(true)` by `try_dynamic_filter_pushdown`.
            // Remaining lookups (e.g. multi-column composite keys, or lookups nested under `OR`)
            // can only run in `PreFilter` on non-string columns because string fast fields carry
            // segment-local `UInt64` ordinals rather than `Utf8View` values here.
            let mut is_numeric = true;
            let mut lookup_columns = Vec::new();

            // We manually inspect the subtree to check the data types of the columns it uses
            node.apply(|sub_node| {
                if let Some(col) = sub_node.downcast_ref::<Column>() {
                    let idx = check_column_index(col, schema)?;
                    if is_string_like_field(schema.field(idx)) {
                        is_numeric = false;
                        return Ok(TreeNodeRecursion::Stop);
                    }
                    lookup_columns.push(idx);
                }
                Ok(TreeNodeRecursion::Continue)
            })?;

            if !is_numeric {
                supported = false;
                return Ok(TreeNodeRecursion::Stop);
            }

            required_columns.extend(lookup_columns);

            // We tell DataFusion's main traversal loop to skip visiting
            // the children of this HashTableLookupExpr, as the child is likely
            // an internal DataFusion hashing node that isn't on our allowlist.
            return Ok(TreeNodeRecursion::Jump);
        } else {
            // Any other node type (e.g. CAST, LIKE, UDFs) blocks the expression from pre-filtering
            supported = false;
            return Ok(TreeNodeRecursion::Stop);
        }

        Ok(TreeNodeRecursion::Continue)
    })?;
    Ok(supported)
}

/// Attempts to rewrite a binary expression involving a String/Bytes column and a Literal
/// into an equivalent expression over segment-local ordinals.
fn try_rewrite_binary(
    binary: &BinaryExpr,
    ffhelper: &FFHelper,
    segment_ord: SegmentOrdinal,
    schema: &SchemaRef,
) -> datafusion::error::Result<Option<Arc<dyn PhysicalExpr>>> {
    let left_col = binary.left().downcast_ref::<Column>();
    let right_lit = binary.right().downcast_ref::<Literal>();

    if let (Some(col), Some(lit)) = (left_col, right_lit) {
        return rewrite_col_op_lit(col, binary.op(), lit, ffhelper, segment_ord, schema);
    }

    let left_lit = binary.left().downcast_ref::<Literal>();
    let right_col = binary.right().downcast_ref::<Column>();

    if let (Some(lit), Some(col)) = (left_lit, right_col)
        && let Some(flipped_op) = flip_operator(binary.op())
    {
        return rewrite_col_op_lit(col, &flipped_op, lit, ffhelper, segment_ord, schema);
    }

    Ok(None)
}

fn extract_bytes_from_scalar(scalar: &ScalarValue) -> Option<Option<&[u8]>> {
    match scalar {
        ScalarValue::Utf8(Some(s))
        | ScalarValue::LargeUtf8(Some(s))
        | ScalarValue::Utf8View(Some(s)) => Some(Some(s.as_bytes())),
        ScalarValue::Binary(Some(b))
        | ScalarValue::LargeBinary(Some(b))
        | ScalarValue::BinaryView(Some(b)) => Some(Some(b.as_slice())),

        ScalarValue::Utf8(None)
        | ScalarValue::LargeUtf8(None)
        | ScalarValue::Utf8View(None)
        | ScalarValue::Binary(None)
        | ScalarValue::LargeBinary(None)
        | ScalarValue::BinaryView(None) => Some(None),

        _ => None,
    }
}

/// Attempts to rewrite a String/Bytes `col [NOT] IN (...)` expression into an equivalent
/// `InListExpr` over segment-local `UInt64` term ordinals (backed by DataFusion's pre-built
/// `StaticFilter` hash set).
fn try_rewrite_in_list(
    in_list: &InListExpr,
    ffhelper: &FFHelper,
    segment_ord: SegmentOrdinal,
    schema: &SchemaRef,
) -> datafusion::error::Result<Option<Arc<dyn PhysicalExpr>>> {
    let col = match in_list.expr().downcast_ref::<Column>() {
        Some(col) => col,
        None => return Ok(None),
    };
    let ff_index = check_column_index(col, schema)?;
    let ff_type = ffhelper.column(segment_ord, ff_index);

    let dict = match ff_type {
        FFType::Text(c) => c.dictionary(),
        FFType::Bytes(c) => c.dictionary(),
        // No column in this segment, so the row's value is NULL and so is any comparison
        // with it. A native evaluation would see the `Null`-typed placeholder array instead
        // and reject the comparison.
        FFType::Junk => return Ok(Some(Arc::new(Literal::new(ScalarValue::Boolean(None))))),
        _ => return Ok(None), // Not a string/bytes column. Leave for native DataFusion eval
    };

    let mut ordinals = Vec::with_capacity(in_list.list().len());

    for lit_expr in in_list.list() {
        let lit = match lit_expr.downcast_ref::<Literal>() {
            Some(lit) => lit,
            None => return Ok(None),
        };
        let bytes = match extract_bytes_from_scalar(lit.value()) {
            Some(Some(b)) => b,
            Some(None) => {
                // Push None to preserve 3VL semantics when a NULL is in the IN list
                ordinals.push(None);
                continue;
            }
            None => return Ok(None), // Early abort if non-string literal is found
        };

        // Terms absent from this segment's dictionary map to `NULL_TERM_ORDINAL` (a sentinel
        // ordinal that no non-null row in the segment can hold) rather than `None`, which
        // would falsely introduce SQL `NULL` 3VL semantics into `NOT IN`.
        let target_ord = dict
            .term_ord(bytes)
            .map_err(|e| {
                datafusion::error::DataFusionError::Execution(format!("Tantivy dict error: {}", e))
            })?
            .unwrap_or(NULL_TERM_ORDINAL);
        ordinals.push(Some(target_ord));
    }

    // Convert the raw vector directly into an Arrow array
    let array = Arc::new(UInt64Array::from(ordinals)) as Arc<dyn Array>;
    let new_col_expr = Arc::new(col.clone()) as Arc<dyn PhysicalExpr>;

    // The rewritten column carries term ordinals, so the validation schema must
    // declare it as `UInt64` to match the ordinal array we just built.
    let mut ord_fields: Vec<Field> = schema.fields().iter().map(|f| f.as_ref().clone()).collect();
    ord_fields[ff_index] = Field::new(
        schema.field(ff_index).name(),
        DataType::UInt64,
        schema.field(ff_index).is_nullable(),
    );
    let ord_schema = Schema::new(ord_fields);
    let new_in_list =
        InListExpr::try_new_from_array(new_col_expr, array, in_list.negated(), &ord_schema)
            .map_err(|e| {
                datafusion::error::DataFusionError::Execution(format!(
                    "try_new_from_array failed: {}",
                    e
                ))
            })?;

    Ok(Some(Arc::new(new_in_list)))
}

/// Rewrites `Column op Literal` to `Column(UInt64) op Literal(UInt64)` if the column is a string type.
fn rewrite_col_op_lit(
    col: &Column,
    op: &Operator,
    lit: &Literal,
    ffhelper: &FFHelper,
    segment_ord: SegmentOrdinal,
    schema: &SchemaRef,
) -> datafusion::error::Result<Option<Arc<dyn PhysicalExpr>>> {
    let ff_index = check_column_index(col, schema)?;
    let ff_type = ffhelper.column(segment_ord, ff_index);

    let bytes = match extract_bytes_from_scalar(lit.value()) {
        Some(Some(b)) => b,
        Some(None) => return Ok(Some(Arc::new(Literal::new(ScalarValue::Boolean(None))))),
        None => return Ok(None), // Not a string/bytes literal. Leave for native DataFusion eval over numerics.
    };

    let dict = match ff_type {
        FFType::Text(c) => c.dictionary(),
        FFType::Bytes(c) => c.dictionary(),
        // No column in this segment, so the row's value is NULL and so is any comparison
        // with it. A native evaluation would see the `Null`-typed placeholder array instead
        // and reject the comparison.
        FFType::Junk => return Ok(Some(Arc::new(Literal::new(ScalarValue::Boolean(None))))),
        _ => return Ok(None), // Not a string/bytes column. Leave for native DataFusion eval over numerics.
    };

    if op == &Operator::NotEq {
        let ord_opt = dict.term_ord(bytes).map_err(|e| {
            datafusion::error::DataFusionError::Execution(format!("Tantivy dict error: {}", e))
        })?;
        // If the term does not exist, all non-null values match.
        // We use NULL_TERM_ORDINAL to represent an ordinal that does not exist in the data.
        let target_ord = ord_opt.unwrap_or(NULL_TERM_ORDINAL);

        let col_expr = Arc::new(col.clone()) as Arc<dyn PhysicalExpr>;
        let lit_expr =
            Arc::new(Literal::new(ScalarValue::UInt64(Some(target_ord)))) as Arc<dyn PhysicalExpr>;
        return Ok(Some(
            Arc::new(BinaryExpr::new(col_expr, Operator::NotEq, lit_expr)) as Arc<dyn PhysicalExpr>,
        ));
    }

    // Convert string bounds to native string bounds.
    let (lower, upper) = match op {
        Operator::Lt => (Bound::Unbounded, Bound::Excluded(bytes)),
        Operator::LtEq => (Bound::Unbounded, Bound::Included(bytes)),
        Operator::Gt => (Bound::Excluded(bytes), Bound::Unbounded),
        Operator::GtEq => (Bound::Included(bytes), Bound::Unbounded),
        Operator::Eq => (Bound::Included(bytes), Bound::Included(bytes)),
        _ => return Ok(None),
    };

    // Lookup ordinal bounds.
    let (lo_ord, hi_ord) = dict.term_bounds_to_ord(lower, upper).map_err(|e| {
        datafusion::error::DataFusionError::Execution(format!("Tantivy dict error: {}", e))
    })?;

    // The Column must point to the correct index in our mock RecordBatch
    let col_expr = Arc::new(col.clone()) as Arc<dyn PhysicalExpr>;

    let mut exprs = Vec::new();
    match lo_ord {
        Bound::Included(ord) => {
            let lit_expr =
                Arc::new(Literal::new(ScalarValue::UInt64(Some(ord)))) as Arc<dyn PhysicalExpr>;
            exprs.push(
                Arc::new(BinaryExpr::new(col_expr.clone(), Operator::GtEq, lit_expr))
                    as Arc<dyn PhysicalExpr>,
            );
        }
        Bound::Excluded(ord) => {
            let lit_expr =
                Arc::new(Literal::new(ScalarValue::UInt64(Some(ord)))) as Arc<dyn PhysicalExpr>;
            exprs.push(
                Arc::new(BinaryExpr::new(col_expr.clone(), Operator::Gt, lit_expr))
                    as Arc<dyn PhysicalExpr>,
            );
        }
        Bound::Unbounded => {}
    }

    match hi_ord {
        Bound::Included(ord) => {
            let lit_expr =
                Arc::new(Literal::new(ScalarValue::UInt64(Some(ord)))) as Arc<dyn PhysicalExpr>;
            exprs.push(
                Arc::new(BinaryExpr::new(col_expr.clone(), Operator::LtEq, lit_expr))
                    as Arc<dyn PhysicalExpr>,
            );
        }
        Bound::Excluded(ord) => {
            let lit_expr =
                Arc::new(Literal::new(ScalarValue::UInt64(Some(ord)))) as Arc<dyn PhysicalExpr>;
            exprs.push(
                Arc::new(BinaryExpr::new(col_expr.clone(), Operator::Lt, lit_expr))
                    as Arc<dyn PhysicalExpr>,
            );
        }
        Bound::Unbounded => {}
    }

    if exprs.is_empty() {
        // Condition represents the entire dictionary range.
        Ok(Some(Arc::new(Literal::new(ScalarValue::Boolean(Some(
            true,
        ))))))
    } else if exprs.len() == 1 {
        Ok(Some(exprs.into_iter().next().unwrap()))
    } else {
        // Map exact bounds (lo_ord AND hi_ord) via AND
        Ok(Some(Arc::new(BinaryExpr::new(
            exprs[0].clone(),
            Operator::And,
            exprs[1].clone(),
        ))))
    }
}

/// Flips a comparison operator so that `Literal op Column` becomes `Column flipped_op Literal`.
fn flip_operator(op: &Operator) -> Option<Operator> {
    match op {
        Operator::Lt => Some(Operator::Gt),
        Operator::LtEq => Some(Operator::GtEq),
        Operator::Gt => Some(Operator::Lt),
        Operator::GtEq => Some(Operator::LtEq),
        Operator::Eq => Some(Operator::Eq),
        Operator::NotEq => Some(Operator::NotEq),
        _ => None,
    }
}

fn extract_physical_scalar_value(expr: &Arc<dyn PhysicalExpr>) -> Option<ScalarValue> {
    if let Some(lit) = expr.downcast_ref::<Literal>() {
        return Some(lit.value().clone());
    }
    None
}

fn extract_pushdown_candidates<'a>(
    expr: &'a Arc<dyn PhysicalExpr>,
    in_lists: &mut Vec<&'a Arc<dyn PhysicalExpr>>,
    hash_lookups: &mut Vec<&'a Arc<dyn PhysicalExpr>>,
) {
    if expr.is::<InListExpr>() {
        in_lists.push(expr);
    } else if expr.is::<HashTableLookupExpr>() {
        hash_lookups.push(expr);
    } else if let Some(binary) = expr.downcast_ref::<BinaryExpr>()
        && matches!(binary.op(), Operator::And)
    {
        extract_pushdown_candidates(binary.left(), in_lists, hash_lookups);
        extract_pushdown_candidates(binary.right(), in_lists, hash_lookups);
    }
}

/// Tantivy [`TermPredicate`] adapter around a single-column DataFusion [`HashTableLookupExpr`].
///
/// Evaluates the remapped `HashTableLookupExpr` over a 1-column `RecordBatch` carrying the
/// scan schema's exact Arrow `DataType` so DataFusion's type-sensitive hashing (`Utf8View`
/// inlined views vs `Utf8`, integer widths, `ArrayMap` integer offsets) matches `HashJoinExec`.
struct HashTableLookupPredicate {
    expr: Arc<dyn PhysicalExpr>,
    batch_schema: SchemaRef,
    probe_data_type: DataType,
}

impl std::fmt::Debug for HashTableLookupPredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HashTableLookupPredicate")
            .field("expr", &self.expr)
            .field("probe_data_type", &self.probe_data_type)
            .finish()
    }
}

impl HashTableLookupPredicate {
    fn evaluate_array(
        &self,
        mut array: ArrayRef,
        num_rows: usize,
        out: &mut [TinySet],
    ) -> tantivy::Result<()> {
        if array.data_type() != &self.probe_data_type {
            array = cast(&array, &self.probe_data_type).map_err(|e| {
                tantivy::TantivyError::InternalError(format!(
                    "Failed to cast predicate batch from {:?} to {:?}: {e}",
                    array.data_type(),
                    self.probe_data_type
                ))
            })?;
        }
        let options = RecordBatchOptions::new().with_row_count(Some(num_rows));
        let batch = RecordBatch::try_new_with_options(
            Arc::clone(&self.batch_schema),
            vec![array],
            &options,
        )
        .map_err(|e| {
            tantivy::TantivyError::InternalError(format!("Failed to build predicate batch: {e}"))
        })?;

        let result = self
            .expr
            .evaluate(&batch)
            .and_then(|cv| cv.into_array(num_rows))
            .map_err(|e| {
                tantivy::TantivyError::InternalError(format!(
                    "Failed to evaluate HashTableLookupExpr: {e}"
                ))
            })?;

        let bool_arr = result
            .as_any()
            .downcast_ref::<BooleanArray>()
            .ok_or_else(|| {
                tantivy::TantivyError::InternalError(
                    "HashTableLookupExpr did not produce a BooleanArray".to_string(),
                )
            })?;

        if bool_arr.null_count() == 0 {
            let chunks = bool_arr.values().bit_chunks();
            for (dst, word) in out.iter_mut().zip(chunks.iter()) {
                *dst = TinySet::deserialize(word.to_le_bytes());
            }
            if chunks.remainder_len() > 0 {
                out[chunks.iter().len()] =
                    TinySet::deserialize(chunks.remainder_bits().to_le_bytes());
            }
        } else {
            for i in 0..num_rows {
                if bool_arr.is_valid(i) && bool_arr.value(i) {
                    out[i / 64].insert_mut((i % 64) as u32);
                }
            }
        }
        Ok(())
    }
}

impl TermPredicate for HashTableLookupPredicate {
    fn filter_bytes(
        &self,
        data: &[u8],
        offsets: &[u32],
        out: &mut [TinySet],
    ) -> tantivy::Result<()> {
        let num_rows = offsets.len().saturating_sub(1);
        if num_rows == 0 {
            return Ok(());
        }
        let is_binary = matches!(
            self.probe_data_type,
            DataType::Binary | DataType::LargeBinary | DataType::BinaryView
        );
        let array: ArrayRef = if is_binary {
            Arc::new(BinaryViewArray::from_iter_values(
                (0..num_rows).map(|i| &data[offsets[i] as usize..offsets[i + 1] as usize]),
            ))
        } else {
            let mut views = Vec::with_capacity(num_rows);
            for i in 0..num_rows {
                let bytes = &data[offsets[i] as usize..offsets[i + 1] as usize];
                let s = std::str::from_utf8(bytes).map_err(|e| {
                    tantivy::TantivyError::InternalError(format!(
                        "Invalid UTF-8 in string dictionary: {e}"
                    ))
                })?;
                views.push(s);
            }
            Arc::new(StringViewArray::from_iter_values(views))
        };
        self.evaluate_array(array, num_rows, out)
    }

    fn filter_monotonic_u64s(
        &self,
        col_type: ColumnType,
        values: &[u64],
        out: &mut [TinySet],
    ) -> tantivy::Result<()> {
        let num_rows = values.len();
        if num_rows == 0 {
            return Ok(());
        }
        let array: ArrayRef = match (col_type, &self.probe_data_type) {
            (ColumnType::I64, DataType::Int8) => {
                Arc::new(Int8Array::from_iter_values(values.iter().map(|&v| {
                    <i64 as MonotonicallyMappableToU64>::from_u64(v) as i8
                })))
            }
            (ColumnType::I64, DataType::Int16) => {
                Arc::new(Int16Array::from_iter_values(values.iter().map(|&v| {
                    <i64 as MonotonicallyMappableToU64>::from_u64(v) as i16
                })))
            }
            (ColumnType::I64, DataType::Int32) => {
                Arc::new(Int32Array::from_iter_values(values.iter().map(|&v| {
                    <i64 as MonotonicallyMappableToU64>::from_u64(v) as i32
                })))
            }
            (ColumnType::I64, DataType::Timestamp(TimeUnit::Microsecond, None)) => {
                Arc::new(TimestampMicrosecondArray::from_iter_values(
                    values
                        .iter()
                        .map(|&v| <i64 as MonotonicallyMappableToU64>::from_u64(v)),
                ))
            }
            (ColumnType::I64, _) => Arc::new(Int64Array::from_iter_values(
                values
                    .iter()
                    .map(|&v| <i64 as MonotonicallyMappableToU64>::from_u64(v)),
            )),
            (ColumnType::U64, DataType::UInt8) => Arc::new(UInt8Array::from_iter_values(
                values.iter().map(|&v| v as u8),
            )),
            (ColumnType::U64, DataType::UInt16) => Arc::new(UInt16Array::from_iter_values(
                values.iter().map(|&v| v as u16),
            )),
            (ColumnType::U64, DataType::UInt32) => Arc::new(UInt32Array::from_iter_values(
                values.iter().map(|&v| v as u32),
            )),
            (ColumnType::U64, _) => Arc::new(UInt64Array::from_iter_values(values.iter().copied())),
            (ColumnType::F64, DataType::Float32) => {
                Arc::new(Float32Array::from_iter_values(values.iter().map(|&v| {
                    <f64 as MonotonicallyMappableToU64>::from_u64(v) as f32
                })))
            }
            (ColumnType::F64, _) => Arc::new(Float64Array::from_iter_values(
                values
                    .iter()
                    .map(|&v| <f64 as MonotonicallyMappableToU64>::from_u64(v)),
            )),
            (ColumnType::Bool, _) => {
                Arc::new(BooleanArray::from_iter(values.iter().map(|&v| {
                    Some(<bool as MonotonicallyMappableToU64>::from_u64(v))
                })))
            }
            (ColumnType::DateTime, _) => Arc::new(TimestampMicrosecondArray::from_iter_values(
                values.iter().map(|&v| {
                    datetime_to_pg_micros(
                        <tantivy::DateTime as MonotonicallyMappableToU64>::from_u64(v),
                    )
                }),
            )),
            (other, _) => {
                return Err(tantivy::TantivyError::InvalidArgument(format!(
                    "Unsupported ColumnType {other:?} in HashTableLookupPredicate"
                )));
            }
        };
        self.evaluate_array(array, num_rows, out)
    }
}

/// Extracts optional tightest `[lower, upper]` `Term` bounds on `col` from top-level `AND`
/// conjuncts (such as the `col >= min AND col <= max` bounds published alongside
/// `HashTableLookupExpr` by `HashJoinExec`).
fn extract_conjunct_term_bounds(
    expr: &Arc<dyn PhysicalExpr>,
    col: &Column,
    field_type: &crate::schema::SearchFieldType,
    tantivy_field: tantivy::schema::Field,
    tantivy_field_type: &FieldType,
    index_created_by_version: Option<crate::api::version::Version>,
) -> BoundsRange<Term> {
    if let FieldType::Date(opts) = tantivy_field_type
        && opts.get_precision() != DATE_TIME_PRECISION_INDEXED
    {
        return BoundsRange::new(Bound::Unbounded, Bound::Unbounded);
    }

    struct ConjunctBoundsVisitor<'a> {
        col: &'a Column,
        field_type: &'a crate::schema::SearchFieldType,
        tantivy_field: tantivy::schema::Field,
        tantivy_field_type: &'a FieldType,
        index_created_by_version: Option<crate::api::version::Version>,
        bounds: BoundsRange<Term>,
    }

    impl ConjunctBoundsVisitor<'_> {
        fn visit(&mut self, node: &Arc<dyn PhysicalExpr>) {
            let Some(binary) = node.downcast_ref::<BinaryExpr>() else {
                return;
            };
            if matches!(binary.op(), Operator::And) {
                self.visit(binary.left());
                self.visit(binary.right());
                return;
            }

            let (col, op, lit) = if let (Some(c), Some(l)) = (
                binary.left().downcast_ref::<Column>(),
                binary.right().downcast_ref::<Literal>(),
            ) {
                (c, *binary.op(), l)
            } else if let (Some(l), Some(c), Some(flipped)) = (
                binary.left().downcast_ref::<Literal>(),
                binary.right().downcast_ref::<Column>(),
                flip_operator(binary.op()),
            ) {
                (c, flipped, l)
            } else {
                return;
            };

            if col.index() != self.col.index()
                || col.name() != self.col.name()
                || lit.value().is_null()
            {
                return;
            }
            let Some(owned) = PdbOwnedValue::from_execution_scalar(lit.value(), self.field_type)
            else {
                return;
            };
            let Ok(term) = value_to_term(
                self.tantivy_field,
                &owned,
                self.tantivy_field_type,
                None,
                self.index_created_by_version,
            ) else {
                return;
            };

            let tighten_lo = |current: &mut Bound<Term>, candidate: Bound<Term>| {
                let replace = match (&*current, &candidate) {
                    (Bound::Unbounded, _) => true,
                    (Bound::Included(cur), Bound::Included(cand)) => {
                        cand.serialized_value_bytes() > cur.serialized_value_bytes()
                    }
                    (Bound::Included(cur), Bound::Excluded(cand)) => {
                        cand.serialized_value_bytes() >= cur.serialized_value_bytes()
                    }
                    (Bound::Excluded(cur), Bound::Included(cand)) => {
                        cand.serialized_value_bytes() > cur.serialized_value_bytes()
                    }
                    (Bound::Excluded(cur), Bound::Excluded(cand)) => {
                        cand.serialized_value_bytes() > cur.serialized_value_bytes()
                    }
                    (_, Bound::Unbounded) => false,
                };
                if replace {
                    *current = candidate;
                }
            };
            let tighten_hi = |current: &mut Bound<Term>, candidate: Bound<Term>| {
                let replace = match (&*current, &candidate) {
                    (Bound::Unbounded, _) => true,
                    (Bound::Included(cur), Bound::Included(cand)) => {
                        cand.serialized_value_bytes() < cur.serialized_value_bytes()
                    }
                    (Bound::Included(cur), Bound::Excluded(cand)) => {
                        cand.serialized_value_bytes() <= cur.serialized_value_bytes()
                    }
                    (Bound::Excluded(cur), Bound::Included(cand)) => {
                        cand.serialized_value_bytes() < cur.serialized_value_bytes()
                    }
                    (Bound::Excluded(cur), Bound::Excluded(cand)) => {
                        cand.serialized_value_bytes() < cur.serialized_value_bytes()
                    }
                    (_, Bound::Unbounded) => false,
                };
                if replace {
                    *current = candidate;
                }
            };

            match op {
                Operator::GtEq => tighten_lo(&mut self.bounds.lower_bound, Bound::Included(term)),
                Operator::Gt => tighten_lo(&mut self.bounds.lower_bound, Bound::Excluded(term)),
                Operator::LtEq => tighten_hi(&mut self.bounds.upper_bound, Bound::Included(term)),
                Operator::Lt => tighten_hi(&mut self.bounds.upper_bound, Bound::Excluded(term)),
                Operator::Eq => {
                    tighten_lo(&mut self.bounds.lower_bound, Bound::Included(term.clone()));
                    tighten_hi(&mut self.bounds.upper_bound, Bound::Included(term));
                }
                _ => {}
            }
        }
    }

    let mut visitor = ConjunctBoundsVisitor {
        col,
        field_type,
        tantivy_field,
        tantivy_field_type,
        index_created_by_version,
        bounds: BoundsRange::new(Bound::Unbounded, Bound::Unbounded),
    };
    visitor.visit(expr);
    visitor.bounds
}

/// Checks whether `probe_data_type`, `search_field`, and `tantivy_field_type` store identical
/// raw values in Tantivy (without lossy normalization, facet `\0` encoding, IPv6 mapping, or
/// JSON path prefixes) and returns [`ExactPostings`] when at least one of the fast field or
/// exact inverted index postings can evaluate [`HashTableLookupPredicate`].
fn resolve_hash_lookup_exact_postings(
    probe_data_type: &DataType,
    search_field: &crate::schema::SearchField,
    tantivy_field_type: &FieldType,
) -> Option<ExactPostings> {
    use crate::schema::SearchFieldType;

    let (has_exact_fast_field, exact_postings) = match tantivy_field_type {
        FieldType::I64(opts)
            if matches!(
                search_field.field_type(),
                SearchFieldType::I64(_) | SearchFieldType::Numeric64(..)
            ) && matches!(
                probe_data_type,
                DataType::Int8
                    | DataType::Int16
                    | DataType::Int32
                    | DataType::Int64
                    | DataType::Timestamp(TimeUnit::Microsecond, None)
            ) =>
        {
            (
                opts.is_fast(),
                if opts.is_indexed() {
                    ExactPostings::Available
                } else {
                    ExactPostings::Unavailable
                },
            )
        }
        FieldType::U64(opts)
            if matches!(search_field.field_type(), SearchFieldType::U64(_))
                && matches!(
                    probe_data_type,
                    DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64
                ) =>
        {
            (
                opts.is_fast(),
                if opts.is_indexed() {
                    ExactPostings::Available
                } else {
                    ExactPostings::Unavailable
                },
            )
        }
        FieldType::F64(opts)
            if matches!(search_field.field_type(), SearchFieldType::F64(_))
                && matches!(probe_data_type, DataType::Float32 | DataType::Float64) =>
        {
            (
                opts.is_fast(),
                if opts.is_indexed() {
                    ExactPostings::Available
                } else {
                    ExactPostings::Unavailable
                },
            )
        }
        FieldType::Bool(opts)
            if matches!(search_field.field_type(), SearchFieldType::Bool(_))
                && matches!(probe_data_type, DataType::Boolean) =>
        {
            (
                opts.is_fast(),
                if opts.is_indexed() {
                    ExactPostings::Available
                } else {
                    ExactPostings::Unavailable
                },
            )
        }
        FieldType::Date(opts)
            if matches!(search_field.field_type(), SearchFieldType::Date(_))
                && matches!(
                    probe_data_type,
                    DataType::Timestamp(TimeUnit::Microsecond, None)
                ) =>
        {
            (
                opts.is_fast(),
                if opts.is_indexed() && opts.get_precision() == DATE_TIME_PRECISION_INDEXED {
                    ExactPostings::Available
                } else {
                    ExactPostings::Unavailable
                },
            )
        }
        FieldType::Bytes(opts)
            if search_field.is_numeric_bytes()
                && matches!(
                    probe_data_type,
                    DataType::Binary | DataType::LargeBinary | DataType::BinaryView
                ) =>
        {
            (
                opts.is_fast(),
                if opts.is_indexed() {
                    ExactPostings::Available
                } else {
                    ExactPostings::Unavailable
                },
            )
        }
        #[allow(deprecated)]
        FieldType::Str(opts)
            if matches!(
                search_field.field_type(),
                SearchFieldType::Text(_)
                    | SearchFieldType::Tokenized(..)
                    | SearchFieldType::Uuid(_)
            ) && matches!(
                probe_data_type,
                DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View
            ) =>
        {
            // Reject normalized fast fields (e.g. `citext` or `normalizer=lowercase`):
            // `TermSetWeight` reads the `StrColumn` dictionary directly, which would
            // hash lowercased strings rather than the scan's raw values.
            if opts.is_fast() && opts.get_fast_field_tokenizer_name() != Some("raw") {
                return None;
            }
            let raw_postings = if opts.get_indexing_options().is_some() && search_field.is_keyword()
            {
                ExactPostings::Available
            } else {
                ExactPostings::Unavailable
            };
            (opts.is_fast(), raw_postings)
        }
        _ => return None,
    };

    if !has_exact_fast_field && exact_postings == ExactPostings::Unavailable {
        return None;
    }
    Some(exact_postings)
}

fn try_convert_hash_lookup_to_query(
    lookup_arc: &Arc<dyn PhysicalExpr>,
    root_expr: &Arc<dyn PhysicalExpr>,
    scan_schema: &Schema,
    schema: &crate::schema::SearchIndexSchema,
    index_created_by_version: Option<crate::api::version::Version>,
    strategy_sink: Option<Arc<std::sync::atomic::AtomicU8>>,
) -> Option<Box<dyn Query>> {
    let children = lookup_arc.children();
    if children.len() != 1 {
        // TODO: Support multi-column composite HashTableLookupExpr pushdown.
        return None;
    }

    let (col, remapped_child, is_direct_col) =
        if let Some(col) = children[0].downcast_ref::<Column>() {
            let zero_col: Arc<dyn PhysicalExpr> = Arc::new(Column::new(col.name(), 0));
            (col, zero_col, true)
        } else if let Some(cast_expr) = children[0].downcast_ref::<CastExpr>()
            && let Some(col) = cast_expr.expr().downcast_ref::<Column>()
        {
            let zero_col: Arc<dyn PhysicalExpr> = Arc::new(Column::new(col.name(), 0));
            let remapped_cast = Arc::clone(children[0])
                .with_new_children(vec![zero_col])
                .ok()?;
            (col, remapped_cast, false)
        } else {
            return None;
        };

    let scan_field = scan_schema.fields().get(col.index())?;
    if scan_field.name() != col.name()
        || is_deferred_field(scan_field)
        || matches!(
            scan_field.data_type(),
            DataType::List(_) | DataType::LargeList(_) | DataType::FixedSizeList(..)
        )
    {
        return None;
    }

    let search_field = schema.search_field(col.name())?;
    let tantivy_schema = schema.tantivy_schema();
    let tantivy_field = tantivy_schema.get_field(col.name()).ok()?;
    let tantivy_field_type = tantivy_schema.get_field_entry(tantivy_field).field_type();
    let probe_data_type = scan_field.data_type().clone();
    let exact_postings =
        resolve_hash_lookup_exact_postings(&probe_data_type, &search_field, tantivy_field_type)?;

    let remapped_lookup = Arc::clone(lookup_arc)
        .with_new_children(vec![remapped_child])
        .ok()?;
    let batch_schema = Arc::new(Schema::new(vec![Field::new(
        col.name(),
        probe_data_type.clone(),
        true,
    )]));

    let field_type = search_field.field_type();
    let bounds = if is_direct_col && search_field.stats_order_matches_values() {
        extract_conjunct_term_bounds(
            root_expr,
            col,
            &field_type,
            tantivy_field,
            tantivy_field_type,
            index_created_by_version,
        )
    } else {
        BoundsRange::new(Bound::Unbounded, Bound::Unbounded)
    };

    let cfg = TermSetStrategyConfig {
        gallop_enabled: crate::gucs::term_set_gallop_enabled(),
        bitset_max_density_unique: crate::gucs::term_set_bitset_max_density_unique(),
        bitset_max_density_multi: crate::gucs::term_set_bitset_max_density_multi(),
        strategy_sink,
        ..TermSetStrategyConfig::default()
    };

    let predicate = Arc::new(HashTableLookupPredicate {
        expr: remapped_lookup,
        batch_schema,
        probe_data_type,
    });
    let pred_query = TermPredicateQuery::new(tantivy_field, predicate)
        .with_bounds(bounds)
        .with_exact_postings(exact_postings)
        .with_strategy_config(cfg);
    Some(Box::new(ConstScoreQuery::new(pred_query, 0.0)))
}

/// Policy for join-derived `InList` predicates whose density exceeds the `BitsetFromPostings`
/// threshold and would fall back to Tantivy's `LinearScan`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InListLinearFallback {
    /// Keep the `InListExpr` as a vectorized batch-level `PreFilter` (`build_standard_stream`).
    KeepAsPreFilter,
    /// Push the `TermSetQuery` into the Tantivy reader anyway (`build_lazy_sort_stream`), so
    /// top-K collectors (`TopDocsByVectorSimilarity` / `TopDocs`) only collect join-matching
    /// documents rather than running expanding unfiltered top-K windows.
    PushToIndex,
}

/// Outcome of trying to convert one join-derived `InList` predicate.
enum InListPushdown {
    /// AND the converted term-set query into the tantivy search.
    Query(Box<dyn Query>),
    /// Not convertible (or faster as a vectorized batch pre-filter than Tantivy
    /// `LinearScan`); keep it as a batch-level pre-filter.
    Keep,
}

fn try_convert_in_list_to_query(
    in_list: &InListExpr,
    schema: &crate::schema::SearchIndexSchema,
    index_created_by_version: Option<crate::api::version::Version>,
    strategy_sink: Option<Arc<std::sync::atomic::AtomicU8>>,
    max_segment_docs: u32,
    sorted_by_field: Option<&str>,
    linear_fallback: InListLinearFallback,
) -> InListPushdown {
    if in_list.negated() {
        return InListPushdown::Keep;
    }

    let Some(col) = in_list.expr().downcast_ref::<Column>() else {
        return InListPushdown::Keep;
    };
    let Some(field) = schema.search_field(col.name()) else {
        return InListPushdown::Keep;
    };

    if field.is_text() && !field.is_keyword() {
        return InListPushdown::Keep;
    }

    let field_type = field.field_type();

    // On standard unsorted scans (`InListLinearFallback::KeepAsPreFilter`), integrating a
    // numeric `InList` into the Tantivy query only pays when the segments admit an index-driven
    // strategy. Past its density gate `TermSetWeight` falls back to `LinearScan`, a per-candidate
    // fast-field probe inside the scorer. Keeping a dense numeric `InListExpr` as a batch-level
    // `PreFilter` instead evaluates its vectorized `static_filter` hash set over the fast-field
    // batch before heap visibility (`fetch_ctids`) and non-filter column materialization.
    // Keyword text columns (`field.is_text()`), fields without a fast representation, and columns
    // the segments are sorted by all push down directly (`TermSetWeight` resolves dictionary
    // ordinals / postings or gallops the sorted column inside the search).
    // On `lazy_sort` scans (`InListLinearFallback::PushToIndex`), leaving the `InList` out of
    // the Tantivy query causes the top-K collector to fill its window with non-matching rows
    // that `PreFilter` then discards, forcing expanding top-K re-queries; always push to the
    // index so the top-K collector filters during collection.
    if linear_fallback == InListLinearFallback::KeepAsPreFilter {
        let linear_fallback_possible = field.is_fast()
            && !field.is_text()
            && !(crate::gucs::term_set_gallop_enabled() && sorted_by_field == Some(col.name()));
        let density = in_list.list().len() as f64 / max_segment_docs.max(1) as f64;
        if linear_fallback_possible && density > crate::gucs::term_set_bitset_max_density_multi() {
            return InListPushdown::Keep;
        }
    }

    let tantivy_schema = schema.tantivy_schema();
    let Ok(tantivy_field) = tantivy_schema.get_field(col.name()) else {
        return InListPushdown::Keep;
    };
    let tantivy_field_type = tantivy_schema.get_field_entry(tantivy_field).field_type();

    let terms: Option<Vec<Term>> = in_list
        .list()
        .iter()
        .filter(|expr| !extract_physical_scalar_value(expr).is_some_and(|s| s.is_null()))
        .map(|expr| {
            let scalar = extract_physical_scalar_value(expr)?;
            // Join-derived InList members are Arrow execution values. For
            // Numeric64 that means already-scaled Int64 — do not re-apply scale
            // via from_scalar (see #6158).
            let owned_value = PdbOwnedValue::from_execution_scalar(&scalar, &field_type)?;
            value_to_term(
                tantivy_field,
                &owned_value,
                tantivy_field_type,
                None,
                index_created_by_version,
            )
            .ok()
        })
        .collect();

    let Some(terms) = terms else {
        return InListPushdown::Keep;
    };
    if terms.is_empty() {
        return InListPushdown::Keep;
    }

    // Build a strategy config from the paradedb.term_set_* GUCs so the
    // dispatch thresholds (kill switch, gallop density gate, and the
    // first-column bitset density gate) can be tuned in production
    // without a recompile. Defaults mirror `TermSetStrategyConfig::default()`
    // in tantivy. `subsequent_bitset_max_density` is not exposed because
    // it gates a branch tantivy doesn't reach in production today;
    // leave it at the tantivy default via struct update syntax. The
    // optional `strategy_sink` is a per-scan AtomicU8 the planner stores
    // its decision into so EXPLAIN ANALYZE can report which strategy fired.
    let cfg = TermSetStrategyConfig {
        gallop_enabled: crate::gucs::term_set_gallop_enabled(),
        bitset_max_density_unique: crate::gucs::term_set_bitset_max_density_unique(),
        bitset_max_density_multi: crate::gucs::term_set_bitset_max_density_multi(),
        strategy_sink,
        ..TermSetStrategyConfig::default()
    };

    let term_set_query = TermSetQuery::new(terms).with_strategy_config(cfg);
    let const_score_query = ConstScoreQuery::new(term_set_query, 0.0);
    InListPushdown::Query(Box::new(const_score_query) as Box<dyn Query>)
}

/// Per-flavor strategy sinks passed into [`try_dynamic_filter_pushdown`].
pub struct DynamicFilterStrategySinks {
    inlist: Option<Arc<std::sync::atomic::AtomicU8>>,
    hash: Option<Arc<std::sync::atomic::AtomicU8>>,
}

impl DynamicFilterStrategySinks {
    pub fn new(
        inlist: Option<Arc<std::sync::atomic::AtomicU8>>,
        hash: Option<Arc<std::sync::atomic::AtomicU8>>,
    ) -> Self {
        Self { inlist, hash }
    }
}

/// Which dynamic-filter flavors were pushed down into the Tantivy query by
/// [`try_dynamic_filter_pushdown`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DynamicFilterPushdownOutcome {
    None,
    InListOnly,
    HashOnly,
    Both,
}

impl DynamicFilterPushdownOutcome {
    pub fn any_pushed(self) -> bool {
        !matches!(self, Self::None)
    }

    pub fn inlist_pushed(self) -> bool {
        matches!(self, Self::InListOnly | Self::Both)
    }

    pub fn hash_pushed(self) -> bool {
        matches!(self, Self::HashOnly | Self::Both)
    }
}

/// Try to push down `InList` and `HashTableLookupExpr` expressions from `dynamic_filters`
/// into the search query.
///
/// This is called during the first `poll_next` of the scan stream to convert eligible
/// build-side predicates from `HashJoinExec` into native Tantivy `TermSetQuery` or
/// `TermPredicateQuery` filters.
///
/// If any expressions are successfully pushed down, this function:
/// 1. Combines them into a `BooleanQuery` and `AND`s it into the `reader`'s query.
/// 2. Mutates the provided `dynamic_filters` array in-place, rewriting the DataFusion
///    expressions to replace the pushed down nodes with `lit(true)` so they are not evaluated again.
/// 3. Returns a [`DynamicFilterPushdownOutcome`] indicating which filter flavors pushed down.
pub fn try_dynamic_filter_pushdown(
    reader: &mut crate::index::reader::index::SearchIndexReader,
    scan_schema: &Schema,
    dynamic_filters: &mut [Arc<dyn PhysicalExpr>],
    sinks: DynamicFilterStrategySinks,
    linear_fallback: InListLinearFallback,
) -> DynamicFilterPushdownOutcome {
    let mut pushed_down_queries: Vec<Box<dyn Query>> = Vec::new();
    let mut pushed_down_pointers = HashSet::default();
    let mut saw_inlist_pushdown = false;
    let mut saw_hash_pushdown = false;
    let schema = reader.schema();
    let index_created_by_version = reader.index_created_by_version();
    let max_segment_docs = reader
        .segment_readers()
        .iter()
        .map(|sr| sr.num_docs())
        .max()
        .unwrap_or(0);
    let sorted_by_field = reader.sort_order().map(|s| s.field_name.to_string());

    for df in dynamic_filters {
        let Some(dynamic) = df.downcast_ref::<DynamicFilterPhysicalExpr>() else {
            continue;
        };
        let Ok(current_expr) = dynamic.current() else {
            continue;
        };

        let mut extracted_in_lists = Vec::new();
        let mut extracted_hash_lookups = Vec::new();
        extract_pushdown_candidates(
            &current_expr,
            &mut extracted_in_lists,
            &mut extracted_hash_lookups,
        );

        for in_list_arc in extracted_in_lists {
            let in_list = in_list_arc.downcast_ref::<InListExpr>().unwrap();

            match try_convert_in_list_to_query(
                in_list,
                schema,
                index_created_by_version,
                sinks.inlist.clone(),
                max_segment_docs,
                sorted_by_field.as_deref(),
                linear_fallback,
            ) {
                InListPushdown::Query(query) => {
                    pushed_down_queries.push(query);
                    pushed_down_pointers.insert(Arc::as_ptr(in_list_arc) as *const () as usize);
                    saw_inlist_pushdown = true;
                }
                InListPushdown::Keep => {}
            }
        }

        for hash_lookup_arc in extracted_hash_lookups {
            if let Some(query) = try_convert_hash_lookup_to_query(
                hash_lookup_arc,
                &current_expr,
                scan_schema,
                schema,
                index_created_by_version,
                sinks.hash.clone(),
            ) {
                pushed_down_queries.push(query);
                pushed_down_pointers.insert(Arc::as_ptr(hash_lookup_arc) as *const () as usize);
                saw_hash_pushdown = true;
            }
        }

        if !pushed_down_pointers.is_empty() {
            // Rewrite the expression to replace those nodes with lit(true)
            let rewritten = current_expr
                .clone()
                .transform_down(|node| {
                    if pushed_down_pointers.contains(&(Arc::as_ptr(&node) as *const () as usize)) {
                        Ok(Transformed::new(
                            Arc::new(Literal::new(ScalarValue::Boolean(Some(true)))),
                            true,
                            TreeNodeRecursion::Jump,
                        ))
                    } else if node.is::<InListExpr>() || node.is::<HashTableLookupExpr>() {
                        // A sibling membership node that stayed in `PreFilter` cannot contain
                        // nested membership nodes; jump past `map_children` to avoid cloning.
                        Ok(Transformed::new(node, false, TreeNodeRecursion::Jump))
                    } else {
                        Ok(Transformed::no(node))
                    }
                })
                .unwrap()
                .data;

            // Replace the DynamicFilterPhysicalExpr with the rewritten normal expression!
            *df = rewritten;
            pushed_down_pointers.clear();
        }
    }

    if !pushed_down_queries.is_empty() {
        let combined_musts: Vec<(Occur, Box<dyn Query>)> = pushed_down_queries
            .into_iter()
            .map(|q| (Occur::Must, q))
            .collect();
        let boolean_query = BooleanQuery::new(combined_musts);
        reader.inject_and_query(Box::new(boolean_query));
    }

    match (saw_inlist_pushdown, saw_hash_pushdown) {
        (false, false) => DynamicFilterPushdownOutcome::None,
        (true, false) => DynamicFilterPushdownOutcome::InListOnly,
        (false, true) => DynamicFilterPushdownOutcome::HashOnly,
        (true, true) => DynamicFilterPushdownOutcome::Both,
    }
}

#[cfg(test)]
mod tests {
    use super::try_extract_score_threshold;
    use datafusion::logical_expr::Operator;
    use datafusion::physical_expr::PhysicalExpr;
    use datafusion::physical_expr::expressions::{BinaryExpr, Column, is_not_null, is_null, lit};
    use datafusion::scalar::ScalarValue;
    use std::sync::Arc;

    const SCORE_IDX: usize = 0;
    const ID_IDX: usize = 1;

    fn score() -> Arc<dyn PhysicalExpr> {
        Arc::new(Column::new("pdb.score()", SCORE_IDX))
    }

    fn id() -> Arc<dyn PhysicalExpr> {
        Arc::new(Column::new("id", ID_IDX))
    }

    fn f32_lit(v: f32) -> Arc<dyn PhysicalExpr> {
        lit(ScalarValue::Float32(Some(v)))
    }

    fn int_lit(v: i64) -> Arc<dyn PhysicalExpr> {
        lit(ScalarValue::Int64(Some(v)))
    }

    fn bin(
        left: Arc<dyn PhysicalExpr>,
        op: Operator,
        right: Arc<dyn PhysicalExpr>,
    ) -> Arc<dyn PhysicalExpr> {
        Arc::new(BinaryExpr::new(left, op, right))
    }

    #[test]
    fn score_threshold_bare_gt_is_exact() {
        // Single-key `ORDER BY score DESC` publish. The extracted threshold is the
        // unrelaxed t: with no tiebreakers, a row tied with the cutoff can never
        // displace it, so the scorer's strict `score > t` mirrors the filter exactly.
        let expr = bin(score(), Operator::Gt, f32_lit(1.5));
        assert_eq!(
            try_extract_score_threshold(&expr, Some(SCORE_IDX)),
            Some(1.5f32)
        );
    }

    #[test]
    fn score_threshold_lexicographic_chain_with_score_leading() {
        // `ORDER BY score DESC, id ASC` publish: score > t OR (score = t AND id < v).
        // Rows tied at t may still win on the tiebreaker, so the extracted bound is
        // next_down(t): under the scorer's strict `>` semantics that keeps score >= t.
        let chain = bin(
            bin(score(), Operator::Gt, f32_lit(1.5)),
            Operator::Or,
            bin(
                bin(score(), Operator::Eq, f32_lit(1.5)),
                Operator::And,
                bin(id(), Operator::Lt, int_lit(10)),
            ),
        );
        assert_eq!(
            try_extract_score_threshold(&chain, Some(SCORE_IDX)),
            Some(1.5f32.next_down())
        );
    }

    #[test]
    fn score_threshold_nulls_first_wrapper() {
        // DESC NULLS FIRST publish: score IS NULL OR score > t. The IS NULL arm can
        // never match (every scored doc has a score), so the bound still holds.
        let expr = bin(
            is_null(score()).unwrap(),
            Operator::Or,
            bin(score(), Operator::Gt, f32_lit(1.5)),
        );
        assert_eq!(
            try_extract_score_threshold(&expr, Some(SCORE_IDX)),
            Some(1.5f32)
        );
    }

    #[test]
    fn score_threshold_conjunctive_null_guard() {
        // DESC NULLS LAST publish: score IS NOT NULL AND score > t. A conjunct's
        // bound holds for the whole conjunction, so extraction is sound.
        let expr = bin(
            is_not_null(score()).unwrap(),
            Operator::And,
            bin(score(), Operator::Gt, f32_lit(1.5)),
        );
        assert_eq!(
            try_extract_score_threshold(&expr, Some(SCORE_IDX)),
            Some(1.5f32)
        );
    }

    #[test]
    fn score_threshold_rejected_when_score_is_tiebreaker() {
        // `ORDER BY id ASC, score DESC` publish: id < v OR (id = v AND score > t).
        // The left arm admits rows of ANY score, so the expression implies no score
        // bound; extracting t here would block-prune rows that match via `id < v`.
        let chain = bin(
            bin(id(), Operator::Lt, int_lit(10)),
            Operator::Or,
            bin(
                bin(id(), Operator::Eq, int_lit(10)),
                Operator::And,
                bin(score(), Operator::Gt, f32_lit(1.5)),
            ),
        );
        assert_eq!(try_extract_score_threshold(&chain, Some(SCORE_IDX)), None);
    }

    #[test]
    fn score_threshold_rejects_foreign_shapes() {
        // Gt on a non-score column.
        let other_col = bin(id(), Operator::Gt, f32_lit(1.5));
        assert_eq!(
            try_extract_score_threshold(&other_col, Some(SCORE_IDX)),
            None
        );

        // Non-Float32 literal: the score column is always Float32, so a Float64
        // comparison (e.g. cast-normalized by an optimizer pass) must not parse.
        let f64_cmp = bin(score(), Operator::Gt, lit(ScalarValue::Float64(Some(1.5))));
        assert_eq!(try_extract_score_threshold(&f64_cmp, Some(SCORE_IDX)), None);

        // The lit(true) placeholder every dynamic filter holds before the first
        // TopK publish.
        assert_eq!(
            try_extract_score_threshold(&lit(true), Some(SCORE_IDX)),
            None
        );
    }

    #[test]
    fn collect_filters_skips_true_literals_and_supports_in_list() {
        use super::collect_filters;
        use datafusion::arrow::array::Int64Array;
        use datafusion::arrow::datatypes::{DataType, Field, Schema};
        use datafusion::physical_plan::expressions::InListExpr;

        let schema = Arc::new(Schema::new(vec![
            Field::new("pdb.score()", DataType::Float32, true),
            Field::new("id", DataType::Int64, true),
        ]));

        // Standalone `lit(true)` should produce no PreFilters.
        let mut filters = Vec::new();
        let mut threshold = None;
        collect_filters(
            &lit(true),
            &schema,
            &mut filters,
            Some(SCORE_IDX),
            &mut threshold,
        )
        .unwrap();
        assert!(filters.is_empty());

        // `lit(true) AND id IN (1, 2, 3)` should skip `lit(true)` and keep only the `InListExpr`
        // with `required_columns == [ID_IDX]`.
        let array = Arc::new(Int64Array::from(vec![1, 2, 3]));
        let in_list = Arc::new(InListExpr::try_new_from_array(id(), array, false, &schema).unwrap())
            as Arc<dyn PhysicalExpr>;
        let conj = bin(lit(true), Operator::And, in_list);
        collect_filters(
            &conj,
            &schema,
            &mut filters,
            Some(SCORE_IDX),
            &mut threshold,
        )
        .unwrap();
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].required_columns, vec![ID_IDX]);

        // An out-of-bounds physical column index must return an internal error.
        let oob_col = Arc::new(Column::new("missing", 99)) as Arc<dyn PhysicalExpr>;
        let oob_expr = bin(oob_col, Operator::Eq, int_lit(1));
        assert!(
            collect_filters(
                &oob_expr,
                &schema,
                &mut filters,
                Some(SCORE_IDX),
                &mut threshold,
            )
            .is_err()
        );
    }
}

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
//! 1. **Query-Time Pushdown (Inverted Index):** Filters that are known before the scan
//!    begins (such as `InList` predicates from a completed HashJoin build-side) are
//!    intercepted during the first `poll_next` of the scan stream. They are converted
//!    into native Tantivy queries (e.g., `TermSetQuery`) and `AND`ed into the main
//!    search query. This filters documents *while* executing the search, leveraging
//!    the inverted index for maximum performance.
//!
//! 2. **Pre-Filter Pushdown (Fast Fields):** Evolving thresholds (such as the rolling
//!    Top K threshold from `SortExec`) or filters that cannot be (or should not be, e.g.
//!    high-density `InList`s that would fall back to Tantivy `LinearScan`) mapped to the
//!    inverted index are applied as `PreFilter`s *after* the search but *before* heap
//!    visibility (`fetch_ctids`) and Arrow column materialization. These evaluate directly
//!    against Tantivy fast fields (using term-ordinal bounds/sets for strings or direct
//!    numeric comparisons).
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
//! try_dynamic_filter_pushdown()      ← Converts eligible static filters (like InList)
//!                                      into a Tantivy Query and modifies the SearchIndexReader.
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
//! `EXPLAIN (ANALYZE)` on a `PgSearchScan` node shows `rows_pruned` and
//! `rows_scanned` metrics when dynamic filters are active. `rows_pruned > 0`
//! confirms that pre-filtering is working. The `dynamic_filters=N` annotation
//! in the non-ANALYZE plan shows how many filters were pushed down.

use std::ops::Bound;
use std::sync::Arc;

use arrow_schema::SchemaRef;
use datafusion::arrow::array::{Array, ArrayRef, BooleanArray, NullArray, UInt64Array};
use datafusion::arrow::compute::cast;
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::ScalarValue;
use datafusion::common::tree_node::{Transformed, TransformedResult, TreeNode, TreeNodeRecursion};
use datafusion::logical_expr::Operator;
use datafusion::physical_expr::PhysicalExpr;
use datafusion::physical_expr::expressions::{
    BinaryExpr, Column, DynamicFilterPhysicalExpr, IsNullExpr, Literal, NotExpr,
};
use datafusion::physical_plan::expressions::InListExpr;
use datafusion::physical_plan::joins::HashTableLookupExpr;
use parking_lot::Mutex;
use tantivy::{Score, SegmentOrdinal};

use crate::api::HashSet;
use crate::index::fast_fields_helper::{FFHelper, FFType, NULL_TERM_ORDINAL};
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::query::value_to_term;
use crate::scan::deferred_encode::is_deferred_field;
use tantivy::Term;
use tantivy::query::{
    BooleanQuery, ConstScoreQuery, Occur, Query, TermQuery, TermSetQuery, TermSetStrategyConfig,
};
use tantivy::schema::IndexRecordOption;

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
///    Normal dynamic filters (`HashJoinExec`'s build-side `InListExpr`, or `SortExec` /
///    `SegmentedTopKExec`'s rolling threshold) are published once or tightened monotonically and
///    restrict every subsequent batch. By contrast, `RankJoinExec` writes a `RankLookupPhysicalExpr`
///    into a side's `DynamicFilterPhysicalExpr` immediately before polling `child_stream.next().await`
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

/// Convert a [`RankLookupPhysicalExpr`] into a zero-scoring Tantivy query (`ConstScoreQuery(..., 0.0)`)
/// suitable for `reader.and_query(...)` during a Random Access (`RA`) point lookup.
pub fn try_convert_rank_lookup_to_query(
    lookup: &RankLookupPhysicalExpr,
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
        let field = schema.search_field(col.name())?;
        if field.is_text() && !field.is_keyword() {
            return None;
        }
        let field_type = field.field_type();
        let tantivy_field = tantivy_schema.get_field(col.name()).ok()?;
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
        let field = schema.search_field(col.name())?;
        if field.is_text() && !field.is_keyword() {
            return None;
        }
        let tantivy_field = tantivy_schema.get_field(col.name()).ok()?;
        let tantivy_field_type = tantivy_schema.get_field_entry(tantivy_field).field_type();
        col_infos.push((field.field_type(), tantivy_field, tantivy_field_type));
    }

    let mut tuple_queries: Vec<(Occur, Box<dyn Query>)> = Vec::with_capacity(lookup.keys.len());
    for key_tuple in &lookup.keys {
        if key_tuple.len() != col_infos.len() {
            continue;
        }
        let mut conj: Vec<(Occur, Box<dyn Query>)> = Vec::with_capacity(col_infos.len());
        let mut valid = true;
        for (scalar, (field_type, tantivy_field, tantivy_field_type)) in
            key_tuple.iter().zip(col_infos.iter())
        {
            if scalar.is_null() {
                valid = false;
                break;
            }
            let Some(owned_value) = PdbOwnedValue::from_execution_scalar(scalar, field_type) else {
                valid = false;
                break;
            };
            let Ok(term) = value_to_term(
                *tantivy_field,
                &owned_value,
                tantivy_field_type,
                None,
                index_created_by_version,
            ) else {
                valid = false;
                break;
            };
            conj.push((
                Occur::Must,
                Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
            ));
        }
        if valid && !conj.is_empty() {
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
        let options = datafusion::arrow::record_batch::RecordBatchOptions::new()
            .with_row_count(Some(num_rows));
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
    // Ignore no-op boolean placeholders (such as unpopulated DynamicFilterPhysicalExprs,
    // `InList` nodes rewritten to `lit(true)` after inverted-index pushdown, or `lit(false)`
    // SA-exhaustion signals).
    if let Some(lit) = expr.downcast_ref::<Literal>()
        && matches!(lit.value(), ScalarValue::Boolean(Some(true | false)))
    {
        return Ok(());
    }

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
            // We only support HashTableLookupExpr for non-string columns.
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

fn extract_in_list_exprs<'a>(
    expr: &'a Arc<dyn PhysicalExpr>,
    in_lists: &mut Vec<&'a Arc<dyn PhysicalExpr>>,
) {
    if expr.is::<InListExpr>() {
        in_lists.push(expr);
    } else if let Some(binary) = expr.downcast_ref::<BinaryExpr>()
        && matches!(binary.op(), Operator::And)
    {
        extract_in_list_exprs(binary.left(), in_lists);
        extract_in_list_exprs(binary.right(), in_lists);
    }
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

    // Check GUC thresholds
    let max_size = crate::gucs::hash_join_inlist_pushdown_max_size() as usize;
    let max_distinct = crate::gucs::hash_join_inlist_pushdown_max_distinct_values() as usize;

    // K cap. Default 20,000 is bench-tuned to the win/lose crossover for
    // pushdown vs PreFilter at N=1M sorted. See
    // gucs::HASH_JOIN_INLIST_PUSHDOWN_MAX_DISTINCT_VALUES doc for the
    // reasoning and trade-off. Set to 0 to disable pushdown.
    if in_list.list().len() > max_distinct {
        return InListPushdown::Keep;
    }

    // Estimate size: this is a rough estimate based on the number of elements
    // and their typical size.
    let estimated_size = in_list.list().len() * 32; // Assume ~32 bytes per element
    if estimated_size > max_size {
        return InListPushdown::Keep;
    }

    // Integrating the term set into the Tantivy query only pays when the segments admit an
    // index-driven strategy. Past its density gate tantivy's planner falls back to `LinearScan`,
    // a per-candidate fast-field probe inside the scorer. Keeping the `InListExpr` as a
    // batch-level `PreFilter` instead evaluates its vectorized `static_filter` hash set over
    // the fast-field batch before heap visibility (`fetch_ctids`) and non-filter column
    // materialization. The gate only guards that fallback: a keyword column or one without a
    // fast representation routes to the FST automaton, and a column the segments are sorted by
    // routes to gallop, both index-driven at any density.
    // Gate on the largest segment: the big segments carry the linear cost, and a small
    // segment's linear scan is cheap even when it misses its own bitset gate. The looser
    // multi-column threshold is used because the column's docs-per-term isn't known here; a
    // unique-keyed column between the two thresholds still lands on `LinearScan`, an
    // accepted residual since either path is inexpensive in that band.
    let linear_fallback_possible = field.is_fast()
        && !field.is_text()
        && !(crate::gucs::term_set_gallop_enabled() && sorted_by_field == Some(col.name()));
    let density = in_list.list().len() as f64 / max_segment_docs.max(1) as f64;
    if linear_fallback_possible && density > crate::gucs::term_set_bitset_max_density_multi() {
        return InListPushdown::Keep;
    }

    let tantivy_schema = schema.tantivy_schema();
    let Ok(tantivy_field) = tantivy_schema.get_field(col.name()) else {
        return InListPushdown::Keep;
    };
    let tantivy_field_type = tantivy_schema.get_field_entry(tantivy_field).field_type();

    let terms: Option<Vec<Term>> = in_list
        .list()
        .iter()
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

/// Try to push down `InList` expressions from `dynamic_filters` into the search query.
///
/// This is called during the first `poll_next` of the scan stream to convert eligible `InList`
/// predicates (e.g. generated from a `HashJoin` build side) into native Tantivy `TermSet` queries.
///
/// By modifying the `SearchIndexReader` directly, this optimization allows Tantivy to filter
/// documents via its inverted index *while* executing the search, rather than filtering them
/// via fast fields after the search has returned.
///
/// If any expressions are successfully pushed down, this function:
/// 1. Combines them into a `BooleanQuery` and `AND`s it into the `reader`'s query.
/// 2. Mutates the provided `dynamic_filters` array in-place, rewriting the DataFusion
///    expressions to replace the pushed down nodes with `lit(true)` so they are not evaluated again.
/// 3. Returns `true` to indicate that a pushdown occurred.
pub fn try_dynamic_filter_pushdown(
    reader: &mut crate::index::reader::index::SearchIndexReader,
    dynamic_filters: &mut [Arc<dyn PhysicalExpr>],
    strategy_sink: Option<Arc<std::sync::atomic::AtomicU8>>,
) -> bool {
    let mut pushed_down_queries: Vec<Box<dyn Query>> = Vec::new();
    let mut pushed_down_pointers = HashSet::default();
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
        extract_in_list_exprs(&current_expr, &mut extracted_in_lists);

        for in_list_arc in extracted_in_lists {
            let in_list = in_list_arc.downcast_ref::<InListExpr>().unwrap();

            match try_convert_in_list_to_query(
                in_list,
                schema,
                index_created_by_version,
                strategy_sink.clone(),
                max_segment_docs,
                sorted_by_field.as_deref(),
            ) {
                InListPushdown::Query(query) => {
                    pushed_down_queries.push(query);
                    pushed_down_pointers.insert(Arc::as_ptr(in_list_arc) as *const () as usize);
                }
                InListPushdown::Keep => {}
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
                    } else if node.is::<InListExpr>() {
                        // A sibling `InListExpr` that returned `InListPushdown::Keep` cannot
                        // contain nested `InListExpr`s; jump past `map_children` to avoid
                        // allocating and cloning its literal list.
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

    if pushed_down_queries.is_empty() {
        false
    } else {
        let combined_musts: Vec<(Occur, Box<dyn Query>)> = pushed_down_queries
            .into_iter()
            .map(|q| (Occur::Must, q))
            .collect();
        let boolean_query = BooleanQuery::new(combined_musts);
        *reader = reader.and_query(Box::new(boolean_query));
        true
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

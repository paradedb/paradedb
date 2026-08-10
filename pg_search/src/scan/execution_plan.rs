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

//! DataFusion `ExecutionPlan` implementations for scanning `pg_search` indexes.
//!
//! See the [JoinScan README](../../postgres/customscan/joinscan/README.md) for
//! how `PgSearchScanPlan` integrates with the JoinScan physical plan and
//! dynamic filters.
//!
//! This module provides the `PgSearchScanPlan`, which scans `pg_search` index segments as a
//! single lazily-claimed partition: segments are claimed dynamically from `ParallelScanState`
//! in parallel execution, or chained end-to-end when serial.

use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use arrow_array::RecordBatch;
use arrow_schema::{SchemaRef, SortOptions};
use datafusion::common::stats::{ColumnStatistics, Precision};
use datafusion::common::{DataFusionError, Result, Statistics};
use datafusion::execution::{RecordBatchStream, SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::{EquivalenceProperties, PhysicalExpr, PhysicalSortExpr};
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::expressions::{Column, DynamicFilterPhysicalExpr};
use datafusion::physical_plan::filter_pushdown::{
    ChildPushdownResult, FilterPushdownPhase, FilterPushdownPropagation, PushedDown,
};
use datafusion::physical_plan::metrics::{
    BaselineMetrics, ExecutionPlanMetricsSet, MetricBuilder, MetricsSet, RecordOutput,
};
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, Partitioning, PlanProperties,
};
use datafusion_distributed::{
    DesiredTaskCountEvent, DesiredTaskCountEventResponse, ScaleUpLeafNodeEvent,
    ScaleUpLeafNodeEventResponse,
};
use datafusion_proto::physical_plan::{
    DefaultPhysicalExtensionCodec, PhysicalPlanDecodeContext, PhysicalProtoConverterExtension,
};
use futures::Stream;
use pgrx::pg_sys;
use tantivy::Score;

use crate::index::fast_fields_helper::FFHelper;
use crate::index::fast_fields_helper::WhichFastField;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::explain::ExplainFormat;
use crate::postgres::customscan::parallel::list_segment_ids;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::options::{SortByDirection, SortByField};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::ParallelScanState;
use crate::query::SearchQueryInput;
use crate::scan::filter_passthrough_exec::FilterPassthroughExec;
use crate::scan::late_materialization::DeferredField;
use crate::scan::pre_filter::{collect_filters, try_dynamic_filter_pushdown, PreFilter};
use crate::scan::range_partitioning::{RangePartitioning, RangePartitioningSample};
use crate::scan::Scanner;

/// A wrapper that implements Send + Sync unconditionally.
/// UNSAFE: Only use this when you guarantee single-threaded access or manual synchronization.
/// This is safe in pg_search because Postgres extensions run single-threaded.
#[derive(Clone)]
pub(crate) struct UnsafeSendSync<T>(pub T);

unsafe impl<T> Send for UnsafeSendSync<T> {}
unsafe impl<T> Sync for UnsafeSendSync<T> {}

/// Ingredients needed to construct a Scanner for deferred search.
#[derive(Clone)]
pub struct ScannerConfig {
    pub which_fast_fields: Vec<WhichFastField>,
    pub heap_relid: u32,
    pub batch_size_hint: Option<usize>,
    /// `need_scores` the index reader was opened with. Carried so a leader-dispatched worker
    /// re-opens its reader with the same scoring behavior (the reader itself can't travel).
    pub score_needed: bool,
}

/// State for a scan partition.
///
/// Uses `Arc<FFHelper>` so the same FFHelper can be shared across multiple partitions.
///
/// Lazily claims segments from `ParallelScanState`. `source_idx = Some(i)` claims from source
/// `i`'s pool for MPP sources; `None` uses the single-counter `checkout_segment_for_source(0)`
/// for basescan and non-MPP parallel joins.
#[derive(Clone)]
pub struct ScanState {
    pub source_idx: Option<usize>,
    pub planner_estimated_rows: u64,
    pub scanner_config: ScannerConfig,
    pub ffhelper: Arc<FFHelper>,
    pub visibility: Box<VisibilityChecker>,
    pub reader: SearchIndexReader,
}

/// Execution state for a single `PgSearchScanPlan`.
///
/// Under multi-partition distributed or parallel execution, a plan variant's `assigned_partition` indicates
/// which partition should consume the state and yield data. Calling `execute()` on that assigned partition
/// consumes the state exactly once and transitions it to `Consumed`.
///
/// Any calls to `execute()` for a partition that does not match `assigned_partition` will yield an
/// empty stream and leave the state untouched. This ensures each plan variant only yields data for its
/// explicitly assigned slice of the workload.
#[derive(Default, Clone)]
pub enum ExecutionState {
    /// In Shared mode, we optionally load-balance via ParallelScanState.
    /// The state is consumed by the assigned partition.
    Shared {
        parallel_state: Option<UnsafeSendSync<*mut crate::postgres::ParallelScanState>>,
        scan_state: Box<UnsafeSendSync<ScanState>>,
    },
    /// In RangePartitioned mode, we static-partition using RangePartitioning.
    /// The state is consumed by the assigned partition.
    RangePartitioned {
        range_boundaries: RangePartitioning,
        scan_state: Box<UnsafeSendSync<ScanState>>,
    },
    /// The state has been consumed by a call to execute().
    Consumed,
    /// Uninitialized placeholder state.
    #[default]
    Uninitialized,
}

/// A DataFusion `ExecutionPlan` for scanning `pg_search` index segments.
///
/// Under multi-partition parallel or distributed execution, this plan's `output_partitioning` declares
/// a *capacity* (`min(segments, target_partitions)`). The plan is then cloned into multiple variants,
/// where each variant has its `assigned_partition` explicitly set. When `execute()` is called, only
/// the assigned partition actually consumes the scan state to produce a stream, guaranteeing exactly-once
/// processing of the underlying data.
pub struct PgSearchScanPlan {
    /// State for this single-partition scan.
    ///
    /// We use a Mutex to allow taking ownership of the scanner during `execute()`.
    /// We wrap the state in `UnsafeSendSync` to satisfy `ExecutionPlan`'s `Send` + `Sync`
    /// requirements. This is safe because we are running in a single-threaded
    /// environment (Postgres), which also means that the duration for which we
    /// hold this Mutex does not impact performance.
    state: Mutex<ExecutionState>,
    /// Estimated row count, computed once at construction.
    /// Stored separately so `partition_statistics` is deterministic, even after
    /// the state has been consumed.
    planner_estimated_rows: u64,
    /// Number of segments this plan will process, derived at construction time
    /// from ParallelScanState or the reader, and kept around for EXPLAIN after
    /// the state is consumed.
    segment_count: usize,
    properties: Arc<PlanProperties>,
    resolved_query: SearchQueryInput,
    /// Dynamic filters pushed down from parent operators (e.g. Top K threshold
    /// from SortExec, join-key bounds from HashJoinExec). Each batch produced
    /// by the scanner is filtered against all of these expressions so that rows
    /// which cannot contribute to the final result are pruned early.
    dynamic_filters: Vec<Arc<dyn PhysicalExpr>>,
    /// Metrics for EXPLAIN ANALYZE.
    metrics: ExecutionPlanMetricsSet,
    deferred_fields: Vec<DeferredField>,
    /// Shared FFHelper for deferred lookup and deferred visibility.
    ///
    /// A scan may participate in late materialization, deferred visibility, or both.
    /// Callers decide whether they should use it by checking the deferred metadata,
    /// and cloning the Arc is cheap.
    ffhelper: Option<Arc<FFHelper>>,
    pub indexrelid: u32,
    /// The JoinScan source identity when visibility is deferred.
    deferred_ctid_plan_position: Option<usize>,
    /// When true, a HashJoin InList was successfully pushed down to a TermSet query.
    dynamic_filter_pushdown: Arc<AtomicBool>,
    /// Sort order preserved across `with_filter_pushdown` rebuilds so the
    /// rebuilt plan keeps its equivalence properties.
    sort_order: Option<SortByField>,
    /// Captures the per-segment `TermSetStrategy` chosen by the tantivy planner
    /// for the pushed-down dynamic filter (issue #4895). Last-segment-wins is
    /// fine because `EXPLAIN ANALYZE` only asks "did any segment use it?".
    /// Stored as `u8` so it can live behind an `AtomicU8`; round-tripped
    /// through `tantivy::query::StrategyTag` at render time. A value of
    /// `StrategyTag::None as u8` (= 0) means no `TermSetWeight` ran on
    /// this scan to write a tag — the EXPLAIN renderer falls back to
    /// `=true` in that case.
    dynamic_filter_strategy: Arc<AtomicU8>,
    range_sample: Option<RangePartitioningSample>,
    /// When executing in a multi-task distributed or parallel environment, this
    /// indicates the specific execution partition this plan variant is responsible for.
    /// It guarantees that the `ExecutionState` is consumed exactly once by the designated worker.
    pub(crate) assigned_partition: Option<usize>,
}

impl Clone for PgSearchScanPlan {
    fn clone(&self) -> Self {
        let state_guard = self.state.lock().unwrap();
        let new_state = state_guard.clone();
        Self {
            state: Mutex::new(new_state),
            planner_estimated_rows: self.planner_estimated_rows,
            segment_count: self.segment_count,
            properties: Arc::clone(&self.properties),
            resolved_query: self.resolved_query.clone(),
            dynamic_filters: self.dynamic_filters.clone(),
            metrics: self.metrics.clone(),
            deferred_fields: self.deferred_fields.clone(),
            ffhelper: self.ffhelper.clone(),
            indexrelid: self.indexrelid,
            deferred_ctid_plan_position: self.deferred_ctid_plan_position,
            dynamic_filter_pushdown: Arc::clone(&self.dynamic_filter_pushdown),
            sort_order: self.sort_order.clone(),
            dynamic_filter_strategy: Arc::clone(&self.dynamic_filter_strategy),
            range_sample: self.range_sample.clone(),
            assigned_partition: self.assigned_partition,
        }
    }
}

impl std::fmt::Debug for PgSearchScanPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgSearchScanPlan")
            .field("properties", &self.properties)
            .finish()
    }
}

impl PgSearchScanPlan {
    /// Creates a new PgSearchScanPlan with pre-opened segments.
    ///
    /// # Arguments
    ///
    /// * `state` - The pre-opened scan state (or None for tests)
    /// * `schema` - Arrow schema for the output
    /// * `resolved_query` - The filter-combined, param-solved query the readers were opened
    ///   with. Used for EXPLAIN and shipped on dispatch.
    /// * `sort_order` - Optional sort order declaration for equivalence properties
    /// * `partition_count` - Number of distributed tasks/partitions this scan natively supports
    ///   (usually `min(segments, target_partitions)`).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: Option<ScanState>,
        schema: SchemaRef,
        resolved_query: SearchQueryInput,
        sort_order: Option<&SortByField>,
        deferred_fields: Vec<DeferredField>,
        ffhelper: Option<Arc<FFHelper>>,
        indexrelid: u32,
        deferred_ctid_plan_position: Option<usize>,
        partition_count: usize,
        parallel_state: Option<*mut crate::postgres::ParallelScanState>,
        range_sample: Option<RangePartitioningSample>,
    ) -> Self {
        let needs_ffhelper = !deferred_fields.is_empty() || deferred_ctid_plan_position.is_some();
        if needs_ffhelper && ffhelper.is_none() {
            panic!("deferred lookup/visibility requires an FFHelper, but ffhelper is None");
        }
        // Output partitioning tells datafusion-distributed how many tasks this leaf can naturally split into.
        // If state is None, execute() will return an EmptyStream for this single partition.
        let range_boundaries = range_sample.as_ref().map(|s| s.build(partition_count));
        let partitioning =
            declared_partitioning(&schema, partition_count, range_boundaries.as_ref());
        let eq_properties = build_equivalence_properties(schema, sort_order);

        let properties = Arc::new(PlanProperties::new(
            eq_properties,
            partitioning,
            EmissionType::Incremental,
            Boundedness::Bounded,
        ));

        let planner_estimated_rows = state
            .as_ref()
            .map(|s| s.planner_estimated_rows)
            .unwrap_or(0);
        let segment_count = state
            .as_ref()
            .map(|s| match parallel_state {
                Some(ps) => unsafe { (*ps).source_segment_count(s.source_idx.unwrap_or(0)) },
                None => s.reader.segment_ids().len(),
            })
            .unwrap_or(0);

        if range_sample.is_none() {
            // A partition count exceeding the segment count indicates a bug in
            // pg_search_scan_desired_task_count, which should cap the tasks
            // to the segment count when range sampling is disabled.
            assert!(
                partition_count <= segment_count.max(1),
                "partition_count {} exceeds segment_count {}",
                partition_count,
                segment_count
            );
        }

        let exec_state = match state {
            Some(s) => {
                if let Some(boundaries) = range_boundaries {
                    ExecutionState::RangePartitioned {
                        range_boundaries: boundaries,
                        scan_state: Box::new(UnsafeSendSync(s)),
                    }
                } else {
                    ExecutionState::Shared {
                        parallel_state: parallel_state.map(UnsafeSendSync),
                        scan_state: Box::new(UnsafeSendSync(s)),
                    }
                }
            }
            None => ExecutionState::Uninitialized,
        };

        Self {
            state: Mutex::new(exec_state),
            planner_estimated_rows,
            segment_count,
            properties,
            resolved_query,
            dynamic_filters: Vec::new(),
            metrics: ExecutionPlanMetricsSet::new(),
            deferred_fields,
            ffhelper,
            indexrelid,
            deferred_ctid_plan_position,
            dynamic_filter_pushdown: Arc::new(AtomicBool::new(false)),
            sort_order: sort_order.cloned(),
            dynamic_filter_strategy: Arc::new(AtomicU8::new(0)),
            range_sample,
            assigned_partition: None,
        }
    }

    /// Returns a new copy of this plan resized to support exactly `target_partitions`.
    ///
    /// This allows a `TaskEstimator` or distributed planner to override the natural
    /// partition count of this plan when limited cluster resources prevent executing
    /// the original quantity.
    ///
    /// - For `Shared` mode, simply modifies the exposed metadata.
    /// - For `RangePartitioned` mode, it asks the internal `range_sample` to safely
    ///   down-sample (or up-sample) the partitioning bounds so the new partition count
    ///   has roughly uniformly distributed boundaries.
    pub fn repartition(&self, target_partitions: usize) -> Result<Arc<dyn ExecutionPlan>> {
        let state_guard = self
            .state
            .lock()
            .map_err(|e| DataFusionError::Internal(format!("lock PgSearchScanPlan state: {e}")))?;

        let new_state = match &*state_guard {
            ExecutionState::Shared {
                parallel_state,
                scan_state,
            } => ExecutionState::Shared {
                parallel_state: parallel_state.clone(),
                scan_state: scan_state.clone(),
            },
            ExecutionState::RangePartitioned { scan_state, .. } => {
                ExecutionState::RangePartitioned {
                    range_boundaries: self.range_sample.as_ref().unwrap().build(target_partitions),
                    scan_state: Box::new(UnsafeSendSync(scan_state.0.clone())),
                }
            }
            _ => {
                return Err(DataFusionError::Internal(
                    "Cannot repartition uninitialized or consumed plan".into(),
                ))
            }
        };

        let range_boundaries = match &new_state {
            ExecutionState::RangePartitioned {
                range_boundaries, ..
            } => Some(range_boundaries),
            _ => None,
        };
        let partitioning = declared_partitioning(
            self.properties.eq_properties.schema(),
            target_partitions,
            range_boundaries,
        );
        let new_properties = Arc::new(PlanProperties::new(
            self.properties.eq_properties.clone(),
            partitioning,
            self.properties.emission_type,
            self.properties.boundedness,
        ));

        Ok(self.with_overrides(new_state, new_properties, self.dynamic_filters.clone()))
    }

    fn with_overrides(
        &self,
        state: ExecutionState,
        properties: Arc<PlanProperties>,
        dynamic_filters: Vec<Arc<dyn PhysicalExpr>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(state),
            planner_estimated_rows: self.planner_estimated_rows,
            segment_count: self.segment_count,
            properties,
            resolved_query: self.resolved_query.clone(),
            dynamic_filters,
            metrics: self.metrics.clone(),
            deferred_fields: self.deferred_fields.clone(),
            ffhelper: self.ffhelper.clone(),
            indexrelid: self.indexrelid,
            deferred_ctid_plan_position: self.deferred_ctid_plan_position,
            dynamic_filter_pushdown: Arc::new(AtomicBool::new(
                self.dynamic_filter_pushdown.load(Ordering::Relaxed),
            )),
            sort_order: self.sort_order.clone(),
            dynamic_filter_strategy: Arc::new(AtomicU8::new(
                self.dynamic_filter_strategy.load(Ordering::Relaxed),
            )),
            range_sample: self.range_sample.clone(),
            assigned_partition: self.assigned_partition,
        })
    }

    /// Late-bind the shared `ParallelScanState` into this scan's execution state (#5667).
    ///
    /// Under the plan-first MPP launch the leader builds its plan before the DSM exists, so
    /// `Shared` scans start with `parallel_state: None`. Once the DSM is created and populated,
    /// the launch stamps the leader's pointer in here — before the first `execute()`, which is
    /// the only reader of the field. `RangePartitioned` and `Uninitialized` scans never consult
    /// the pointer, so they are left untouched.
    pub(crate) fn set_parallel_state(&self, ps: *mut ParallelScanState) {
        let mut state_guard = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let ExecutionState::Shared { parallel_state, .. } = &mut *state_guard {
            *parallel_state = Some(UnsafeSendSync(ps));
        }
    }

    pub fn has_deferred_fields(&self) -> bool {
        !self.deferred_fields.is_empty()
    }

    pub fn ffhelper(&self) -> Option<Arc<FFHelper>> {
        self.ffhelper.clone()
    }

    pub fn deferred_ctid_plan_position(&self) -> Option<usize> {
        self.deferred_ctid_plan_position
    }

    /// Serialize this scan into a transport-neutral descriptor for leader dispatch.
    ///
    /// Only the recipe and the reader-rebuild inputs travel; the live `ScanState` (tantivy
    /// readers, visibility checkers) is process-local and gets rebuilt on the receiving worker
    /// from its own `ParallelScanState`. `resolved_query` is the filter-combined,
    /// param-solved query the reader was opened with, so the receiver needs no `ExprContext`.
    ///
    /// Installed dynamic filters travel as proto expression nodes stamped with their
    /// `expression_id`; decoding the fragment with a deduplicating proto converter re-shares
    /// each filter's inner state with the operator that updates it (see
    /// `deserialize_physical_plan_with_runtime`).
    pub(crate) fn encode_for_dispatch(
        &self,
        proto_converter: &dyn PhysicalProtoConverterExtension,
    ) -> Result<Vec<u8>> {
        let state_guard = self
            .state
            .lock()
            .map_err(|e| DataFusionError::Internal(format!("lock PgSearchScanPlan state: {e}")))?;
        let state = match &*state_guard {
            ExecutionState::Shared { scan_state, .. } => scan_state,
            ExecutionState::RangePartitioned { scan_state, .. } => scan_state,
            _ => {
                return Err(DataFusionError::Internal(
                    "PgSearchScan dispatch: partition already consumed or uninitialized".into(),
                ));
            }
        };
        let (source_idx, planner_estimated_rows, scanner_config) = (
            state.0.source_idx,
            state.0.planner_estimated_rows,
            state.0.scanner_config.clone(),
        );

        let schema = self.properties.eq_properties.schema().clone();
        let schema_proto: datafusion_proto::protobuf::Schema =
            schema.as_ref().try_into().map_err(|e| {
                DataFusionError::Internal(format!("PgSearchScan dispatch: schema encode: {e}"))
            })?;

        // Dynamic filters self-serialize (columns/literals/comparisons only — no pg_search
        // extension exprs), so the default codec suffices.
        let codec = DefaultPhysicalExtensionCodec {};
        let dynamic_filters = self
            .dynamic_filters
            .iter()
            .map(|f| {
                let node = proto_converter.physical_expr_to_proto(f, &codec)?;
                Ok(prost::Message::encode_to_vec(&node))
            })
            .collect::<Result<Vec<_>>>()?;

        let descriptor = ScanDispatchDescriptor {
            schema_proto: prost::Message::encode_to_vec(&schema_proto),
            dynamic_filters,
            query: self.resolved_query.clone(),
            score_needed: scanner_config.score_needed,
            sort_order: self.sort_order.clone(),
            indexrelid: self.indexrelid,
            deferred_fields: self.deferred_fields.clone(),
            deferred_ctid_plan_position: self.deferred_ctid_plan_position,
            which_fast_fields: scanner_config.which_fast_fields,
            heap_relid: scanner_config.heap_relid,
            batch_size_hint: scanner_config.batch_size_hint,
            source_idx,
            planner_estimated_rows,
            partition_count: self.properties.output_partitioning().partition_count(),
            range_sample: self.range_sample.clone(),
            assigned_partition: self.assigned_partition,
        };
        serde_json::to_vec(&descriptor).map_err(|e| {
            DataFusionError::Internal(format!("PgSearchScan dispatch: serialize: {e}"))
        })
    }

    /// Rebuild a scan from a dispatch descriptor, injecting the receiving worker's runtime
    /// state. Mirrors the tail of `PgSearchTableProvider::scan_inner`: open the index reader
    /// under the worker's MVCC view, build the fast-field helper + visibility checker, and wrap
    /// a single lazy partition that claims segments at runtime from `parallel_state`.
    ///
    /// Dynamic filters decode through `proto_converter` — the fragment-wide deduplicating
    /// deserializer — so the rebuilt instances share inner state with the copies decoded
    /// inside the operators that update them (hash-join bounds, aggregate group filters).
    pub(crate) fn decode_for_dispatch(
        buf: &[u8],
        parallel_state: Option<*mut ParallelScanState>,
        expr_context: Option<*mut pg_sys::ExprContext>,
        ctx: &TaskContext,
        proto_converter: &dyn PhysicalProtoConverterExtension,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let descriptor: ScanDispatchDescriptor = serde_json::from_slice(buf).map_err(|e| {
            DataFusionError::Internal(format!("PgSearchScan dispatch: deserialize: {e}"))
        })?;

        let schema_proto = <datafusion_proto::protobuf::Schema as prost::Message>::decode(
            descriptor.schema_proto.as_slice(),
        )
        .map_err(|e| {
            DataFusionError::Internal(format!("PgSearchScan dispatch: schema decode: {e}"))
        })?;
        let schema: SchemaRef = Arc::new((&schema_proto).try_into().map_err(|e| {
            DataFusionError::Internal(format!("PgSearchScan dispatch: schema parse: {e}"))
        })?);

        let codec = DefaultPhysicalExtensionCodec {};
        let decode_ctx = PhysicalPlanDecodeContext::new(ctx, &codec);
        let dynamic_filters = descriptor
            .dynamic_filters
            .iter()
            .map(|bytes| {
                let node =
                    <datafusion_proto::protobuf::PhysicalExprNode as prost::Message>::decode(
                        bytes.as_slice(),
                    )
                    .map_err(|e| {
                        DataFusionError::Internal(format!(
                            "PgSearchScan dispatch: dynamic filter decode: {e}"
                        ))
                    })?;
                proto_converter.proto_to_physical_expr(&node, schema.as_ref(), &decode_ctx)
            })
            .collect::<Result<Vec<_>>>()?;

        let index_rel = PgSearchRelation::open(pg_sys::Oid::from(descriptor.indexrelid));
        let heap_rel = PgSearchRelation::open(pg_sys::Oid::from(descriptor.heap_relid));

        // MVCC view: an MPP source (source_idx Some) reads its per-source frozen segment
        // list from `ParallelScanState`; a standard parallel scan (source_idx None) reads
        // the worker's full segment list. Mirrors the MVCC dispatch in `scan_inner`.
        let mvcc = match (descriptor.source_idx, parallel_state) {
            (None, Some(ps)) => MvccSatisfies::ParallelWorker(unsafe { list_segment_ids(ps) }),
            (Some(idx), Some(ps)) => {
                MvccSatisfies::ParallelWorker(unsafe { (*ps).segment_ids_for_source(idx) })
            }
            (_, None) => MvccSatisfies::Snapshot,
        };

        let query = descriptor.query;
        let needs_tokenizer = query.needs_tokenizer();
        let reader = SearchIndexReader::open_with_context(
            &index_rel,
            query.clone(),
            descriptor.score_needed,
            mvcc,
            expr_context.and_then(std::ptr::NonNull::new),
            // TODO: MPP is currently disabled when a scan requires parameter solving: see
            // https://github.com/paradedb/paradedb/issues/5445.
            None,
            needs_tokenizer,
        )
        .map_err(|e| {
            DataFusionError::Internal(format!("PgSearchScan dispatch: open reader: {e}"))
        })?;

        let ffhelper = Arc::new(FFHelper::with_fields(
            &reader,
            &descriptor.which_fast_fields,
        ));
        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
        let visibility = VisibilityChecker::with_rel_and_snap(&heap_rel, snapshot);

        let scanner_config = ScannerConfig {
            which_fast_fields: descriptor.which_fast_fields,
            heap_relid: descriptor.heap_relid,
            batch_size_hint: descriptor.batch_size_hint,
            score_needed: descriptor.score_needed,
        };
        let state = ScanState {
            source_idx: descriptor.source_idx,
            planner_estimated_rows: descriptor.planner_estimated_rows,
            scanner_config,
            ffhelper: Arc::clone(&ffhelper),
            visibility: Box::new(visibility) as Box<VisibilityChecker>,
            reader,
        };

        let deferred = descriptor.deferred_fields;
        let deferred_ctid_plan_position = descriptor.deferred_ctid_plan_position;
        let ffhelper_arg = if deferred.is_empty() && deferred_ctid_plan_position.is_none() {
            None
        } else {
            Some(ffhelper)
        };

        let mut plan = PgSearchScanPlan::new(
            Some(state),
            schema,
            query,
            descriptor.sort_order.as_ref(),
            deferred,
            ffhelper_arg,
            descriptor.indexrelid,
            descriptor.deferred_ctid_plan_position,
            descriptor.partition_count,
            parallel_state,
            descriptor.range_sample,
        );
        plan.assigned_partition = descriptor.assigned_partition;
        plan.dynamic_filters = dynamic_filters;
        Ok(Arc::new(plan))
    }
}

/// Transport-neutral description of a `PgSearchScanPlan` for leader dispatch. Carries the
/// recipe plus the inputs needed to re-open the reader on the receiving worker; the live tantivy
/// state is rebuilt there from the worker's own `ParallelScanState`.
#[derive(serde::Serialize, serde::Deserialize)]
struct ScanDispatchDescriptor {
    /// Arrow schema, `datafusion_proto::protobuf::Schema`-encoded (arrow schema isn't serde).
    schema_proto: Vec<u8>,
    /// Installed dynamic filters (join-key bounds, top-k thresholds), each a prost-encoded
    /// `PhysicalExprNode`. Their `expr_id` lets a deduplicating decode re-share one instance
    /// with the operator that updates it.
    dynamic_filters: Vec<Vec<u8>>,
    query: SearchQueryInput,
    score_needed: bool,
    sort_order: Option<SortByField>,
    indexrelid: u32,
    deferred_fields: Vec<DeferredField>,
    deferred_ctid_plan_position: Option<usize>,
    which_fast_fields: Vec<WhichFastField>,
    heap_relid: u32,
    batch_size_hint: Option<usize>,
    /// `Some(i)` for an MPP source (claims from source `i`'s pool); `None` for
    /// single-counter checkout (basescan and non-MPP parallel joins). All-sources position.
    source_idx: Option<usize>,
    planner_estimated_rows: u64,
    partition_count: usize,
    range_sample: Option<RangePartitioningSample>,
    assigned_partition: Option<usize>,
}

/// The output partitioning a scan declares to DataFusion.
///
/// `Partitioning::Range` is declared only when the boundaries cover exactly
/// `partition_count` partitions and translate faithfully to DataFusion's model.
/// Otherwise `UnknownPartitioning` preserves the requested count, where any
/// partitions beyond the boundaries execute as empty streams (e.g. when the
/// sample is smaller than the requested count).
fn declared_partitioning(
    schema: &SchemaRef,
    partition_count: usize,
    range_boundaries: Option<&RangePartitioning>,
) -> Partitioning {
    if partition_count > 1 {
        if let Some(boundaries) = range_boundaries {
            if boundaries.split_points.len() + 1 == partition_count {
                if let Some(partitioning) = boundaries.to_datafusion(schema) {
                    return partitioning;
                }
            }
        }
    }
    Partitioning::UnknownPartitioning(partition_count)
}

/// Build `EquivalenceProperties` with the specified sort ordering.
///
/// If `sort_order` is `Some`, the returned properties will declare that the
/// data is sorted by the specified field in the specified direction.
/// If `sort_order` is `None`, returns empty equivalence properties.
fn build_equivalence_properties(
    schema: SchemaRef,
    sort_order: Option<&SortByField>,
) -> EquivalenceProperties {
    let mut eq_properties = EquivalenceProperties::new(schema.clone());

    if let Some(sort_field) = sort_order {
        // Find the column index for the sort field
        let field_name = sort_field.field_name.as_ref();
        if let Some((col_idx, _)) = schema.column_with_name(field_name) {
            let sort_options = SortOptions {
                descending: matches!(sort_field.direction, SortByDirection::Desc),
                // Tantivy's sort behavior:
                // - ASC: nulls sort first
                // - DESC: nulls sort last
                nulls_first: matches!(sort_field.direction, SortByDirection::Asc),
            };

            let sort_expr = PhysicalSortExpr {
                expr: Arc::new(Column::new(field_name, col_idx)),
                options: sort_options,
            };

            // Add the ordering to the equivalence properties
            eq_properties.add_ordering(std::iter::once(sort_expr));
        }
    }

    eq_properties
}

/// Translate a `tantivy::query::StrategyTag` back into the human-readable
/// strategy name surfaced in `EXPLAIN ANALYZE` output.
fn strategy_name(strategy: tantivy::query::StrategyTag) -> &'static str {
    use tantivy::query::StrategyTag;
    match strategy {
        StrategyTag::None => "none",
        StrategyTag::Gallop => "gallop",
        StrategyTag::Linear => "linear",
        StrategyTag::Bitset => "bitset_from_postings",
        StrategyTag::Automaton => "automaton",
        StrategyTag::Empty => "empty",
    }
}

impl DisplayAs for PgSearchScanPlan {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "PgSearchScan: segments={}", self.segment_count)?;
        if let Some(range_sample) = &self.range_sample {
            if let Some(assigned) = self.assigned_partition {
                let partitioning =
                    range_sample.build(self.properties.output_partitioning().partition_count());

                let lower = if assigned > 0 {
                    let val = &partitioning.split_points[assigned - 1];
                    serde_json::to_string(val).unwrap_or_else(|_| format!("{:?}", val))
                } else {
                    "-∞".to_string()
                };

                let upper = if assigned < partitioning.split_points.len() {
                    let val = &partitioning.split_points[assigned];
                    serde_json::to_string(val).unwrap_or_else(|_| format!("{:?}", val))
                } else {
                    "∞".to_string()
                };

                write!(
                    f,
                    ", partition={}[{}..{})",
                    range_sample.partition_by.as_ref(),
                    lower,
                    upper
                )?;
            } else {
                write!(f, ", partition_by={}", range_sample.partition_by.as_ref())?;
            }
        }
        if !self.dynamic_filters.is_empty() {
            write!(f, ", dynamic_filters={}", self.dynamic_filters.len())?;
        }
        if self.dynamic_filter_pushdown.load(Ordering::Relaxed) {
            // Render a single token. `TermSetWeight` writes the chosen
            // `StrategyTag` to the sink on every dispatch, so the value
            // is the strategy name (`gallop` / `linear` /
            // `bitset_from_postings` / `automaton` / `empty`). Falls
            // back to `true` only when pushdown was indicated but no
            // `TermSetWeight` ran to record a tag — e.g., the dynamic
            // filter handled a non-TermSet shape, or the scan
            // short-circuited before any segment was processed.
            let tag = self.dynamic_filter_strategy.load(Ordering::Relaxed);
            let strategy = tantivy::query::StrategyTag::try_from(tag)
                .unwrap_or(tantivy::query::StrategyTag::None);
            if matches!(strategy, tantivy::query::StrategyTag::None) {
                write!(f, ", dynamic_filter_pushdown=true")?;
            } else {
                write!(f, ", dynamic_filter_pushdown={}", strategy_name(strategy))?;
            }
        }
        write!(f, ", query={}", self.resolved_query.explain_format())
    }
}

impl ExecutionPlan for PgSearchScanPlan {
    fn name(&self) -> &str {
        "PgSearchScan"
    }

    fn properties(&self) -> &Arc<PlanProperties> {
        &self.properties
    }

    fn partition_statistics(&self, partition: Option<usize>) -> Result<Arc<Statistics>> {
        let partition_count = self.properties.output_partitioning().partition_count();
        let num_rows = match partition {
            None => Precision::Inexact(self.planner_estimated_rows as usize),
            Some(p) if p < partition_count => {
                let rows_per_partition =
                    self.planner_estimated_rows as usize / partition_count.max(1);
                Precision::Inexact(rows_per_partition)
            }
            Some(p) => {
                return Err(DataFusionError::Internal(format!(
                    "Partition {} out of range (have {} partitions)",
                    p, partition_count
                )))
            }
        };

        let column_statistics = self
            .properties
            .eq_properties
            .schema()
            .fields
            .iter()
            .map(|_| ColumnStatistics::default())
            .collect();

        Ok(Arc::new(Statistics {
            num_rows,
            total_byte_size: Precision::Absent,
            column_statistics,
        }))
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![]
    }

    fn with_new_children(
        self: Arc<Self>,
        _children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(self)
    }

    fn execute(
        &self,
        partition: usize,
        _context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        #[cfg(debug_assertions)]
        if crate::gucs::mpp_test_panic_in_worker() {
            pgrx::error!("artificial panic to test worker error propagation");
        }

        // Under DataFusion Distributed execution, upstream stage operators (such as `RepartitionExec`)
        // call `execute(p)` for all partition indices `p` in the stage's requested partition range.
        // For partitions where `p != assigned`, we return an empty stream so multi-partition
        // stage pipelines can pull and repartition data without failing.
        if let Some(assigned) = self.assigned_partition {
            if partition != assigned {
                let schema = self.properties.eq_properties.schema().clone();
                return Ok(Box::pin(unsafe {
                    UnsafeSendStream::new(futures::stream::empty(), schema)
                }));
            }
        }

        let mut state_guard = self.state.lock().map_err(|e| {
            DataFusionError::Internal(format!("Failed to lock PgSearchScanPlan state: {e}"))
        })?;

        if partition >= self.properties.output_partitioning().partition_count() {
            return Err(DataFusionError::Internal(format!(
                "Partition {} out of range (have {} partitions)",
                partition,
                self.properties.output_partitioning().partition_count()
            )));
        }

        let state = std::mem::replace(&mut *state_guard, ExecutionState::Consumed);

        // Handle state transitions for execution.
        let (scan_state, parallel_state, range_boundaries) = match state {
            ExecutionState::Shared {
                parallel_state,
                scan_state,
            } => (scan_state.0, parallel_state.map(|p| p.0), None),
            ExecutionState::RangePartitioned {
                range_boundaries,
                scan_state,
            } => {
                // If target_partitions scaled past the available sample boundaries, we
                // just return empty streams for the extra partitions.
                if partition > range_boundaries.split_points.len() {
                    let schema = self.properties.eq_properties.schema().clone();
                    return Ok(Box::pin(unsafe {
                        UnsafeSendStream::new(futures::stream::empty(), schema)
                    }));
                }

                (scan_state.0, None, Some(range_boundaries))
            }
            ExecutionState::Consumed => {
                return Err(DataFusionError::Internal(format!(
                    "PgSearchScanPlan partition {partition} executed more than once"
                )));
            }
            ExecutionState::Uninitialized => {
                let schema = self.properties.eq_properties.schema().clone();
                return Ok(Box::pin(unsafe {
                    UnsafeSendStream::new(futures::stream::empty(), schema)
                }));
            }
        };

        let ScanState {
            source_idx,
            planner_estimated_rows,
            scanner_config,
            ffhelper,
            mut visibility,
            reader,
        } = scan_state;

        let has_dynamic_filters = !self.dynamic_filters.is_empty();
        let rows_scanned = has_dynamic_filters
            .then(|| MetricBuilder::new(&self.metrics).counter("rows_scanned", partition));
        let rows_pruned = has_dynamic_filters
            .then(|| MetricBuilder::new(&self.metrics).counter("rows_pruned", partition));

        let baseline_metrics = BaselineMetrics::new(&self.metrics, partition);
        let schema = self.properties.eq_properties.schema().clone();
        let score_column_schema_idx: Option<usize> = schema
            .column_with_name(&WhichFastField::Score.name())
            .map(|(idx, _)| idx);
        let dynamic_filters = self.dynamic_filters.clone();
        let dynamic_filter_pushdown = self.dynamic_filter_pushdown.clone();
        let dynamic_filter_strategy = self.dynamic_filter_strategy.clone();

        let stream_gen = async_stream::try_stream! {
            // Create a local copy of the reader if the query changed
            let mut reader = match &range_boundaries {
                Some(rb) => reader.and_query_input(&rb.partition_bounds(partition)),
                None => reader,
            };

            // Optimized Search Integration:
            // We initialize the search here, inside the stream, because for HashJoin
            // this block is evaluated lazily during the first `poll_next`, which happens
            // AFTER the build side has completed and dynamic filters are published.
            let mut dynamic_filters = dynamic_filters.clone();
            if !dynamic_filters.is_empty()
                && try_dynamic_filter_pushdown(
                    &mut reader,
                    &mut dynamic_filters,
                    Some(dynamic_filter_strategy.clone()),
                )
            {
                dynamic_filter_pushdown.store(true, Ordering::Relaxed);
            }

            let search_results = if range_boundaries.is_some() {
                // Range partitioned mode explicitly scans all segments
                reader.search()
            } else {
                // Standard mode delegates to the parallel state if present
                match parallel_state {
                    Some(ps) => reader.search_lazy(ps, source_idx, planner_estimated_rows),
                    None => reader.search(),
                }
            };
            let mut scanner = Scanner::new(
                search_results,
                scanner_config.batch_size_hint,
                scanner_config.which_fast_fields,
                scanner_config.heap_relid,
            );
            let df_batch_size = crate::gucs::dynamic_filter_batch_size();
            if df_batch_size > 0 {
                scanner.set_batch_size(df_batch_size as usize);
            }

            loop {
                let timer = baseline_metrics.elapsed_compute().timer();
                let (pre_filters, score_threshold) = build_filters(&dynamic_filters, &schema, score_column_schema_idx);
                let pre_filters_wrapper = if pre_filters.is_empty() {
                    None
                } else {
                    Some(crate::scan::pre_filter::PreFilters {
                        filters: &pre_filters,
                        schema: &schema,
                    })
                };

                scanner.set_score_threshold(score_threshold);
                let next_batch = scanner.next(
                    &ffhelper,
                    &mut visibility,
                    pre_filters_wrapper.as_ref(),
                );
                timer.done();

                match next_batch {
                    Some(batch) => {
                        let record_batch = batch.to_record_batch(&schema);
                        yield record_batch.record_output(&baseline_metrics);
                    }
                    None => {
                        // Flush pre-materialization filter stats from Scanner.
                        if let Some(ref counter) = rows_scanned {
                            counter.add(scanner.pre_filter_rows_scanned);
                        }
                        if let Some(ref counter) = rows_pruned {
                            counter.add(scanner.pre_filter_rows_pruned);
                        }
                        break;
                    }
                }
            }
            baseline_metrics.done();
        };

        // SAFETY: pg_search operates in a single-threaded Tokio executor within Postgres,
        // so it is safe to wrap !Send types for use within DataFusion.
        let stream = unsafe {
            UnsafeSendStream::new(stream_gen, self.properties.eq_properties.schema().clone())
        };
        Ok(Box::pin(stream))
    }

    fn metrics(&self) -> Option<MetricsSet> {
        Some(self.metrics.clone_inner())
    }

    fn handle_child_pushdown_result(
        &self,
        phase: FilterPushdownPhase,
        child_pushdown_result: ChildPushdownResult,
        _config: &datafusion::common::config::ConfigOptions,
    ) -> Result<FilterPushdownPropagation<Arc<dyn ExecutionPlan>>> {
        // Only handle dynamic filters in the Post phase (Top K pushdown happens here).
        if !matches!(phase, FilterPushdownPhase::Post) {
            return Ok(FilterPushdownPropagation::if_all(child_pushdown_result));
        }

        // Collect all DynamicFilterPhysicalExpr instances from the parent filters.
        // Multiple sources may push dynamic filters (e.g. Top K from SortExec,
        // join-key bounds from HashJoinExec). We accept and apply all of them.
        //
        // The pushdown pass can potentially run more than once. Producers assume
        // pushed-down filters remain installed between passes and may not re-push
        // already-pushed filters on subsequent passes. To handle this, we merge and
        // dedupe the filter list on each pass.
        //
        // Dedupe by `expression_id`, not pointer identity: remapping a filter's
        // columns on its way down the tree (`DynamicFilterPhysicalExpr::
        // with_new_children`) mints a fresh wrapper per pass, but wrappers of the
        // same logical filter share an id. On a match, keep the incoming wrapper —
        // its column remap reflects the current tree shape.
        let mut dynamic_filters = self.dynamic_filters.clone();
        let mut filters = Vec::with_capacity(child_pushdown_result.parent_filters.len());
        let mut saw_dynamic = false;
        let mut changed = false;

        for filter_result in &child_pushdown_result.parent_filters {
            if filter_result.filter.is::<DynamicFilterPhysicalExpr>() {
                saw_dynamic = true;
                let incoming = &filter_result.filter;
                let id = incoming.expression_id();
                match dynamic_filters.iter_mut().find(|f| f.expression_id() == id) {
                    Some(slot) if !Arc::ptr_eq(incoming, slot) => {
                        *slot = Arc::clone(incoming);
                        changed = true;
                    }
                    Some(_) => {}
                    None => {
                        dynamic_filters.push(Arc::clone(incoming));
                        changed = true;
                    }
                };
                filters.push(PushedDown::Yes);
            } else {
                filters.push(filter_result.any());
            }
        }

        if changed {
            // Transfer state from the old plan to the new one.
            let state = std::mem::take(&mut *self.state.lock().map_err(|e| {
                DataFusionError::Internal(format!(
                    "Failed to lock PgSearchScanPlan state during filter pushdown: {e}"
                ))
            })?);

            let new_plan = self.with_overrides(state, self.properties.clone(), dynamic_filters);
            Ok(
                FilterPushdownPropagation::with_parent_pushdown_result(filters)
                    .with_updated_node(new_plan as Arc<dyn ExecutionPlan>),
            )
        } else if saw_dynamic {
            // Every delivered dynamic filter was already installed by an earlier
            // pass; acknowledge them without rebuilding the node.
            Ok(FilterPushdownPropagation::with_parent_pushdown_result(
                filters,
            ))
        } else {
            Ok(FilterPushdownPropagation::if_all(child_pushdown_result))
        }
    }
}

/// Evaluate the current dynamic filter expressions and convert them into
/// [`PreFilter`]s that the `Scanner` can apply before column materialization.
///
/// While doing that, we also attempt to extract a top-k score threshold if one exists.
/// We process the threshold-containing expression as we do the rest of the expressions.
/// The threshold-containing expression may be top-level, so we need to allow for
/// the rest of the expression to be applied.
///
/// This is called on every `poll_next` (or loop iteration) so that tightening thresholds (e.g.
/// from Top K) are picked up immediately.
///
/// Only filter predicates that can be lowered to fast-field or term-ordinal
/// comparisons are retained. Anything else (unsupported types, non-comparison
/// operators) is silently dropped — the parent operator is still responsible
/// for enforcing the full predicate, so correctness is not affected.
fn build_filters(
    dynamic_filters: &[Arc<dyn PhysicalExpr>],
    schema: &SchemaRef,
    score_col_schema_idx: Option<usize>,
) -> (Vec<PreFilter>, Option<Score>) {
    let mut filters = Vec::new();
    let mut score_threshold = None;
    for df in dynamic_filters {
        if let Some(dynamic) = df.downcast_ref::<DynamicFilterPhysicalExpr>() {
            if let Ok(current_expr) = dynamic.current() {
                collect_filters(
                    &current_expr,
                    schema,
                    &mut filters,
                    score_col_schema_idx,
                    &mut score_threshold,
                );
            }
        } else {
            collect_filters(
                df,
                schema,
                &mut filters,
                score_col_schema_idx,
                &mut score_threshold,
            );
        }
    }
    (filters, score_threshold)
}

/// A wrapper that unsafely implements Send for a Stream.
///
/// This is used to wrap `ScanStream` which is !Send because it contains Tantivy and Postgres
/// state that is not Send. This is safe because pg_search operates in a single-threaded
/// Tokio executor within Postgres, and these objects will never cross thread boundaries.
pub(crate) struct UnsafeSendStream<T> {
    stream: T,
    schema: SchemaRef,
}

impl<T> UnsafeSendStream<T> {
    pub(crate) unsafe fn new(stream: T, schema: SchemaRef) -> Self {
        Self { stream, schema }
    }
}

unsafe impl<T> Send for UnsafeSendStream<T> {}

impl<T: Stream> Stream for UnsafeSendStream<T> {
    type Item = T::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().stream).poll_next(cx) }
    }
}

impl<T: Stream<Item = Result<RecordBatch>>> RecordBatchStream for UnsafeSendStream<T> {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}

/// Caps a `PgSearchScanPlan`'s stage at its `partition_count` tasks.
///
/// This preserves useful scan parallelism: a single-segment scan has one task, while larger or
/// range-partitioned scans expose their partitions to the distributed planner.
pub(crate) fn pg_search_scan_desired_task_count(
    ev: DesiredTaskCountEvent,
) -> Option<datafusion::error::Result<DesiredTaskCountEventResponse>> {
    let _ = ev.plan.downcast_ref::<PgSearchScanPlan>()?;
    let partition_count = ev.plan.properties().output_partitioning().partition_count();
    // `maximum` rather than `desired`: `partition_count` is already clamped to the number
    // of physical index segments (when range sampling is disabled). A single segment cannot
    // be concurrently scanned by multiple workers, so scaling the stage past it would just
    // starve tasks with useless setup work (like building empty hash tables) for zero rows.
    Some(Ok(DesiredTaskCountEventResponse::maximum(partition_count)))
}

/// Replaces a `PgSearchScanPlan` leaf with per-task variants once its stage's task
/// count is final. One partition per task is the distributed contract: the plan is
/// repartitioned so the counts match, and each variant consumes exactly its own
/// partition; partitions past a short range sample bound empty ranges rather than
/// re-chunking tasks.
pub(crate) fn pg_search_scan_scale_up_leaf_node(
    ev: ScaleUpLeafNodeEvent,
) -> Option<datafusion::error::Result<ScaleUpLeafNodeEventResponse>> {
    let scan_plan = ev.plan.downcast_ref::<PgSearchScanPlan>()?;

    let scale = || -> datafusion::error::Result<ScaleUpLeafNodeEventResponse> {
        let current_partitions = ev.plan.properties().output_partitioning().partition_count();
        let final_plan = if ev.task_count != current_partitions {
            scan_plan.repartition(ev.task_count)?
        } else {
            Arc::clone(ev.plan)
        };

        // Downcast back because repartition returns `Arc<dyn ExecutionPlan>`
        let final_scan_plan = final_plan.downcast_ref::<PgSearchScanPlan>().unwrap();

        // Assign each task its explicit execution partition to ensure it is the only one
        // executing it.
        let variants = (0..ev.task_count)
            .map(|i| {
                let mut variant = final_scan_plan.clone();
                variant.assigned_partition = Some(i);
                Arc::new(variant) as Arc<dyn ExecutionPlan>
            })
            .collect::<Vec<_>>();

        Ok(ScaleUpLeafNodeEventResponse::new(Arc::new(
            datafusion_distributed::DistributedLeafExec::try_new(final_plan, variants)?,
        )))
    };
    Some(scale())
}

/// Stamp the leader's `ParallelScanState` pointer into every `PgSearchScanPlan` reachable from
/// `plan` (#5667).
///
/// The plan-first MPP launch builds the leader's plan before the DSM exists; this walk binds the
/// pointer afterwards, mirroring core PG's `ExecParallelInitializeDSM` (plans are address-free,
/// execution state binds late). Network boundaries expose their stage plan through `children()`,
/// so the walk reaches nested stages too — stamping worker-bound stage plans is inert: dispatch
/// encodes are context-free recipes and workers inject their own pointer at decode.
///
/// `DistributedLeafExec` needs explicit descent: its `children()` is empty, and when
/// `scale_up_leaf_node` repartitioned the scan, the wrapper's `original`/`variants` are a *new*
/// `PgSearchScanPlan` instance — the wrapper is the only live path to it. The leader executes
/// `original` (its `DistributedTaskContext` is `task_count = 1`), but stamp the variants too:
/// they are usually clones of the same instance, and a future divergence must not silently
/// un-stamp them.
pub(crate) fn stamp_parallel_state(plan: &Arc<dyn ExecutionPlan>, ps: *mut ParallelScanState) {
    visit_scan_nodes(plan, &mut |scan| scan.set_parallel_state(ps));
}

/// Visit every [`PgSearchScanPlan`] reachable from `plan`, including scans wrapped in
/// [`DistributedLeafExec`] — whose `children()` is empty, and whose `original`/`variants` may be
/// a repartitioned instance not present anywhere else in the tree. Split from
/// [`stamp_parallel_state`] so the traversal is unit-testable without a live
/// `ParallelScanState`.
///
/// [`DistributedLeafExec`]: datafusion_distributed::DistributedLeafExec
fn visit_scan_nodes(plan: &Arc<dyn ExecutionPlan>, visit: &mut impl FnMut(&PgSearchScanPlan)) {
    if let Some(scan) = plan.downcast_ref::<PgSearchScanPlan>() {
        visit(scan);
    }
    if let Some(leaf) = plan.downcast_ref::<datafusion_distributed::DistributedLeafExec>() {
        visit_scan_nodes(leaf.original(), visit);
        for variant in leaf.variants() {
            visit_scan_nodes(variant, visit);
        }
        return;
    }
    // `FilterPassthroughExec::children()` forwards to its inner node's children, skipping the
    // inner node itself. Today it only ever wraps `SortPreservingMergeExec` (never a scan), so
    // the plain walk would still reach every scan — but that invariant lives in
    // `segmented_topk_rule`, not here. Descend through `inner()` explicitly so a future
    // wrapping of a scan cannot silently escape the stamp.
    if let Some(fp) = plan.downcast_ref::<FilterPassthroughExec>() {
        visit_scan_nodes(fp.inner(), visit);
        return;
    }
    for child in plan.children() {
        visit_scan_nodes(child, visit);
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use std::sync::Arc;

    use arrow_schema::{Schema, SchemaRef};
    use pgrx::prelude::*;

    use crate::query::SearchQueryInput;

    use super::PgSearchScanPlan;

    fn empty_schema() -> SchemaRef {
        Arc::new(Schema::empty())
    }

    /// #5667: `DistributedLeafExec::children()` is empty, and after `repartition()` its
    /// `original`/`variants` can be a scan instance not present anywhere else in the tree — so
    /// a `children()`-only walk silently misses it and the leader would execute an unstamped
    /// scan. This pins the explicit descent.
    #[pg_test]
    fn visit_scan_nodes_descends_through_distributed_leaf_exec() {
        use datafusion::physical_plan::ExecutionPlan;
        use datafusion_distributed::DistributedLeafExec;

        fn make_scan() -> Arc<dyn ExecutionPlan> {
            Arc::new(PgSearchScanPlan::new(
                None,
                empty_schema(),
                SearchQueryInput::All,
                None,
                Vec::new(),
                None,
                0,
                None,
                1,
                None,
                None,
            ))
        }

        // Distinct instances on purpose: `repartition()` gives the wrapper a scan that exists
        // nowhere else in the tree, so the visitor must reach `original` and each variant
        // independently — not merely alias the same node twice.
        let leaf: Arc<dyn ExecutionPlan> = Arc::new(
            DistributedLeafExec::try_new(make_scan(), [make_scan()]).expect("leaf construction"),
        );

        let mut visited = 0usize;
        super::visit_scan_nodes(&leaf, &mut |_| visited += 1);
        assert_eq!(
            visited, 2,
            "visit_scan_nodes must descend through DistributedLeafExec \
             (original + 1 distinct variant); its children() is empty"
        );
    }

    #[pg_test]
    #[should_panic(expected = "deferred lookup/visibility requires an FFHelper")]
    fn deferred_visibility_requires_ffhelper() {
        let _ = PgSearchScanPlan::new(
            None,
            empty_schema(),
            SearchQueryInput::All,
            None,
            Vec::new(),
            None,
            0,
            Some(1),
            1,
            None,
            None,
        );
    }

    #[pg_test]
    fn can_construct_plan() {
        let _ = PgSearchScanPlan::new(
            None,
            empty_schema(),
            SearchQueryInput::All,
            None,
            Vec::new(),
            None,
            0,
            None,
            1,
            None,
            None,
        );
    }
}

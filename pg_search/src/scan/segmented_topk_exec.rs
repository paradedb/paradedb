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

//! Per-segment Top K with global threshold pruning.
//!
//! See the [JoinScan README](../../postgres/customscan/joinscan/README.md) for
//! how this node fits into the overall physical plan and pruning pipeline.
//!
//! `SegmentedTopKExec` sits between `TantivyLookupExec` and its child in the
//! physical plan. It operates on the 2-way deferred `UnionArray` columns emitted
//! by late materialization:
//!   - State 0 (doc_address): unpacks `(segment_ord, doc_id)` and bulk-fetches
//!     term ordinals via `FFHelper`.
//!   - State 1 (term_ordinals): uses ordinals directly (already resolved by
//!     pre-filter memoization).
//!
//! For States 0 and 1, a per-segment Vec-based buffer (capacity 2×K) with
//! QuickSelect retains only the top K rows per segment. All batches are
//! collected during the input phase, and survivors are emitted in a single
//! pass once all input is consumed.
//!
//! ## Global threshold
//!
//! As rows are ingested, a global threshold is published to the scanner.
//! Once a segment's buffer undergoes its first QuickSelect (accumulating 2×K
//! rows), the K-th element's deferred ordinals are converted back to strings
//! via `FFHelper::ord_to_str` and published as a `DynamicFilterPhysicalExpr`.
//! DataFusion's standard filter pushdown mechanism routes this to
//! `PgSearchScanPlan`, where `pre_filter::try_rewrite_binary` translates
//! the string literals to per-segment ordinal bounds automatically.
//!
//! ## Output bound
//!
//! The cutoff for each segment is the worst (K-th best) `OwnedRow` in that
//! segment's heap. All rows with `OwnedRow <= cutoff` survive. When sort keys
//! are unique, this is exactly K rows per segment. With ties at the boundary,
//! all tied rows are conservatively retained:
//!
//!   survivors_s = K + (T_s - H_s)
//!
//! where `T_s` is the total number of rows in segment `s` sharing the cutoff
//! value, and `H_s` is how many of those occupy heap slots (`H_s >= 1`).
//! Total ordinal-comparable rows reaching `TantivyLookupExec`:
//!
//!   sum_s(survivors_s) <= K * S  (when no boundary ties)
//!
//! where `S` is the number of segments. Pass-through rows (NULL
//! ordinals) are emitted immediately and are not bounded by K.
//!
//! **Compound sorts:** every sort column is used, not just the primary. The
//! per-segment buffer keys on the full compound `OwnedRow`, and the global
//! threshold is published as a lexicographic filter over all sort exprs — each
//! deferred column resolved via `ord_to_str`, non-deferred columns read
//! directly — so `ORDER BY val DESC, id ASC LIMIT 25` breaks ties on `id`
//! rather than retaining every row that shares the boundary `val`. Only rows
//! tied across the *entire* sort key are conservatively retained, per the
//! `survivors_s` bound above.

use crate::api::HashMap;
use crate::index::fast_fields_helper::{CanonicalColumn, FFHelper, FFType, NULL_TERM_ORDINAL};
use crate::postgres::customscan::joinscan::build::CtidColumn;
use crate::postgres::customscan::joinscan::visibility_filter::{
    materialize_deferred_ctid, DeferredCtidMaterializationState,
};
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::scan::deferred_encode::unpack_doc_address;
use crate::scan::execution_plan::UnsafeSendStream;
use crate::scan::tantivy_lookup_exec::{open_rebuilt_ffhelper, rebuild_mvcc, LookupRebuildContext};
use arrow_array::{
    Array, ArrayRef, BooleanArray, RecordBatch, StructArray, UInt32Array, UInt64Array, UnionArray,
};
use arrow_schema::SchemaRef;
use arrow_select::concat::concat_batches;
use arrow_select::filter::filter_record_batch;
use datafusion::arrow::row::{OwnedRow, RowConverter, SortField};
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::{SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::expressions::DynamicFilterPhysicalExpr;
use datafusion::physical_expr::{EquivalenceProperties, LexOrdering, PhysicalExpr};
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::filter_pushdown::{
    ChildFilterDescription, ChildPushdownResult, FilterDescription, FilterPushdownPhase,
    FilterPushdownPropagation,
};
use datafusion::physical_plan::metrics::{
    Count, ExecutionPlanMetricsSet, MetricBuilder, MetricsSet,
};
use datafusion::physical_plan::{DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties};
use pgrx::pg_sys;
use std::sync::{Arc, Mutex};
use tantivy::termdict::TermOrdinal;
use tantivy::{DocId, SegmentOrdinal};

/// Minimum per-segment buffer capacity on visibility plans. Visibility checking has a
/// per-batch setup cost (snapshot acquisition, packed-DocAddress → ctid resolution, HOT
/// chain search), so running it on tiny batches is wasteful.
///
/// On visibility plans a segment's buffer capacity is
/// `max(2 * K, MINIMUM_VISIBILITY_CHECK_SIZE)`: [`SegmentedTopKState::truncate_top_k`]
/// runs only once the buffer fills, so every visibility pass covers a reasonably sized
/// batch and no segment is checked on a tiny batch. Non-visibility plans use a plain
/// `2 * K` capacity (no visibility pass, so no reason to delay the QuickSelect).
///
/// Aligned with `DEFERRED_BATCH_SIZE` (the deferred-field scan batch size), since this
/// path batches deferred rows. Tunable via benchmarking.
const MINIMUM_VISIBILITY_CHECK_SIZE: usize = 8192;

/// The serializable "recipe" for rebuilding [`AbsorbedVisibilityData`] on a dispatched
/// worker: `(plan_position, heap OID)` pairs plus table names. `None` when the plan carries
/// no absorbed visibility./// The live ctid resolvers (FFHelpers) are not part of the recipe;
/// they are re-wired from the decoded subtree in `decode_for_dispatch`,
/// or rebuilt if the scan sits behind a network boundary.
type VisibilityRecipe = Option<(Vec<(usize, pg_sys::Oid)>, Vec<String>, Vec<(usize, u32)>)>;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DeferredSortColumn {
    pub sort_col_idx: usize,
    pub canonical: CanonicalColumn,
    /// How a dispatched fragment rebuilds the fast-field helper when the column's scan is not
    /// in its decoded subtree (the top-k above a network boundary).
    #[serde(default)]
    pub rebuild: Option<crate::scan::late_materialization::DeferredLookupRebuild>,
}

/// One wired ctid resolver: the index it reads and the fast-field helper over its segments.
type CtidResolver = (u32, Arc<FFHelper>);

/// Visibility data absorbed from a `VisibilityFilterExec` during the `SegmentedTopKRule`
/// optimization pass.
///
/// When `SegmentedTopKExec` absorbs a `VisibilityFilterExec` that was its direct child,
/// it takes ownership of the plan_position/OID pairs and ctid resolvers so it can
/// perform MVCC visibility checks inline, right after each prune cycle and at final
/// emission, instead of deferring them to a separate downstream node.
pub struct AbsorbedVisibilityData {
    plan_pos_oids: Vec<(usize, pg_sys::Oid)>,
    table_names: Vec<String>,
    /// Per-plan_position FFHelpers for resolving packed DocAddresses to real ctids.
    /// Wired by `VisibilityCtidResolverRule` after plan construction.
    ctid_resolvers: Mutex<Vec<Option<CtidResolver>>>,
}

impl std::fmt::Debug for AbsorbedVisibilityData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AbsorbedVisibilityData")
            .field("plan_pos_oids", &self.plan_pos_oids)
            .field("table_names", &self.table_names)
            .finish_non_exhaustive()
    }
}

impl AbsorbedVisibilityData {
    pub fn new(plan_pos_oids: Vec<(usize, pg_sys::Oid)>, table_names: Vec<String>) -> Self {
        let resolver_len = plan_pos_oids
            .iter()
            .map(|(p, _)| *p)
            .max()
            .map_or(0, |m| m + 1);
        Self {
            plan_pos_oids,
            table_names,
            ctid_resolvers: Mutex::new(vec![None; resolver_len]),
        }
    }

    pub fn plan_pos_oids(&self) -> &[(usize, pg_sys::Oid)] {
        &self.plan_pos_oids
    }

    pub fn set_ctid_resolver(&self, plan_pos: usize, indexrelid: u32, ffhelper: Arc<FFHelper>) {
        let mut resolvers = self
            .ctid_resolvers
            .lock()
            .expect("AbsorbedVisibilityData ctid_resolvers lock poisoned");
        if plan_pos >= resolvers.len() {
            resolvers.resize(plan_pos + 1, None);
        }
        resolvers[plan_pos] = Some((indexrelid, ffhelper));
    }
}

pub struct SegmentedTopKExec {
    input: Arc<dyn ExecutionPlan>,
    /// The sort expressions defining the Top K order.
    sort_exprs: LexOrdering,
    /// The deferred string/bytes columns that are part of the Top K order.
    deferred_columns: Vec<DeferredSortColumn>,
    /// FFHelper for Tantivy fast field access (shared with TantivyLookupExec).
    ffhelper: Arc<FFHelper>,
    /// Maximum rows to keep per segment.
    k: usize,
    /// Dynamic filter pushed down through DataFusion's standard filter pushdown
    /// mechanism. Updated at runtime with a global threshold (materialized
    /// string literals) that the scanner's `try_rewrite_binary` translates to
    /// per-segment ordinal bounds.
    dynamic_filter: Arc<DynamicFilterPhysicalExpr>,
    /// Visibility data absorbed from a `VisibilityFilterExec` during plan optimization.
    /// Present when VFExec was the direct child of `TantivyLookupExec` (e.g. for inner
    /// joins or the preserved sides of outer/semi/anti joins). When present, this node
    /// owns MVCC visibility checking.
    visibility_data: Option<Arc<AbsorbedVisibilityData>>,
    properties: Arc<PlanProperties>,
    metrics: ExecutionPlanMetricsSet,
}

impl std::fmt::Debug for SegmentedTopKExec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sort_exprs_str = self
            .sort_exprs
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        f.debug_struct("SegmentedTopKExec")
            .field("expr", &sort_exprs_str)
            .field("k", &self.k)
            .field("deferred_columns", &self.deferred_columns)
            .finish()
    }
}

impl SegmentedTopKExec {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        input: Arc<dyn ExecutionPlan>,
        sort_exprs: LexOrdering,
        deferred_columns: Vec<DeferredSortColumn>,
        ffhelper: Arc<FFHelper>,
        k: usize,
        visibility_data: Option<Arc<AbsorbedVisibilityData>>,
    ) -> Self {
        use datafusion::physical_expr::expressions::lit;

        let mut eq_props = EquivalenceProperties::new(input.schema());
        eq_props.add_ordering(sort_exprs.clone());
        let properties = Arc::new(PlanProperties::new(
            eq_props,
            input.properties().output_partitioning().clone(),
            EmissionType::Final,
            Boundedness::Bounded,
        ));

        // Create a DynamicFilterPhysicalExpr with the sort expression columns
        // as children. The initial expression is `lit(true)` (no filtering).
        // At runtime, `update()` replaces this with the global threshold.
        let children: Vec<Arc<dyn PhysicalExpr>> =
            sort_exprs.iter().map(|e| Arc::clone(&e.expr)).collect();
        let dynamic_filter = Arc::new(DynamicFilterPhysicalExpr::new(children, lit(true)));

        Self {
            input,
            sort_exprs,
            deferred_columns,
            ffhelper,
            k,
            dynamic_filter,
            visibility_data,
            properties,
            metrics: ExecutionPlanMetricsSet::new(),
        }
    }

    /// Returns the `(plan_position, heap_oid)` pairs whose ctid columns need
    /// visibility checking. Empty when no `VisibilityFilterExec` was absorbed.
    pub fn plan_pos_oids(&self) -> &[(usize, pg_sys::Oid)] {
        self.visibility_data
            .as_deref()
            .map(AbsorbedVisibilityData::plan_pos_oids)
            .unwrap_or(&[])
    }

    /// Wire an FFHelper for resolving packed DocAddresses to real ctids for
    /// the given plan_position. Called by `VisibilityCtidResolverRule`.
    pub fn set_ctid_resolver(&self, plan_pos: usize, indexrelid: u32, ffhelper: Arc<FFHelper>) {
        if let Some(vd) = &self.visibility_data {
            vd.set_ctid_resolver(plan_pos, indexrelid, ffhelper);
        }
    }

    fn create_mat_row_converter(
        sort_exprs: &LexOrdering,
        deferred_columns: &[DeferredSortColumn],
        ffhelper: &FFHelper,
        schema: &arrow_schema::Schema,
    ) -> Result<RowConverter> {
        let materialized_sort_fields: Vec<SortField> = sort_exprs
            .iter()
            .map(|expr| {
                let is_deferred = expr
                    .expr
                    .downcast_ref::<datafusion::physical_expr::expressions::Column>()
                    .and_then(|c| {
                        deferred_columns
                            .iter()
                            .find(|d| d.sort_col_idx == c.index())
                    });
                let data_type = if let Some(deferred) = is_deferred {
                    let col = ffhelper.column(0, deferred.canonical.ff_index);
                    match col {
                        FFType::Bytes(_) => arrow_schema::DataType::BinaryView,
                        _ => arrow_schema::DataType::Utf8View,
                    }
                } else {
                    expr.expr
                        .data_type(schema)
                        .unwrap_or(arrow_schema::DataType::Utf8View)
                };
                SortField::new_with_options(data_type, expr.options)
            })
            .collect();

        Ok(RowConverter::new(materialized_sort_fields)?)
    }

    /// Serialize for leader dispatch. The `ffhelper` is live and doesn't travel; the worker
    /// pulls it from the scan in its decoded subtree. The `dynamic_filter` is internal and is
    /// recreated fresh by `new` on decode; the leader-side `FilterPushdown` wiring of that filter
    /// into the scans' `PreFilter` does not travel, so a dispatched fragment scans without the
    /// runtime top-k pruning (correct, just slower). Rewiring on decode is an open follow-up.
    /// `decoders`/`properties` are derived.
    pub(crate) fn encode_for_dispatch(&self) -> Result<Vec<u8>> {
        let codec = datafusion_proto::physical_plan::DefaultPhysicalExtensionCodec {};
        let proto_conv = datafusion_proto::physical_plan::DefaultPhysicalProtoConverter {};
        let sort_proto = datafusion_proto::physical_plan::to_proto::serialize_physical_sort_exprs(
            self.sort_exprs.iter().cloned(),
            &codec,
            &proto_conv,
        )?;
        let sort_bytes: Vec<Vec<u8>> = sort_proto
            .iter()
            .map(prost::Message::encode_to_vec)
            .collect();
        // Ship the visibility "recipe" (serializable plan_position/heap-OID pairs + table
        // names) so a dispatched worker can rebuild AbsorbedVisibilityData. The live ctid
        // resolvers (FFHelpers) don't travel; they are re-collected from the decoded
        // subtree in decode_for_dispatch, mirroring how VisibilityFilterExec dispatches.
        // Without this, a worker would decode visibility_data: None and silently skip
        // visibility (returning dead rows), since absorption already removed the VFExec on
        // the leader and workers do not re-run the optimizer rule.
        let visibility_recipe: VisibilityRecipe = self.visibility_data.as_ref().map(|vd| {
            let resolver_indexes: Vec<(usize, u32)> = vd
                .ctid_resolvers
                .lock()
                .expect("ctid_resolvers lock poisoned")
                .iter()
                .enumerate()
                .filter_map(|(pos, r)| r.as_ref().map(|(relid, _)| (pos, *relid)))
                .collect();
            (
                vd.plan_pos_oids.clone(),
                vd.table_names.clone(),
                resolver_indexes,
            )
        });
        let payload = (
            sort_bytes,
            self.deferred_columns.clone(),
            self.k,
            visibility_recipe,
        );
        serde_json::to_vec(&payload).map_err(|e| {
            DataFusionError::Internal(format!("SegmentedTopKExec dispatch: serialize: {e}"))
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn decode_for_dispatch(
        buf: &[u8],
        input: Arc<dyn ExecutionPlan>,
        ffhelpers: HashMap<u32, Arc<FFHelper>>,
        ctid_resolvers: Vec<(usize, u32, Arc<FFHelper>)>,
        ctx: &TaskContext,
        index_segment_ids: &[crate::api::HashSet<tantivy::index::SegmentId>],
        parallel_state: Option<*mut crate::postgres::ParallelScanState>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let (sort_bytes, deferred_columns, k, visibility_recipe): (
            Vec<Vec<u8>>,
            Vec<DeferredSortColumn>,
            usize,
            VisibilityRecipe,
        ) = serde_json::from_slice(buf).map_err(|e| {
            DataFusionError::Internal(format!("SegmentedTopKExec dispatch: deserialize: {e}"))
        })?;
        // The deferred sort columns all resolve against one index (the sorted relation), and
        // `ff_index` is relative to that index's fast-field list. A join leaves the other
        // index's scan in the same subtree, so pick the helper by `indexrelid` instead of
        // grabbing whichever scan comes first. When that scan is behind a network boundary
        // (no helper in the subtree), rebuild one over the same segment view the scan's
        // reader opens, so segment ordering matches the ordinals the producers packed.
        let ffhelper = match deferred_columns.first() {
            Some(first) => match ffhelpers.get(&first.canonical.indexrelid).cloned() {
                Some(helper) => helper,
                None => {
                    let entries: Vec<_> = deferred_columns
                        .iter()
                        .filter_map(|d| d.rebuild.as_ref().map(|rb| (d.canonical.ff_index, rb)))
                        .collect();
                    let (_, first_rb) = entries.first().ok_or_else(|| {
                        DataFusionError::Internal(format!(
                            "SegmentedTopKExec dispatch: no ffhelper for indexrelid {} and no \
                             rebuild info",
                            first.canonical.indexrelid
                        ))
                    })?;
                    let mvcc = rebuild_mvcc(LookupRebuildContext { parallel_state }, first_rb)?;
                    open_rebuilt_ffhelper(first.canonical.indexrelid, &entries, mvcc)?
                }
            },
            None => ffhelpers
                .into_values()
                .next()
                .unwrap_or_else(|| Arc::new(FFHelper::empty())),
        };
        let sort_proto = sort_bytes
            .iter()
            .map(|b| {
                <datafusion_proto::protobuf::PhysicalSortExprNode as prost::Message>::decode(
                    b.as_slice(),
                )
            })
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| {
                DataFusionError::Internal(format!("SegmentedTopKExec dispatch: sort decode: {e}"))
            })?;
        let codec = datafusion_proto::physical_plan::DefaultPhysicalExtensionCodec {};
        let proto_conv = datafusion_proto::physical_plan::DefaultPhysicalProtoConverter {};
        let decode_ctx =
            datafusion_proto::physical_plan::PhysicalPlanDecodeContext::new(ctx, &codec);
        let input_schema = input.schema();
        let exprs = datafusion_proto::physical_plan::from_proto::parse_physical_sort_exprs(
            &sort_proto,
            &decode_ctx,
            input_schema.as_ref(),
            &proto_conv,
        )?;
        let sort_exprs = LexOrdering::new(exprs).ok_or_else(|| {
            DataFusionError::Internal("SegmentedTopKExec dispatch: empty sort order".into())
        })?;
        // Rebuild absorbed visibility data from the shipped recipe and re-wire the live
        // ctid resolvers pulled from the decoded subtree. Workers do not re-run the
        // optimizer rule, so the VFExec is already absorbed and gone; leaving this `None`
        // would make the worker skip visibility and return dead rows. The VisibilityChecker
        // itself is built later at execute() time from GetActiveSnapshot(), so no snapshot
        // travels (the worker's active snapshot is already the leader's MVCC view).
        let visibility_data = match visibility_recipe {
            Some((plan_pos_oids, table_names, resolver_indexes)) => {
                let vd = AbsorbedVisibilityData::new(plan_pos_oids, table_names);
                for (plan_pos, indexrelid, resolver) in &ctid_resolvers {
                    vd.set_ctid_resolver(*plan_pos, *indexrelid, Arc::clone(resolver));
                }
                for (plan_pos, indexrelid) in resolver_indexes {
                    if ctid_resolvers.iter().any(|(pos, _, _)| *pos == plan_pos) {
                        continue;
                    }
                    let ids = index_segment_ids.get(plan_pos).cloned().ok_or_else(|| {
                        DataFusionError::Internal(format!(
                            "SegmentedTopKExec dispatch: missing canonical segment ids for \
                             plan_position {plan_pos}"
                        ))
                    })?;
                    let ffhelper = crate::scan::tantivy_lookup_exec::open_rebuilt_ffhelper(
                        indexrelid,
                        &[],
                        crate::index::mvcc::MvccSatisfies::ParallelWorker(ids),
                    )?;
                    vd.set_ctid_resolver(plan_pos, indexrelid, ffhelper);
                }
                Some(Arc::new(vd))
            }
            None => None,
        };
        Ok(Arc::new(SegmentedTopKExec::new(
            input,
            sort_exprs,
            deferred_columns,
            ffhelper,
            k,
            visibility_data,
        )))
    }
}

impl DisplayAs for SegmentedTopKExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let sort_exprs_str = self
            .sort_exprs
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(
            f,
            "SegmentedTopKExec: expr=[{}], k={}",
            sort_exprs_str, self.k
        )?;
        // Signal in the plan that this node performs MVCC visibility checking (it has
        // absorbed a VisibilityFilterExec), and for which tables, so it is visible when
        // reading a plan where the checking happens.
        if let Some(vd) = &self.visibility_data {
            write!(f, ", visibility_checks=[{}]", vd.table_names.join(", "))?;
        }
        Ok(())
    }
}

impl ExecutionPlan for SegmentedTopKExec {
    fn name(&self) -> &str {
        "SegmentedTopKExec"
    }

    fn properties(&self) -> &Arc<PlanProperties> {
        &self.properties
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![&self.input]
    }

    fn with_new_children(
        self: Arc<Self>,
        mut children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let mut new = SegmentedTopKExec::new(
            children.remove(0),
            self.sort_exprs.clone(),
            self.deferred_columns.clone(),
            Arc::clone(&self.ffhelper),
            self.k,
            self.visibility_data.clone(),
        );
        // Preserve the existing dynamic filter so that filter pushdown
        // wiring (which already holds a reference) stays connected.
        new.dynamic_filter = Arc::clone(&self.dynamic_filter);
        Ok(Arc::new(new))
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let mut input_stream = self.input.execute(partition, context)?;
        let rows_input = MetricBuilder::new(&self.metrics).counter("rows_input", partition);
        let rows_output = MetricBuilder::new(&self.metrics).counter("rows_output", partition);
        let segments_seen = MetricBuilder::new(&self.metrics).counter("segments_seen", partition);
        // Only register rows_filtered_invisible in the MetricsSet when visibility
        // filtering is actually active. For non-visibility plans the counter would
        // always be 0, and render_plan_with_metrics shows every registered metric
        // (including zeros), which would pollute EXPLAIN ANALYZE output and break
        // pg_regress .out files. A standalone Count::new() is functionally equivalent
        // for increment purposes but is never added to the MetricsSet.
        let rows_filtered_invisible = if self.visibility_data.is_some() {
            MetricBuilder::new(&self.metrics).counter("rows_filtered_invisible", partition)
        } else {
            Count::new()
        };

        // Build the row converter
        let sort_fields = self
            .sort_exprs
            .iter()
            .map(|expr| {
                let expr_type = expr
                    .expr
                    .data_type(self.properties.eq_properties.schema())?;
                // If it's a deferred column, we treat its sorting type as UInt64 (the ordinal type).
                let data_type = if expr
                    .expr
                    .downcast_ref::<datafusion::physical_expr::expressions::Column>()
                    .is_some_and(|c| {
                        self.deferred_columns
                            .iter()
                            .any(|d| d.sort_col_idx == c.index())
                    }) {
                    arrow_schema::DataType::UInt64
                } else {
                    expr_type
                };
                Ok(SortField::new_with_options(data_type, expr.options))
            })
            .collect::<Result<Vec<_>>>()?;

        let row_converter = RowConverter::new(sort_fields)?;

        let mat_row_converter = Self::create_mat_row_converter(
            &self.sort_exprs,
            &self.deferred_columns,
            &self.ffhelper,
            self.properties.eq_properties.schema(),
        )?;

        // Build per-relation visibility checker entries from the absorbed VFExec data
        // (if any). These are created eagerly here, before the async stream body,
        // so that `execute()` can return a clean `Err` on misconfiguration rather
        // than failing mid-stream.
        let visibility_entries: Vec<StkVisibilityEntry> = if let Some(vd) = &self.visibility_data {
            let resolvers = vd
                .ctid_resolvers
                .lock()
                .expect("ctid_resolvers lock poisoned")
                .clone();
            // SAFETY: GetActiveSnapshot is safe during query execution.
            let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
            if snapshot.is_null() {
                return Err(DataFusionError::Execution(
                    "SegmentedTopKExec: requires an active Postgres snapshot \
                         for visibility checking"
                        .into(),
                ));
            }
            let schema = self.properties.eq_properties.schema();
            let mut entries = Vec::with_capacity(vd.plan_pos_oids.len());
            for &(plan_pos, heap_oid) in &vd.plan_pos_oids {
                let col_name = CtidColumn::new(plan_pos).to_string();
                let (col_idx, _) = schema.column_with_name(&col_name).ok_or_else(|| {
                    DataFusionError::Execution(format!(
                        "SegmentedTopKExec: missing ctid column '{}' \
                                 for visibility checking",
                        col_name
                    ))
                })?;
                let heaprel = PgSearchRelation::open(heap_oid);
                let checker = VisibilityChecker::with_rel_and_snap(&heaprel, snapshot);
                let resolver = resolvers
                    .get(plan_pos)
                    .and_then(|r| r.as_ref().map(|(_, ff)| Arc::clone(ff)))
                    .ok_or_else(|| {
                        DataFusionError::Execution(format!(
                            "SegmentedTopKExec: no ctid resolver wired for \
                                 plan_position {plan_pos}. \
                                 VisibilityCtidResolverRule must run before execute."
                        ))
                    })?;
                entries.push(StkVisibilityEntry {
                    col_idx,
                    checker,
                    resolver,
                    deferred_ctid_state: DeferredCtidMaterializationState::default(),
                });
            }
            entries
        } else {
            Vec::new()
        };

        let mut state = SegmentedTopKState {
            sort_exprs: self.sort_exprs.clone(),
            deferred_columns: self.deferred_columns.clone(),
            ffhelper: Arc::clone(&self.ffhelper),
            k: self.k,
            schema: self.properties.eq_properties.schema().clone(),
            row_converter,
            segment_bufs: Vec::new(),
            segment_cutoffs: Vec::new(),
            dynamic_filter: Arc::clone(&self.dynamic_filter),
            batches: Vec::new(),
            pass_through_rows: Vec::new(),
            last_segment_cutoffs: Vec::new(),
            mat_row_converter,
            last_published_global: None,
            visibility_entries,
            rows_input,
            rows_output,
            segments_seen,
            rows_filtered_invisible,
            pass_through_scratch: Vec::new(),
            row_to_seg_scratch: Vec::new(),
            sort_arrays_scratch: Vec::with_capacity(self.sort_exprs.len()),
        };

        let stream_gen = async_stream::try_stream! {
            use futures::StreamExt;
            while let Some(batch_res) = input_stream.next().await {
                let batch = batch_res?;
                state.rows_input.add(batch.num_rows());

                // Store the batch BEFORE collecting: collect_batch can fire
                // truncate_top_k, whose visibility pass reads the current batch's
                // rows out of state.batches. Cloning a RecordBatch only bumps the
                // column Arcs.
                let batch_idx = state.batches.len();
                state.batches.push(batch);
                let batch = state.batches[batch_idx].clone();
                state.collect_batch(&batch, batch_idx)?;
                state.maybe_compact()?;
            }

            // All input consumed — perform final sort + limit and emit exactly K rows.
            let final_batch = state.emit_final_topk()?;
            if let Some(batch) = final_batch {
                state.rows_output.add(batch.num_rows());
                yield batch;
            }
        };

        // SAFETY: pg_search operates in a single-threaded Tokio executor within Postgres.
        let stream = unsafe {
            UnsafeSendStream::new(stream_gen, self.properties.eq_properties.schema().clone())
        };
        Ok(Box::pin(stream))
    }

    fn metrics(&self) -> Option<MetricsSet> {
        Some(self.metrics.clone_inner())
    }

    /// Pushes `SegmentedTopKExec`'s own [`DynamicFilterPhysicalExpr`] (the global
    /// threshold with materialized string literals) down to child nodes via
    /// DataFusion's standard filter pushdown mechanism.
    fn gather_filters_for_pushdown(
        &self,
        phase: FilterPushdownPhase,
        parent_filters: Vec<Arc<dyn PhysicalExpr>>,
        _config: &datafusion::common::config::ConfigOptions,
    ) -> Result<FilterDescription> {
        // Only push filters in the Post phase (same as SortExec).
        if !matches!(phase, FilterPushdownPhase::Post) {
            return Ok(FilterDescription::all_unsupported(
                &parent_filters,
                &self.children(),
            ));
        }

        // Route parent filters to our child based on column compatibility,
        // and add our own dynamic filter as a self-filter.
        Ok(FilterDescription::new().with_child(
            ChildFilterDescription::from_child(&parent_filters, &self.input)?
                .with_self_filter(Arc::clone(&self.dynamic_filter) as Arc<dyn PhysicalExpr>),
        ))
    }

    fn handle_child_pushdown_result(
        &self,
        _phase: FilterPushdownPhase,
        child_pushdown_result: ChildPushdownResult,
        _config: &datafusion::common::config::ConfigOptions,
    ) -> Result<FilterPushdownPropagation<Arc<dyn ExecutionPlan>>> {
        // Pass through: report parent filter support based on what the child accepted.
        Ok(FilterPushdownPropagation::if_all(child_pushdown_result))
    }
}

/// Per-plan_position runtime state for ctid resolution and visibility checking.
/// A plan will hold one of these entries for every `(plan_pos, heap_oid)` pair it absorbed
/// from a `VisibilityFilterExec`.
struct StkVisibilityEntry {
    /// Index of the `ctid_{plan_position}` column in the input batch schema.
    col_idx: usize,
    /// MVCC visibility checker for this relation.
    checker: VisibilityChecker,
    /// Resolves packed DocAddresses to real ctids before visibility checking.
    resolver: Arc<FFHelper>,
    /// Reusable scratch buffers for packed DocAddress materialization.
    deferred_ctid_state: DeferredCtidMaterializationState,
}

/// One segment's rolling buffer of top-K candidates.
///
/// `rows` holds `(batch_idx, row_idx, sort row)` entries whose locations refer to
/// [`SegmentedTopKState::batches`]. The buffer fills up to its capacity (`2 * K`, or
/// `max(2 * K, MINIMUM_VISIBILITY_CHECK_SIZE)` on visibility plans) and is then pruned
/// back to its K best rows by [`SegmentedTopKState::truncate_top_k`].
#[derive(Default)]
struct SegmentBuf {
    rows: Vec<(usize, usize, OwnedRow)>,
    /// Visibility watermark: `rows[..checked]` have been visibility checked and are
    /// alive. Rows at or past `checked` have not been checked yet. Visibility against
    /// a fixed query snapshot never changes mid-query, so checked rows are checked at
    /// most once. Meaningless (always trailing) on non-visibility plans.
    checked: usize,
}

struct SegmentedTopKState {
    sort_exprs: LexOrdering,
    deferred_columns: Vec<DeferredSortColumn>,
    ffhelper: Arc<FFHelper>,
    k: usize,
    schema: SchemaRef,
    row_converter: RowConverter,
    /// Per-segment rolling buffers, indexed by `SegmentOrdinal` (dense, 0..N).
    /// Filled in `collect_batch` identically for visibility and non-visibility plans;
    /// pruned to the K best rows by `truncate_top_k` whenever a buffer reaches its
    /// capacity, and eagerly before batch compaction.
    segment_bufs: Vec<Option<SegmentBuf>>,
    /// Per-segment K-th best row (the cutoff threshold) after the most recent
    /// `truncate_top_k`, indexed by `SegmentOrdinal`. `None` for segments that have not
    /// yet accumulated K rows. On visibility plans, dead rows are removed before the
    /// QuickSelect, so a cutoff is always derived from live rows only.
    segment_cutoffs: Vec<Option<OwnedRow>>,
    /// Dynamic filter updated with global thresholds (materialized strings).
    /// Pushed down through DataFusion's standard filter pushdown to the scanner.
    dynamic_filter: Arc<DynamicFilterPhysicalExpr>,
    /// Buffered batches during the collection phase.
    batches: Vec<RecordBatch>,
    /// Buffered pass-through rows (NULL ordinals) that bypass
    /// ordinal comparison. These are included in the final sort + limit.
    pass_through_rows: Vec<(usize, usize)>,

    /// For each segment that has a cutoff, we cache the resolved values of its current
    /// K-th best row (the cutoff threshold). Indexed by `SegmentOrdinal`; `None` for
    /// segments without a resolved cutoff yet.
    /// Tuple: (local ordinal OwnedRow, materialized ScalarValues, materialized OwnedRow)
    last_segment_cutoffs: Vec<Option<(OwnedRow, Vec<datafusion::common::ScalarValue>, OwnedRow)>>,

    /// Row converter for materialized sorts, used to compare resolved thresholds lexicographically.
    mat_row_converter: RowConverter,

    /// Cache of the last published global threshold to avoid redundant filter updates.
    /// Stores the best of the worst materialized rows across segments.
    last_published_global: Option<OwnedRow>,

    rows_input: Count,
    rows_output: Count,
    /// Counts segments that had rows participating in ordinal comparison (States 0+1).
    /// Segments with only NULLs are not counted.
    segments_seen: Count,
    /// Counts rows removed because they were dead (invisible) under the current snapshot.
    /// Incremented by `truncate_top_k` and at final emission (emit_final_topk).
    rows_filtered_invisible: Count,
    /// Runtime visibility checker entries, one per absorbed `(plan_pos, heap_oid)` pair.
    /// Empty when no `VisibilityFilterExec` was absorbed.
    visibility_entries: Vec<StkVisibilityEntry>,

    /// Scratch buffers to avoid per-batch allocation
    pass_through_scratch: Vec<bool>,
    row_to_seg_scratch: Vec<Option<SegmentOrdinal>>,
    sort_arrays_scratch: Vec<ArrayRef>,
}

impl SegmentedTopKState {
    /// Return a mutable reference to the per-segment slot at `idx`, growing the
    /// vector with `None`s as needed. `SegmentOrdinal` is dense (0..N), so these
    /// per-segment vectors are indexed directly by the ordinal.
    fn ensure_slot<T>(vec: &mut Vec<Option<T>>, idx: usize) -> &mut Option<T> {
        if idx >= vec.len() {
            vec.resize_with(idx + 1, || None);
        }
        &mut vec[idx]
    }

    fn get_or_create_segment_buf(&mut self, seg_idx: usize) -> &mut SegmentBuf {
        let slot = Self::ensure_slot(&mut self.segment_bufs, seg_idx);
        if slot.is_none() {
            self.segments_seen.add(1);
        }
        slot.get_or_insert_with(SegmentBuf::default)
    }

    /// Per-segment buffer capacity: `2 * K`, raised to
    /// `MINIMUM_VISIBILITY_CHECK_SIZE` on visibility plans so that each visibility
    /// pass in [`Self::truncate_top_k`] covers a reasonably sized batch.
    fn buffer_capacity(&self) -> usize {
        if self.visibility_entries.is_empty() {
            2 * self.k
        } else {
            (2 * self.k).max(MINIMUM_VISIBILITY_CHECK_SIZE)
        }
    }

    /// Prune one segment's buffer down to its K best rows:
    ///
    /// 1. visibility check the unchecked suffix of the buffer (everything past the
    ///    `checked` watermark; the whole buffer on its first fill) and drop dead rows,
    /// 2. QuickSelect the K best of the remaining (all live) rows,
    /// 3. record the K-th best as the segment cutoff and truncate the buffer to K,
    /// 4. publish the (possibly improved) global threshold.
    ///
    /// On non-visibility plans step 1 is a no-op, which is what lets one code path
    /// serve both cases. On visibility plans, every row behind the watermark was
    /// checked alive against the query snapshot, and visibility against a fixed
    /// snapshot never changes mid-query, so each row is checked at most once and the
    /// cutoff (and therefore the published threshold) is always derived from live
    /// rows only. That also means a previously published threshold can never become
    /// too aggressive after dead rows are removed: dead rows only ever exist past the
    /// watermark and never define a cutoff.
    fn truncate_top_k(&mut self, seg_idx: usize) -> Result<()> {
        if self.k == 0 {
            return Ok(());
        }

        // Step 1: visibility check the unchecked suffix and drop dead rows.
        if !self.visibility_entries.is_empty() {
            let (watermark, row_keys) = {
                let Some(buf) = self.segment_bufs.get(seg_idx).and_then(|b| b.as_ref()) else {
                    return Ok(());
                };
                let keys: Vec<(usize, usize)> = buf.rows[buf.checked..]
                    .iter()
                    .map(|(bi, ri, _)| (*bi, *ri))
                    .collect();
                (buf.checked, keys)
            };
            if !row_keys.is_empty() {
                // Corrected ctids are discarded here: the stored batches keep packed
                // DocAddresses (real ctid write-back happens at emit_final_topk).
                let (visible_mask, _) = self.check_rows_visible(&row_keys)?;
                let dead = visible_mask.iter().filter(|&&v| !v).count();
                if dead > 0 {
                    self.rows_filtered_invisible.add(dead);
                    if let Some(buf) = self.segment_bufs.get_mut(seg_idx).and_then(|b| b.as_mut()) {
                        let mut idx = 0usize;
                        buf.rows.retain(|_| {
                            let keep = idx < watermark || visible_mask[idx - watermark];
                            idx += 1;
                            keep
                        });
                    }
                }
            }
            if let Some(buf) = self.segment_bufs.get_mut(seg_idx).and_then(|b| b.as_mut()) {
                buf.checked = buf.rows.len();
            }
        }

        // Steps 2 + 3: QuickSelect around the K-th element, record the cutoff,
        // truncate the buffer to its K best rows.
        let cutoff = {
            let Some(buf) = self.segment_bufs.get_mut(seg_idx).and_then(|b| b.as_mut()) else {
                return Ok(());
            };
            if buf.rows.len() < self.k {
                // Fewer than K live rows so far (only possible before this segment's
                // first cutoff): no cutoff yet, keep filling.
                return Ok(());
            }
            buf.rows
                .select_nth_unstable_by(self.k - 1, |a, b| a.2.cmp(&b.2));
            let cutoff = buf.rows[self.k - 1].2.clone();
            buf.rows.truncate(self.k);
            buf.checked = buf.checked.min(buf.rows.len());
            cutoff
        };
        *Self::ensure_slot(&mut self.segment_cutoffs, seg_idx) = Some(cutoff);

        // Step 4: publish the improved threshold so the scan can prune earlier.
        self.publish_global_threshold()
    }

    /// Ingest a single batch: extract ordinals, update per-segment buffers,
    /// and publish thresholds. The batch is buffered for the final emission
    /// phase. Pass-through rows (NULL ordinals) are buffered
    /// in `pass_through_rows` for the final sort + limit.
    fn collect_batch(&mut self, batch: &RecordBatch, batch_idx: usize) -> Result<()> {
        let num_rows = batch.num_rows();
        self.pass_through_scratch.clear();
        self.pass_through_scratch.resize(num_rows, false);
        self.row_to_seg_scratch.clear();
        self.row_to_seg_scratch.resize(num_rows, None);
        let mut deferred_ords: HashMap<usize, Vec<Option<TermOrdinal>>> = HashMap::default();

        for deferred_col in &self.deferred_columns {
            let global_term_ords = Self::extract_deferred_ordinals(
                &self.ffhelper,
                batch,
                deferred_col,
                num_rows,
                &mut self.pass_through_scratch,
                &mut self.row_to_seg_scratch,
            )?;
            deferred_ords.insert(deferred_col.sort_col_idx, global_term_ords);
        }

        // Build the evaluation arrays for the RowConverter
        self.sort_arrays_scratch.clear();
        for expr in &self.sort_exprs {
            let col_idx = expr
                .expr
                .downcast_ref::<datafusion::physical_expr::expressions::Column>()
                .map(|c| c.index());

            if let Some(Some(ords)) = col_idx.map(|idx| deferred_ords.remove(&idx)) {
                // Use our artificially constructed ordinals array
                let ords_array = Arc::new(UInt64Array::from(ords)) as ArrayRef;
                self.sort_arrays_scratch.push(ords_array);
            } else {
                let val = expr.expr.evaluate(batch)?;
                self.sort_arrays_scratch.push(val.into_array(num_rows)?);
            }
        }

        let converted_rows = self
            .row_converter
            .convert_columns(&self.sort_arrays_scratch)?;

        // Buffer capacity: plain 2 * K, but on visibility plans at least
        // MINIMUM_VISIBILITY_CHECK_SIZE so each visibility pass in truncate_top_k
        // covers a reasonably sized batch.
        let capacity = self.buffer_capacity();

        for row_idx in 0..num_rows {
            if self.pass_through_scratch[row_idx] {
                self.pass_through_rows.push((batch_idx, row_idx));
                continue;
            }

            if let Some(seg_idx) = self.row_to_seg_scratch[row_idx].map(|s| s as usize) {
                let row_view = converted_rows.row(row_idx);

                // Pre-filter: rows already worse than this segment's cutoff cannot
                // enter the top K, so drop them before they reach the buffer. On
                // visibility plans the cutoff is always derived from live rows (see
                // truncate_top_k), so this never prunes a row that a dead row would
                // otherwise have displaced.
                if self
                    .segment_cutoffs
                    .get(seg_idx)
                    .and_then(|c| c.as_ref())
                    .is_some_and(|cutoff| row_view.as_ref() > cutoff.as_ref())
                {
                    continue;
                }

                let buf_len = {
                    let buf = self.get_or_create_segment_buf(seg_idx);
                    buf.rows.push((batch_idx, row_idx, row_view.owned()));
                    buf.rows.len()
                };

                if self.k > 0 && buf_len >= capacity {
                    self.truncate_top_k(seg_idx)?;
                }
            }
        }

        Ok(())
    }

    /// Helper to extract term ordinals from a deferred UnionArray.
    /// Mutates `pass_through` for rows that contain NULLs, and populates `row_to_seg` mapping.
    fn extract_deferred_ordinals(
        ffhelper: &FFHelper,
        batch: &RecordBatch,
        deferred_col: &DeferredSortColumn,
        num_rows: usize,
        pass_through: &mut [bool],
        row_to_seg: &mut [Option<SegmentOrdinal>],
    ) -> Result<Vec<Option<TermOrdinal>>> {
        let column = batch.column(deferred_col.sort_col_idx);
        let union_col = column
            .as_any()
            .downcast_ref::<UnionArray>()
            .ok_or_else(|| {
                DataFusionError::Internal(format!(
                    "SegmentedTopKExec: sort column should be a deferred UnionArray but found {:?} at index {}",
                    column.data_type(), deferred_col.sort_col_idx
                ))
            })?;

        let type_ids = union_col.type_ids();
        let offsets = union_col.offsets().ok_or_else(|| {
            DataFusionError::Internal("SegmentedTopKExec: expected dense union with offsets".into())
        })?;

        let mut state0_rows: Vec<usize> = Vec::new();
        let mut state1_rows: Vec<usize> = Vec::new();
        for row_idx in 0..num_rows {
            match type_ids[row_idx] {
                0 => state0_rows.push(row_idx),
                1 => state1_rows.push(row_idx),
                _ => unreachable!("Invalid Union state"),
            }
        }

        let mut global_term_ords: Vec<Option<TermOrdinal>> = vec![None; num_rows];

        // State 0: compact doc address child.
        let mut state0_by_seg: HashMap<SegmentOrdinal, Vec<(usize, DocId)>> = HashMap::default();
        if !state0_rows.is_empty() {
            let doc_addr_child = union_col
                .child(0)
                .as_any()
                .downcast_ref::<UInt64Array>()
                .ok_or_else(|| {
                    DataFusionError::Internal(
                        "SegmentedTopKExec: child 0 should be UInt64 doc addresses".into(),
                    )
                })?;
            for &row_idx in &state0_rows {
                let packed = doc_addr_child.value(offsets[row_idx] as usize);
                let (seg_ord, doc_id) = unpack_doc_address(packed);
                state0_by_seg
                    .entry(seg_ord)
                    .or_default()
                    .push((row_idx, doc_id));
                row_to_seg[row_idx] = Some(seg_ord);
            }
        }

        // State 1: compact term ordinal child.
        if !state1_rows.is_empty() {
            let term_ord_child = union_col
                .child(1)
                .as_any()
                .downcast_ref::<StructArray>()
                .ok_or_else(|| {
                    DataFusionError::Internal(
                        "SegmentedTopKExec: child 1 should be StructArray of term ordinals".into(),
                    )
                })?;
            let seg_ord_array = term_ord_child
                .column(0)
                .as_any()
                .downcast_ref::<UInt32Array>()
                .ok_or_else(|| {
                    DataFusionError::Internal(
                        "SegmentedTopKExec: term_ordinal.segment_ord should be UInt32".into(),
                    )
                })?;
            let ord_array = term_ord_child
                .column(1)
                .as_any()
                .downcast_ref::<UInt64Array>()
                .ok_or_else(|| {
                    DataFusionError::Internal(
                        "SegmentedTopKExec: term_ordinal.term_ord should be UInt64".into(),
                    )
                })?;

            for &row_idx in &state1_rows {
                let ci = offsets[row_idx] as usize;
                let seg_ord = seg_ord_array.value(ci);
                row_to_seg[row_idx] = Some(seg_ord);
                if !ord_array.is_null(ci) {
                    let ord = ord_array.value(ci);
                    if ord != NULL_TERM_ORDINAL {
                        global_term_ords[row_idx] = Some(ord);
                    } else {
                        pass_through[row_idx] = true;
                    }
                } else {
                    pass_through[row_idx] = true;
                }
            }
        }

        // Bulk-fetch term ordinals for State 0 rows via FFHelper
        for (seg_ord, rows) in state0_by_seg {
            let doc_ids: Vec<DocId> = rows.iter().map(|(_, doc_id)| *doc_id).collect();
            let mut term_ords: Vec<Option<TermOrdinal>> = vec![None; doc_ids.len()];

            let col = ffhelper.column(seg_ord, deferred_col.canonical.ff_index);
            match col {
                FFType::Text(str_col) => {
                    str_col.ords().first_vals(&doc_ids, &mut term_ords);
                }
                FFType::Bytes(bytes_col) => {
                    bytes_col.ords().first_vals(&doc_ids, &mut term_ords);
                }
                _ => {
                    panic!(
                            "SegmentedTopKExec: ff_index {} is not a Text or Bytes dictionary column \
                             — the optimizer should never plan this node for non-dictionary columns",
                            deferred_col.canonical.ff_index
                        );
                }
            }

            for (i, (row_idx, _)) in rows.into_iter().enumerate() {
                let ord = term_ords[i].unwrap_or(NULL_TERM_ORDINAL);
                if ord == NULL_TERM_ORDINAL {
                    pass_through[row_idx] = true;
                } else {
                    global_term_ords[row_idx] = Some(ord);
                }
            }
        }

        Ok(global_term_ords)
    }

    /// Build a chained lexicographic filter expression from threshold values.
    ///
    /// For `ORDER BY a ASC, b ASC` with thresholds `(t_a, t_b)`, produces:
    ///   `a < t_a OR (a = t_a AND b < t_b)`
    ///
    /// Handles NULL semantics via IS NULL / IS NOT NULL based on NULLS FIRST/LAST.
    fn build_lexicographic_filter(
        sort_exprs: &LexOrdering,
        values: &[datafusion::common::ScalarValue],
    ) -> Option<Arc<dyn PhysicalExpr>> {
        use datafusion::logical_expr::Operator;
        use datafusion::physical_expr::expressions::{is_not_null, is_null, lit, BinaryExpr};

        let mut filters = Vec::with_capacity(values.len());
        let mut prev_eq: Option<Arc<dyn PhysicalExpr>> = None;

        for (sort_expr, value) in sort_exprs.iter().zip(values) {
            let col_expr = &sort_expr.expr;
            let op = if sort_expr.options.descending {
                Operator::Gt
            } else {
                Operator::Lt
            };

            let value_null = value.is_null();

            // col <op> threshold
            let comparison = Arc::new(BinaryExpr::new(
                Arc::clone(col_expr),
                op,
                lit(value.clone()),
            )) as Arc<dyn PhysicalExpr>;

            // Wrap with NULL handling.
            let filter = match (sort_expr.options.nulls_first, value_null) {
                (true, true) => lit(false),
                (true, false) => {
                    let is_null_expr = is_null(Arc::clone(col_expr)).ok()?;
                    Arc::new(BinaryExpr::new(is_null_expr, Operator::Or, comparison))
                        as Arc<dyn PhysicalExpr>
                }
                (false, true) => is_not_null(Arc::clone(col_expr)).ok()?,
                (false, false) => comparison,
            };

            // col = threshold (for tiebreaker chaining).
            let mut eq_expr = Arc::new(BinaryExpr::new(
                Arc::clone(col_expr),
                Operator::Eq,
                lit(value.clone()),
            )) as Arc<dyn PhysicalExpr>;
            if value_null {
                let is_null_expr = is_null(Arc::clone(col_expr)).ok()?;
                eq_expr = Arc::new(BinaryExpr::new(is_null_expr, Operator::Or, eq_expr));
            }

            // Chain: first column stands alone; subsequent columns are
            // gated by "all prior columns equal their thresholds".
            match prev_eq.take() {
                None => {
                    filters.push(filter);
                }
                Some(p) => {
                    filters.push(Arc::new(BinaryExpr::new(
                        Arc::clone(&p),
                        Operator::And,
                        filter,
                    )));
                    eq_expr = Arc::new(BinaryExpr::new(p, Operator::And, eq_expr));
                }
            }
            prev_eq = Some(eq_expr);
        }

        filters
            .into_iter()
            .reduce(|a, b| Arc::new(BinaryExpr::new(a, Operator::Or, b)) as Arc<dyn PhysicalExpr>)
    }

    /// Evaluates the current local thresholds across all segments to determine
    /// if a new global threshold can be published.
    ///
    /// This method is responsible for computing a safe, conservative global threshold
    /// by finding the "best of the worst" materialized cutoff among all full segments.
    /// By only resolving the worst entry of each segment rather than every row, it
    /// minimizes the overhead of translating segment-local ordinals into global strings.
    fn publish_global_threshold(&mut self) -> Result<()> {
        let mut best_worst_mat_row: Option<OwnedRow> = None;
        let mut best_worst_values: Option<Vec<datafusion::common::ScalarValue>> = None;

        // 1. Examine the "worst" row (the root of the heap) for each segment that
        //    has reached size `K`.
        let full_segment_cutoffs: Vec<(SegmentOrdinal, OwnedRow)> = self
            .segment_cutoffs
            .iter()
            .enumerate()
            .filter_map(|(i, cutoff)| cutoff.as_ref().map(|c| (i as SegmentOrdinal, c.clone())))
            .collect();

        for (seg_ord, worst_local) in full_segment_cutoffs {
            // 2. Resolve the local ordinal threshold into a materialized row.
            let (mat_values, mat_row) = self.resolve_segment_cutoff(seg_ord, &worst_local)?;

            // 3. Find the "best of the worst" (minimum of maximums) among all segments'
            //    thresholds. If we use a bound greater than any full segment's local cutoff,
            //    we might prune competitive rows in other segments. By taking the tightest
            //    upper bound across all full segments, we ensure a mathematically safe
            //    threshold for global pruning.
            match &best_worst_mat_row {
                None => {
                    best_worst_mat_row = Some(mat_row);
                    best_worst_values = Some(mat_values);
                }
                Some(current_best) => {
                    if &mat_row < current_best {
                        best_worst_mat_row = Some(mat_row);
                        best_worst_values = Some(mat_values);
                    }
                }
            }
        }

        // 4. Finally, if the newly calculated global threshold is better than the one
        //    we previously published, build a new dynamic filter expression and
        //    push it down to the scanner.
        let (Some(best_row), Some(best_values)) = (best_worst_mat_row, best_worst_values) else {
            return Ok(());
        };

        let changed = match &self.last_published_global {
            Some(prev) => &best_row != prev,
            None => true,
        };

        if changed {
            if let Some(expr) = Self::build_lexicographic_filter(&self.sort_exprs, &best_values) {
                let _ = self.dynamic_filter.update(expr);
                self.last_published_global = Some(best_row);
            }
        }

        Ok(())
    }

    /// Resolves a segment-local ordinal threshold into a globally comparable materialized row.
    ///
    /// It attempts to reuse previously resolved values if the local threshold hasn't changed.
    /// If the threshold is new, it pays the cost to decode the segment-local ordinals via
    /// `FFHelper::ord_to_str` and then constructs a materialized `OwnedRow`.
    fn resolve_segment_cutoff(
        &mut self,
        seg_ord: SegmentOrdinal,
        worst_local: &OwnedRow,
    ) -> Result<(Vec<datafusion::common::ScalarValue>, OwnedRow)> {
        // a. Compare this local threshold with a cached version from the previous
        //    batch. If the threshold hasn't changed, reuse the materialized string
        //    values. If it has changed, pay the cost to resolve the segment-local
        //    ordinals into global string/bytes values via `resolve_global_threshold_values`.
        if let Some((cached_local, vals, row)) = self
            .last_segment_cutoffs
            .get(seg_ord as usize)
            .and_then(|c| c.as_ref())
        {
            if cached_local == worst_local {
                return Ok((vals.clone(), row.clone()));
            }
        }

        let arrays = self
            .row_converter
            .convert_rows(std::iter::once(worst_local.row()))?;

        let values = self.resolve_global_threshold_values(&arrays, seg_ord)?;

        let val_arrays = values
            .iter()
            .map(|v| v.to_array())
            .collect::<Result<Vec<_>, _>>()?;

        // b. Convert these materialized scalar values into an `OwnedRow` using
        //    `mat_row_converter`. This enables fast, correct lexicographical comparison
        //    of values across different segments.
        let converted = self.mat_row_converter.convert_columns(&val_arrays)?;

        let mat_row = converted.row(0).owned();
        *Self::ensure_slot(&mut self.last_segment_cutoffs, seg_ord as usize) =
            Some((worst_local.clone(), values.clone(), mat_row.clone()));
        Ok((values, mat_row))
    }

    /// Resolve threshold values for the global filter.
    ///
    /// For deferred columns, converts ordinals back to materialized strings
    /// via `FFHelper::ord_to_str`. For non-deferred columns, reads the scalar
    /// directly from the array. Returns `None` if any conversion fails.
    fn resolve_global_threshold_values(
        &self,
        arrays: &[ArrayRef],
        seg_ord: SegmentOrdinal,
    ) -> Result<Vec<datafusion::common::ScalarValue>> {
        use datafusion::common::ScalarValue;

        let mut values = Vec::with_capacity(self.sort_exprs.len());
        for (i, sort_expr) in self.sort_exprs.iter().enumerate() {
            let is_deferred = sort_expr
                .expr
                .downcast_ref::<datafusion::physical_expr::expressions::Column>()
                .and_then(|c| {
                    self.deferred_columns
                        .iter()
                        .find(|d| d.sort_col_idx == c.index())
                });

            let value = if let Some(deferred) = is_deferred {
                let term_ord = arrays[i]
                    .as_any()
                    .downcast_ref::<UInt64Array>()
                    .ok_or_else(|| {
                        datafusion::error::DataFusionError::Internal(
                            "Expected UInt64Array for deferred ordinal".to_string(),
                        )
                    })?
                    .value(0);
                let col = self.ffhelper.column(seg_ord, deferred.canonical.ff_index);
                match col {
                    FFType::Text(str_col) => {
                        let mut s = String::new();
                        str_col.ord_to_str(term_ord, &mut s).map_err(|e| {
                            datafusion::error::DataFusionError::Internal(format!(
                                "Failed to resolve string ordinal: {}",
                                e
                            ))
                        })?;
                        ScalarValue::Utf8View(Some(s))
                    }
                    FFType::Bytes(bytes_col) => {
                        let mut b = Vec::new();
                        bytes_col.ord_to_bytes(term_ord, &mut b).map_err(|e| {
                            datafusion::error::DataFusionError::Internal(format!(
                                "Failed to resolve bytes ordinal: {}",
                                e
                            ))
                        })?;
                        ScalarValue::BinaryView(Some(b))
                    }
                    _ => {
                        return Err(datafusion::error::DataFusionError::Internal(
                            "Unexpected column type for deferred field".to_string(),
                        ));
                    }
                }
            } else {
                ScalarValue::try_from_array(&arrays[i], 0)?
            };
            values.push(value);
        }
        Ok(values)
    }

    /// Compact the stored batches by discarding rows no longer referenced by any
    /// per-segment buffer (or pass-through). This bounds memory at O(K * segments)
    /// instead of O(N) for large inputs — analogous to the batch compaction
    /// step in upstream DataFusion Top K.
    fn maybe_compact(&mut self) -> Result<()> {
        // Fire only when the stored batches hold at least twice as many rows as are
        // still referenced, so each compaction at least halves the stored rows
        // (amortized O(1) work per input row). The floor keeps small inputs from
        // ever compacting, mirroring the previous 4 * K * segments trigger.
        let referenced: usize = self
            .segment_bufs
            .iter()
            .flatten()
            .map(|b| b.rows.len())
            .sum::<usize>()
            + self.pass_through_rows.len();
        let stored: usize = self.batches.iter().map(|b| b.num_rows()).sum();
        let num_segments = self.segment_bufs.iter().flatten().count().max(1);
        let floor = 4 * self.k * num_segments;
        if stored < (2 * referenced).max(floor) {
            return Ok(());
        }

        // Eagerly truncate every buffer first so compaction works with fresh, live
        // top-K survivors: dead rows in an unchecked suffix are never compacted into
        // the retained batch, and buffers holding more than K rows are pruned before
        // their rows are copied.
        for seg_idx in 0..self.segment_bufs.len() {
            let has_rows = self.segment_bufs[seg_idx]
                .as_ref()
                .is_some_and(|b| !b.rows.is_empty());
            if has_rows {
                self.truncate_top_k(seg_idx)?;
            }
        }

        // Survivors are exactly the rows still referenced by a buffer or pass-through.
        let mut survivors = crate::api::HashSet::default();
        for buf in self.segment_bufs.iter().flatten() {
            for (batch_idx, row_idx, _) in &buf.rows {
                survivors.insert((*batch_idx, *row_idx));
            }
        }
        for &(batch_idx, row_idx) in &self.pass_through_rows {
            survivors.insert((batch_idx, row_idx));
        }

        if survivors.is_empty() {
            self.batches.clear();
            return Ok(());
        }

        // Filter each stored batch, build old→new row mapping, concatenate.
        let mut filtered_batches = Vec::new();
        let mut mapping: HashMap<(usize, usize), usize> = HashMap::default();
        let mut global_offset = 0usize;

        for (batch_idx, batch) in self.batches.iter().enumerate() {
            let mask: BooleanArray = (0..batch.num_rows())
                .map(|ri| Some(survivors.contains(&(batch_idx, ri))))
                .collect();

            if mask.true_count() == 0 {
                continue;
            }

            for ri in 0..batch.num_rows() {
                if survivors.contains(&(batch_idx, ri)) {
                    mapping.insert((batch_idx, ri), global_offset);
                    global_offset += 1;
                }
            }

            let filtered = filter_record_batch(batch, &mask)
                .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
            filtered_batches.push(filtered);
        }

        let compacted = concat_batches(&self.schema, &filtered_batches)
            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;

        // Remap buffer rows and pass_through_rows into the single compacted batch.
        for buf in self.segment_bufs.iter_mut().flatten() {
            for entry in &mut buf.rows {
                let new_ri = mapping[&(entry.0, entry.1)];
                entry.0 = 0;
                entry.1 = new_ri;
            }
        }
        for entry in &mut self.pass_through_rows {
            let new_ri = mapping[&(entry.0, entry.1)];
            entry.0 = 0;
            entry.1 = new_ri;
        }

        self.batches = vec![compacted];
        Ok(())
    }

    /// Check visibility for a set of rows identified by `(batch_idx, row_idx)` pairs.
    ///
    /// For each absorbed `(plan_pos, heap_oid)` pair, extracts the packed DocAddress
    /// from the ctid column of the stored batch, resolves it to a real ctid via
    /// `FFHelper`, then calls `VisibilityChecker::check_batch`. The overall result
    /// is the AND of all per-relation visibility masks.
    ///
    /// Returns `(visible_mask, corrected_ctids_per_entry)`:
    /// - `visible_mask[i]` is `false` when the i-th row is invisible to any relation.
    /// - `corrected_ctids_per_entry[e][i]` is the HOT-corrected real ctid for the i-th
    ///   row as seen by entry `e`. `None` means invisible (or unresolvable) for that
    ///   entry. These HOT-corrected values must be used in the final output so that
    ///   `fetch_tuple_direct` (which does NOT follow HOT chains) receives the right ctid.
    ///
    /// Returns all-true / empty-corrected immediately when `visibility_entries` is empty.
    #[allow(clippy::type_complexity)]
    fn check_rows_visible(
        &mut self,
        row_keys: &[(usize, usize)],
    ) -> Result<(Vec<bool>, Vec<Vec<Option<u64>>>)> {
        if self.visibility_entries.is_empty() || row_keys.is_empty() {
            return Ok((vec![true; row_keys.len()], Vec::new()));
        }

        let n = row_keys.len();
        let mut overall_visible = vec![true; n];
        let mut all_corrected: Vec<Vec<Option<u64>>> =
            Vec::with_capacity(self.visibility_entries.len());

        for entry in self.visibility_entries.iter_mut() {
            // Track which input positions had a null ctid so we can mark them
            // invisible regardless of what materialize_deferred_ctid returns.
            // We must NOT use 0 as a sentinel: (seg_ord=0, doc_id=0) is a valid
            // packed DocAddress that could resolve to a real ctid.
            let mut input_null: Vec<bool> = Vec::with_capacity(n);

            // Extract packed doc addresses from the stored batches.
            let mut packed_values: Vec<u64> = Vec::with_capacity(n);
            for &(batch_idx, row_idx) in row_keys {
                let col = self.batches[batch_idx].column(entry.col_idx);
                let arr = col.as_any().downcast_ref::<UInt64Array>().ok_or_else(|| {
                    DataFusionError::Internal("SegmentedTopKExec: ctid column is not UInt64".into())
                })?;
                if arr.is_null(row_idx) {
                    input_null.push(true);
                    packed_values.push(0u64); // placeholder; overridden below
                } else {
                    input_null.push(false);
                    packed_values.push(arr.value(row_idx));
                }
            }

            // Resolve packed DocAddresses → real ctids via FFHelper.
            let packed_array = UInt64Array::from(packed_values);
            let resolved_array = materialize_deferred_ctid(
                &entry.resolver,
                &packed_array,
                &mut entry.deferred_ctid_state,
            )?;
            let resolved = resolved_array
                .as_any()
                .downcast_ref::<UInt64Array>()
                .ok_or_else(|| {
                    DataFusionError::Internal(
                        "SegmentedTopKExec: resolved ctid array is not UInt64".into(),
                    )
                })?;

            // Collect valid (non-null, non-null-input) ctids with their original indices.
            // check_batch panics on None inputs, so we filter to resolvable rows only.
            let mut valid: Vec<(usize, u64)> = Vec::with_capacity(n);
            for (i, vis) in overall_visible.iter_mut().enumerate() {
                if input_null[i] || resolved.is_null(i) {
                    // Null input ctid or unresolvable → treat as invisible.
                    *vis = false;
                } else {
                    valid.push((i, resolved.value(i)));
                }
            }

            // Per-entry HOT-corrected ctids: None for invisible rows, Some(ctid) for
            // visible rows. Populated from check_batch results below.
            let mut entry_corrected: Vec<Option<u64>> = vec![None; n];

            if !valid.is_empty() {
                let ctids_for_check: Vec<Option<u64>> =
                    valid.iter().map(|(_, c)| Some(*c)).collect();
                let mut results: Vec<Option<u64>> = vec![None; valid.len()];
                entry.checker.check_batch(&ctids_for_check, &mut results);

                for ((orig_idx, _), result) in valid.iter().zip(results.iter()) {
                    match result {
                        // heap_hot_search_buffer returned the HOT-corrected ctid.
                        // Store it so emit_final_topk can write it to the output
                        // column instead of the raw index ctid.
                        Some(corrected_ctid) => {
                            entry_corrected[*orig_idx] = Some(*corrected_ctid);
                        }
                        None => {
                            overall_visible[*orig_idx] = false;
                        }
                    }
                }
            }

            all_corrected.push(entry_corrected);
        }

        Ok((overall_visible, all_corrected))
    }

    /// Perform the final sort + limit after all input is consumed.
    ///
    ///
    /// Steps:
    /// 1. Collect candidates from the per-segment buffers.
    /// 2. Merge them with pass-through rows into the candidate set.
    ///    2a. (Visibility) Filter invisible rows from candidates, including pass-through rows.
    /// 3. Materialize sort column values for each candidate.
    /// 4. Sort candidates by materialized values, take top K.
    /// 5. Emit a single sorted batch.
    fn emit_final_topk(&mut self) -> Result<Option<RecordBatch>> {
        use datafusion::common::ScalarValue;

        // 1. Collect candidates: every row still held in a per-segment buffer. Each
        //    buffer holds its segment's current top K plus any not-yet-truncated
        //    recent rows, all within the segment cutoff (enforced on insert), so the
        //    true per-segment top K is always a subset of the buffer. On visibility
        //    plans, rows past a buffer's watermark may still be dead; the visibility
        //    pass below removes them, and it can never leave a segment short: the K
        //    rows kept by the last truncate_top_k were checked alive, and visibility
        //    against a fixed snapshot never changes mid-query.
        type Candidate = (usize, usize, Option<(SegmentOrdinal, OwnedRow)>);
        let mut candidates: Vec<Candidate> = Vec::new();

        for (seg_idx, slot) in self.segment_bufs.iter().enumerate() {
            let Some(buf) = slot else { continue };
            for (batch_idx, row_idx, row_val) in &buf.rows {
                candidates.push((
                    *batch_idx,
                    *row_idx,
                    Some((seg_idx as SegmentOrdinal, row_val.clone())),
                ));
            }
        }

        // Always include pass-through rows (NULL ordinals).
        for &(batch_idx, row_idx) in &self.pass_through_rows {
            candidates.push((batch_idx, row_idx, None));
        }

        // 2a. Visibility filter: remove invisible rows from candidates.
        //     pass_through_rows are checked here (they bypass the prune cycle).
        //
        // corrected_lookup[entry_idx][(batch_idx, row_idx)] = HOT-corrected real ctid.
        // Populated here and consumed in the final output column write below so that
        // fetch_tuple_direct (which does NOT follow HOT chains) gets the right address.
        // Declared outside the if block so it remains in scope for the output step.
        let corrected_lookup: Vec<HashMap<(usize, usize), u64>>;

        if !self.visibility_entries.is_empty() && !candidates.is_empty() {
            let row_keys: Vec<(usize, usize)> =
                candidates.iter().map(|(bi, ri, _)| (*bi, *ri)).collect();
            let (visible_mask, corrected_per_entry) = self.check_rows_visible(&row_keys)?;
            // Build per-entry (batch_idx, row_idx) → HOT-corrected ctid lookup.
            corrected_lookup = corrected_per_entry
                .into_iter()
                .map(|ctids| {
                    row_keys
                        .iter()
                        .zip(ctids)
                        .filter_map(|(key, opt)| opt.map(|c| (*key, c)))
                        .collect::<HashMap<_, _>>()
                })
                .collect();
            let invisible_count = visible_mask.iter().filter(|&&v| !v).count();
            if invisible_count > 0 {
                self.rows_filtered_invisible.add(invisible_count);
            }
            candidates = candidates
                .into_iter()
                .zip(visible_mask)
                .filter(|(_, visible)| *visible)
                .map(|(c, _)| c)
                .collect();
        } else {
            corrected_lookup = Vec::new();
        }

        if candidates.is_empty() {
            return Ok(None);
        }

        // 3. Materialize sort column values for each candidate and build a
        //    second RowConverter using materialized data types (Utf8View/BinaryView
        //    for deferred columns, original type for non-deferred).
        struct SortCol<'a> {
            expr: &'a datafusion::physical_expr::PhysicalSortExpr,
            deferred: Option<&'a DeferredSortColumn>,
            mat_type: arrow_schema::DataType,
        }

        let sort_cols: Vec<SortCol> = self
            .sort_exprs
            .iter()
            .map(|expr| {
                let deferred = expr
                    .expr
                    .downcast_ref::<datafusion::physical_expr::expressions::Column>()
                    .and_then(|c| {
                        self.deferred_columns
                            .iter()
                            .find(|d| d.sort_col_idx == c.index())
                    });
                let mat_type = if let Some(deferred) = deferred {
                    match self.ffhelper.column(0, deferred.canonical.ff_index) {
                        FFType::Bytes(_) => arrow_schema::DataType::BinaryView,
                        _ => arrow_schema::DataType::Utf8View,
                    }
                } else {
                    expr.expr
                        .data_type(&self.schema)
                        .unwrap_or(arrow_schema::DataType::Utf8View)
                };
                SortCol {
                    expr,
                    deferred,
                    mat_type,
                }
            })
            .collect();

        let materialized_sort_fields: Vec<SortField> = sort_cols
            .iter()
            .map(|sort_col| {
                SortField::new_with_options(sort_col.mat_type.clone(), sort_col.expr.options)
            })
            .collect();

        let mat_row_converter = RowConverter::new(materialized_sort_fields)?;

        // A NULL must match the RowConverter's declared field type:
        // convert_columns rejects mismatches ("expected BinaryView got
        // Utf8View" for a NULL in a Bytes-backed NUMERIC sort key).
        // If the type is unsupported, propagate the error rather than
        // substituting a differently typed NULL that the converter will reject.
        let typed_null = |sort_col: &SortCol| -> Result<ScalarValue> {
            ScalarValue::try_from(&sort_col.mat_type)
        };

        // Batch-convert all ordinal survivors' rows in a single convert_rows call.
        // We collect `Row<'_>` directly to avoid cloning `OwnedRow`.
        let ord_rows: Vec<_> = candidates
            .iter()
            .filter_map(|(_, _, ord_info)| ord_info.as_ref().map(|(_, row_val)| row_val.row()))
            .collect();

        let all_ord_arrays: Option<Vec<ArrayRef>> = if !ord_rows.is_empty() {
            Some(
                self.row_converter
                    .convert_rows(ord_rows)
                    .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?,
            )
        } else {
            None
        };

        // Build column-major ScalarValues and batch-convert all candidates at once.
        let mut column_values: Vec<Vec<ScalarValue>> = (0..self.sort_exprs.len())
            .map(|_| Vec::with_capacity(candidates.len()))
            .collect();

        let mut ord_pos = 0;
        for (batch_idx, row_idx, ord_info) in candidates.iter() {
            for (i, sort_col) in sort_cols.iter().enumerate() {
                let value = if let Some(deferred) = sort_col.deferred {
                    if let Some((seg_ord, _)) = ord_info {
                        // Ordinal survivor: use pre-batched arrays with our sequential counter.
                        let arrays = all_ord_arrays
                            .as_ref()
                            .expect("all_ord_arrays is None for ordinal survivor");
                        let term_ord = arrays[i]
                            .as_any()
                            .downcast_ref::<UInt64Array>()
                            .map(|a| a.value(ord_pos));
                        if let Some(term_ord) = term_ord {
                            let ff_col =
                                self.ffhelper.column(*seg_ord, deferred.canonical.ff_index);
                            match ff_col {
                                FFType::Text(str_col) => {
                                    let mut s = String::new();
                                    if str_col.ord_to_str(term_ord, &mut s).is_ok() {
                                        ScalarValue::Utf8View(Some(s))
                                    } else {
                                        typed_null(sort_col)?
                                    }
                                }
                                FFType::Bytes(bytes_col) => {
                                    let mut b = Vec::new();
                                    if bytes_col.ord_to_bytes(term_ord, &mut b).is_ok() {
                                        ScalarValue::BinaryView(Some(b))
                                    } else {
                                        typed_null(sort_col)?
                                    }
                                }
                                _ => typed_null(sort_col)?,
                            }
                        } else {
                            typed_null(sort_col)?
                        }
                    } else {
                        // NULL ordinal pass-through
                        typed_null(sort_col)?
                    }
                } else {
                    // Non-deferred column: evaluate directly from the batch.
                    let batch = &self.batches[*batch_idx];
                    let val = sort_col.expr.expr.evaluate(batch)?;
                    let arr = val.into_array(batch.num_rows())?;
                    ScalarValue::try_from_array(&arr, *row_idx).or_else(|_| typed_null(sort_col))?
                };
                column_values[i].push(value);
            }

            if ord_info.is_some() {
                ord_pos += 1;
            }
        }

        // Batch convert all candidates in a single convert_columns call.
        let arrays: Vec<ArrayRef> = column_values
            .into_iter()
            .map(|col| ScalarValue::iter_to_array(col))
            .collect::<Result<Vec<_>>>()?;
        let converted = mat_row_converter
            .convert_columns(&arrays)
            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
        let mut mat_rows: Vec<(usize, OwnedRow)> = Vec::with_capacity(candidates.len());
        for idx in 0..candidates.len() {
            mat_rows.push((idx, converted.row(idx).owned()));
        }

        // 4. Sort candidates by materialized OwnedRow and take top K.
        mat_rows.sort_by(|a, b| a.1.cmp(&b.1));
        mat_rows.truncate(self.k);

        if mat_rows.is_empty() {
            return Ok(None);
        }

        // 5. Emit a single sorted batch.
        //    Concatenate all buffered batches into one mega-batch, then use
        //    row indices to select and reorder the winners.
        let mut batch_offsets: Vec<usize> = Vec::with_capacity(self.batches.len());
        let mut running = 0usize;
        for batch in &self.batches {
            batch_offsets.push(running);
            running += batch.num_rows();
        }

        let mega_batch = if self.batches.len() == 1 {
            self.batches[0].clone()
        } else {
            concat_batches(&self.schema, &self.batches)
                .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?
        };

        // Compute global row index for each winner.
        let indices: Vec<usize> = mat_rows
            .iter()
            .map(|(candidate_idx, _)| {
                let (batch_idx, row_idx, _) = &candidates[*candidate_idx];
                batch_offsets[*batch_idx] + row_idx
            })
            .collect();

        // Use interleave to reorder columns. interleave expects (array_idx, row_idx)
        // pairs — with a single source array, array_idx is always 0.
        let interleave_indices: Vec<(usize, usize)> = indices.iter().map(|&ri| (0, ri)).collect();

        let mut output_columns = Vec::with_capacity(mega_batch.num_columns());
        for col in mega_batch.columns() {
            let col_refs: Vec<&dyn arrow_array::Array> = vec![col.as_ref()];
            let reordered = arrow_select::interleave::interleave(&col_refs, &interleave_indices)
                .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
            output_columns.push(reordered);
        }

        let mut result = RecordBatch::try_new(self.schema.clone(), output_columns)
            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;

        // Write HOT-corrected real ctids to the absorbed ctid output columns.
        //
        // When SegmentedTopKRule absorbs a VisibilityFilterExec, the batches flowing
        // through come directly from HashJoinExec and therefore still carry packed
        // DocAddresses in the ctid_N columns.  We must resolve them before emitting
        // so downstream JoinScanState can pass a real ctid to fetch_tuple_direct.
        //
        // We use the HOT-corrected values from corrected_lookup (populated by
        // check_rows_visible above via heap_hot_search_buffer) rather than calling
        // materialize_deferred_ctid again.  materialize_deferred_ctid would return
        // the raw index ctid, which is wrong for rows whose heap tuple has been
        // moved by a HOT update: fetch_tuple_direct does NOT follow HOT chains, so
        // it would silently return no data or stale data for those rows.
        if !corrected_lookup.is_empty() {
            let mut columns: Vec<ArrayRef> = result.columns().to_vec();
            for (entry_idx, entry) in self.visibility_entries.iter().enumerate() {
                // Build the corrected-ctid array for the K winners in sorted order.
                // Every winner was visible (invisible rows were filtered out above),
                // so corrected_lookup[entry_idx] must contain an entry for each.
                let corrected: Vec<Option<u64>> = mat_rows
                    .iter()
                    .map(|(candidate_idx, _)| {
                        let (batch_idx, row_idx, _) = &candidates[*candidate_idx];
                        corrected_lookup[entry_idx]
                            .get(&(*batch_idx, *row_idx))
                            .copied()
                    })
                    .collect();
                columns[entry.col_idx] = Arc::new(UInt64Array::from(corrected)) as ArrayRef;
            }
            result = RecordBatch::try_new(self.schema.clone(), columns)
                .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
        }

        Ok(Some(result))
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;

    // Disambiguate the `Array` trait (glob-imported via `super::*` from multiple arrow
    // re-exports) so `array.is_valid`/`len`/`value` resolve unambiguously.
    use arrow_array::Array;
    use std::collections::BTreeSet;

    use crate::index::fast_fields_helper::WhichFastField;
    use crate::index::mvcc::MvccSatisfies;
    use crate::index::reader::index::SearchIndexReader;
    use crate::postgres::rel::PgSearchRelation;
    use crate::query::SearchQueryInput;
    use crate::scan::deferred_encode::{build_state_doc_address, deferred_union_data_type};
    use crate::scan::segmented_topk_exec::DeferredSortColumn;
    use crate::schema::SearchFieldType;

    use arrow_schema::{Field, Schema};
    use datafusion::execution::TaskContext;
    use datafusion::physical_expr::expressions::Column;
    use datafusion::physical_expr::{LexOrdering, PhysicalSortExpr};
    use datafusion::physical_plan::test::TestMemoryExec;
    use datafusion::physical_plan::ExecutionPlan;
    use futures::StreamExt;
    use pgrx::prelude::*;
    use proptest::prelude::*;

    fn setup_test_table() {
        Spi::run(
            r#"
            DROP TABLE IF EXISTS segmented_topk_test;
            CREATE TABLE segmented_topk_test (
                id SERIAL PRIMARY KEY,
                name TEXT,
                sort_col TEXT
            );
            INSERT INTO segmented_topk_test (name, sort_col)
            SELECT 'lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor ' ||
                   'incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis ' ||
                   'nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. ' ||
                   'Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu ' ||
                   'fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in ' ||
                   'culpa qui officia deserunt mollit anim id est laborum.',
                   'val_' || lpad(id::text, 6, '0')
            FROM generate_series(1, 35000) id;
            "#,
        )
        .expect("failed to setup test table");
    }

    #[pg_test]
    fn test_segmented_topk_exec() {
        setup_test_table();

        // Force the single builder to create many segments by artificially restricting memory.
        Spi::run("SET max_parallel_workers = 0;").unwrap();
        Spi::run("SET max_parallel_maintenance_workers = 0;").unwrap();
        Spi::run("SET maintenance_work_mem = '15MB';").unwrap();

        // Create an index with target_segment_count = 4 to guarantee multiple segments.
        Spi::run(
            r#"
            CREATE INDEX segmented_topk_test_idx ON segmented_topk_test 
            USING bm25 (id, name, sort_col) 
            WITH (
                key_field = 'id', 
                target_segment_count = 4, 
                text_fields = '{"sort_col": {"fast": true}}'
            );
            "#,
        )
        .unwrap();

        let index_oid =
            Spi::get_one::<pgrx::pg_sys::Oid>("SELECT 'segmented_topk_test_idx'::regclass;")
                .unwrap()
                .unwrap();
        let index_rel = PgSearchRelation::open(index_oid);

        let reader = SearchIndexReader::open(
            &index_rel,
            SearchQueryInput::All,
            false,
            MvccSatisfies::Snapshot,
        )
        .unwrap();

        assert_eq!(reader.total_segment_count(), 4);

        let fields = vec![
            WhichFastField::Named(
                "sort_col".to_string(),
                SearchFieldType::Text(pgrx::pg_sys::TEXTOID),
            ),
            WhichFastField::Named(
                "id".to_string(),
                SearchFieldType::I64(pgrx::pg_sys::INT4OID),
            ),
        ];
        let ffhelper = Arc::new(crate::index::fast_fields_helper::FFHelper::with_fields(
            &reader, &fields,
        ));

        let schema = Arc::new(Schema::new(vec![
            Field::new("sort_col", deferred_union_data_type(), true),
            Field::new("id", arrow_schema::DataType::Int64, true),
        ]));

        let segment_readers = reader.segment_readers();

        let max_docs_per_segment: Vec<u32> =
            segment_readers.iter().map(|sr| sr.max_doc()).collect();

        // Proptest to pick random subsets of doc_ids from the existing segments
        proptest!(|(
            subset_selector in proptest::collection::vec(
                proptest::collection::vec(any::<bool>(), 0..1000),
                max_docs_per_segment.len()
            )
        )| {
            let mut batches = vec![];
            let mut all_selected_ids = BTreeSet::new();

            for (seg_ord, segment_reader) in segment_readers.iter().enumerate() {
                let max_doc = segment_reader.max_doc();
                let ffr = segment_reader.fast_fields();
                let id_col = ffr.i64("id").expect("id field missing");

                let mut doc_ids = vec![];

                // Use the random boolean selector to pick doc_ids
                let selectors = subset_selector.get(seg_ord);
                for doc_id in 0..max_doc {
                    // Default to selecting the document if we don't have enough booleans
                    let should_select = selectors.and_then(|s| s.get(doc_id as usize)).copied().unwrap_or(true);
                    if should_select {
                        doc_ids.push(doc_id);
                        let val = id_col.first(doc_id).unwrap_or_default();
                        all_selected_ids.insert(val);
                    }
                }

                if doc_ids.is_empty() {
                    continue;
                }

                let name_array = build_state_doc_address(seg_ord as u32, &doc_ids);
                let mut id_builder = arrow_array::builder::Int64Builder::with_capacity(doc_ids.len());

                for doc_id in &doc_ids {
                    let val = id_col.first(*doc_id).unwrap_or_default();
                    id_builder.append_value(val);
                }
                let id_array = Arc::new(id_builder.finish()) as ArrayRef;

                let batch = RecordBatch::try_new(schema.clone(), vec![name_array, id_array]).unwrap();
                batches.push(batch);
            }

            if batches.is_empty() {
                return Ok(());
            }

            let memory_exec = TestMemoryExec::try_new_exec(&[batches], schema.clone(), None).unwrap();

            let sort_exprs = LexOrdering::new(vec![
                PhysicalSortExpr {
                    expr: Arc::new(Column::new("sort_col", 0)),
                    options: datafusion::arrow::compute::SortOptions {
                        descending: false,
                        nulls_first: false,
                    },
                }
            ]).unwrap();

            let deferred_columns = vec![
                DeferredSortColumn {
                    sort_col_idx: 0,
                    canonical: crate::index::fast_fields_helper::CanonicalColumn {
                        indexrelid: index_oid.to_u32(),
                        ff_index: 0,
                    },
                    rebuild: None,
                }
            ];

            let topk_exec = SegmentedTopKExec::new(
                memory_exec,
                sort_exprs,
                deferred_columns,
                ffhelper.clone(),
                10,
                None,
            );

            let task_ctx = Arc::new(TaskContext::default());
            let mut stream = topk_exec.execute(0, task_ctx).unwrap();

            let runtime = tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap();

            let mut result_ids = vec![];

            runtime.block_on(async {
                while let Some(batch) = stream.next().await {
                    let batch = batch.unwrap();
                    let col = batch.column(1); // 'id' column
                    let array = col.as_any().downcast_ref::<arrow_array::Int64Array>().unwrap();
                    for i in 0..array.len() {
                        if array.is_valid(i) {
                            result_ids.push(array.value(i));
                        }
                    }
                }
            });

            let expected_limit = all_selected_ids.len().min(10);
            prop_assert_eq!(result_ids.len(), expected_limit);

            // Because sort_col is lpad(id, 6, '0'), numeric sort matches string sort!
            // We just grab the smallest K items from our BTreeSet.
            let expected_ids: Vec<i64> = all_selected_ids.into_iter().take(expected_limit).collect();
            prop_assert_eq!(result_ids, expected_ids);
        });
    }
}

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

use crate::customscan::aggregatescan::AggregateCSClause;
use crate::customscan::aggregatescan::exec::AggregationResultsRow;
use crate::index::reader::index::SearchIndexManifest;
use crate::postgres::PgSearchRelation;
use crate::postgres::customscan::CustomScanState;
use crate::postgres::customscan::aggregatescan::explain::AggregateParallelism;
use crate::postgres::customscan::aggregatescan::join_targetlist::JoinAggregateTargetList;
use crate::postgres::customscan::aggregatescan::pdb_agg::PdbAggPlan;
use crate::postgres::customscan::aggregatescan::privdat::{DataFusionTopK, FilterExpr};
use crate::postgres::customscan::bitmap_intersection::BitmapExec;
use crate::postgres::customscan::joinscan::build::RelNode;
use crate::postgres::customscan::mpp::glue::MppLaunchTiming;
use crate::postgres::customscan::mpp::launch::MppLifecycle;
use crate::postgres::customscan::projections::{PlaceholderColumn, PlaceholderProjection};
use crate::postgres::customscan::solve_expr::SolvePostgresExpressions;
use crate::postgres::heap::VisibilityStats;
use crate::query::tid_bitmap_stream::BitmapCell;
use std::ptr::NonNull;

use arrow_array::RecordBatch;
use datafusion::physical_plan::SendableRecordBatchStream;
use pgrx::pg_sys;

use super::AggIndexInfo;

#[derive(Default)]
pub enum ExecutionState {
    #[default]
    NotStarted,
    Emitting(std::vec::IntoIter<AggregationResultsRow>),
    Completed,
}

/// State for the DataFusion aggregate execution backend.
pub struct DataFusionAggState {
    /// The join tree.
    pub plan: RelNode,
    /// GROUP BY columns and aggregate functions.
    pub targetlist: JoinAggregateTargetList,
    /// Optional TopK sort+limit pushed down from Postgres.
    pub topk: Option<DataFusionTopK>,
    /// Raw PG Expr pointers from custom_exprs (after setrefs transforms
    /// Var nodes to INDEX_VAR references). Used to translate non-@@@
    /// cross-table predicates at execution time.
    pub custom_exprs: *mut pg_sys::List,
    /// The custom_scan_tlist from the CustomScan node. Used to resolve
    /// INDEX_VAR references in custom_exprs back to original (rti, attno)
    /// pairs during DataFusion expression translation.
    pub custom_scan_tlist: *mut pg_sys::List,
    /// HAVING clause filter applied after aggregation.
    pub having_filter: Option<FilterExpr>,
    /// Tokio runtime for async DataFusion execution.
    pub runtime: Option<tokio::runtime::Runtime>,
    /// The executed physical plan, kept so EXPLAIN ANALYZE can merge the worker metrics that
    /// arrive over the mesh into its display.
    pub physical_plan: Option<std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>>,
    /// DataFusion result stream.
    pub stream: Option<SendableRecordBatchStream>,
    /// Current batch being consumed row-by-row.
    pub current_batch: Option<RecordBatch>,
    /// Row index within current_batch.
    pub batch_row_idx: usize,
    /// Mapping from `group_columns[i]` to its 0-based column index in DataFusion's
    /// output RecordBatch. Needed because DataFusion deduplicates grouping
    /// expressions (e.g. metadata.brand).
    pub group_df_indices: Vec<usize>,
    /// The number of grouping columns in DataFusion's output RecordBatch.
    pub num_group_exprs: usize,
    /// The `pdb.agg()` grouping-set layout, set when the query has any such call.
    pub pdb_plan: Option<PdbAggPlan>,
    /// `HAVING` of a scalar `pdb.agg()` query, judged on the assembled root row.
    pub pdb_root_having: Option<datafusion::logical_expr::Expr>,
    /// Assembled `pdb.agg()` documents for each row of `current_batch`, which then
    /// holds the SQL-level rows only.
    pub pdb_agg_json: Option<Vec<Vec<serde_json::Value>>>,
    /// Where MPP sits in its launch lifecycle for this scan: marked pending at begin, launched
    /// on first exec once the built plan's stages are committed (#5667: the plan comes first;
    /// workers spawn only after it exists). Stays `Inactive` on the serial path.
    /// Applies only when parallel execution is enabled and the query qualifies (binary join +
    /// supported aggregate).
    pub mpp: MppLifecycle,
    /// Captured from PostgreSQL's statement-wide `PlannerGlobal.parallelModeOK`. When false, this
    /// scan may still use DataFusion, but it must never launch MPP producer workers.
    pub parallel_mode_ok: bool,
    /// Per-phase launch timing for `EXPLAIN ANALYZE`'s `MPP Launch` line. Set only when the
    /// query launched distributed.
    pub launch_timing: Option<MppLaunchTiming>,
    /// Set (at most once) by `build_task_context`'s `on_spill` callback the first time the
    /// leader's own local execution spills an operator to disk. Serial queries have no
    /// `ParallelScanState` to record this in, so it's tracked here instead; `shutdown_custom_scan`
    /// reads it directly for the serial case, and ORs it with `ParallelScanState::did_spill()`
    /// for the MPP case, since the leader can spill locally in addition to (or instead of) any
    /// worker.
    pub spilled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// State for projecting wrapped aggregate expressions through Postgres' own
/// `ExecBuildProjectionInfo`.
///
/// When the targetlist contains aggregates wrapped in `FuncExpr` calls, we
/// build a copy of the targetlist with each `FuncExpr`'s aggregate replaced by
/// a placeholder, and project that copy. Before each per-row projection we
/// write the live aggregate values into the placeholder columns.
pub struct WrappedAggregateProjection {
    /// Projection of the targetlist copy, built one time for the scan.
    pub projection: PlaceholderProjection,
    /// The placeholder column and its type, indexed by target entry position
    /// (0-based). `None` for entries without a placeholder.
    pub placeholders: Vec<Option<(PlaceholderColumn, pg_sys::Oid)>>,
}

#[derive(Default)]
pub struct AggregateScanState {
    pub visibility_stats: VisibilityStats,
    pub parallelism: Option<AggregateParallelism>,
    pub state: ExecutionState,
    pub indexrelid: pg_sys::Oid,
    pub indexrel: Option<(pg_sys::LOCKMODE, PgSearchRelation)>,
    pub execution_rti: pg_sys::Index,
    pub aggregate_clause: AggregateCSClause,
    pub base_aggregate_clause: Option<AggregateCSClause>,
    /// Where the Tantivy path solves `aggregate_clause`; `None` when it has nothing to solve.
    /// Separate from `ps_ExprContext`, which wrapped-aggregate projection resets per row: see the
    /// rules in `solve_expr.rs`. Unused on the DataFusion path.
    pub runtime_context: Option<NonNull<pg_sys::ExprContext>>,

    /// Execution state for the child bitmap scan, if a bitmap intersection source was
    /// harvested at plan time.
    pub bitmap_exec: Option<BitmapExec>,
    pub bitmap_cell: Option<BitmapCell>,

    /// DataFusion backend state. When `Some`, the DataFusion path is active
    /// and the Tantivy-specific fields above are unused.
    pub datafusion_state: Option<DataFusionAggState>,

    /// Wrapped-aggregate projection state. `Some` only when the targetlist
    /// has aggregates inside `FuncExpr` wrappers that need per-row projection.
    pub wrapped_projection: Option<WrappedAggregateProjection>,

    /// Tantivy-only reusable tuple slot for aggregate result rows.
    /// Created once during begin_custom_scan and cleared/reused for each row
    /// to avoid per-row memory allocation and leaks
    pub scan_slot: Option<*mut pg_sys::TupleTableSlot>,

    /// MPP-only: captured source manifests held by the leader. Serves two
    /// purposes (mirrors JoinScan):
    /// 1. Provides segment counts for DSM sizing in `estimate_dsm_custom_scan`
    ///    and segment readers for DSM population in `initialize_dsm_custom_scan`.
    /// 2. Keeps Tantivy buffer pins alive through `exec_custom_scan` so
    ///    background merges don't recycle the canonical segments before
    ///    workers can open them via `MvccSatisfies::ParallelWorker(ids)`.
    pub source_manifests: Vec<SearchIndexManifest>,

    /// A collection of things needed for result-rewriting decisions that
    /// are expensive to look up.
    precomputed_index_info: Option<AggIndexInfo>,
}

impl AggregateScanState {
    pub fn open_relations(&mut self, lockmode: pg_sys::LOCKMODE) {
        self.indexrel = Some((
            lockmode,
            PgSearchRelation::with_lock(self.indexrelid, lockmode),
        ));
        self.precomputed_index_info = Some(AggIndexInfo::from(self.indexrel()))
    }

    #[inline(always)]
    pub fn indexrel(&self) -> &PgSearchRelation {
        self.indexrel
            .as_ref()
            .map(|(_, rel)| rel)
            .expect("BaseScanState: indexrel should be initialized")
    }

    /// Returns true if the DataFusion backend is active.
    pub fn is_datafusion_backend(&self) -> bool {
        self.datafusion_state.is_some()
    }

    pub fn precomputed_index_info(&self) -> Option<&AggIndexInfo> {
        self.precomputed_index_info.as_ref()
    }
}

impl CustomScanState for AggregateScanState {
    fn init_exec_method(&mut self, _cstate: *mut pg_sys::CustomScanState) {
        // TODO: Unused currently. See the comment on `trait CustomScanState` regarding making this
        // more useful.
    }
}

/// Only the Tantivy path solves through this trait: `begin_custom_scan` and `exec_custom_scan`
/// take the DataFusion branch before reaching it, and that path solves each source's query in
/// `PgSearchTableProvider::scan` instead (see `solve_expr.rs`).
impl SolvePostgresExpressions for AggregateScanState {
    fn has_postgres_expressions(&mut self) -> bool {
        self.aggregate_clause.query_mut().has_postgres_expressions()
            || self
                .aggregate_clause
                .aggregates_mut()
                .any(|agg| agg.has_postgres_expressions())
    }

    fn has_parameters(&mut self) -> bool {
        self.aggregate_clause.query_mut().has_parameters()
            || self
                .aggregate_clause
                .aggregates_mut()
                .any(|agg| agg.has_parameters())
    }

    fn init_search_query_input(&mut self) {
        if let Some(base) = &self.base_aggregate_clause {
            self.aggregate_clause = base.clone();
        }
    }

    /// The aggregate's leader builds and streams privately; its MPP workers do
    /// not receive a cell yet and evaluate filters directly (correct, unpruned).
    fn bitmap_source_cell(&mut self, _planstate: *mut pg_sys::PlanState) -> Option<BitmapCell> {
        self.bitmap_exec.as_ref()?;
        // The cell is filled in `execute_aggregate`, which knows whether the
        // build must be private (count fast path) or shared (worker pool) —
        // building privately here would be thrown away by the shared rebuild.
        Some(
            self.bitmap_cell
                .get_or_insert_with(BitmapCell::default)
                .clone(),
        )
    }

    fn attach_bitmap_cell(&mut self, cell: &BitmapCell) {
        self.aggregate_clause.query_mut().attach_bitmap_cell(cell);
    }

    fn init_postgres_expressions(&mut self, planstate: *mut pg_sys::PlanState) {
        self.aggregate_clause
            .query_mut()
            .init_postgres_expressions(planstate);
        self.aggregate_clause
            .aggregates_mut()
            .for_each(|agg| agg.init_postgres_expressions(planstate));
    }

    /// Resets `expr_context` once, then solves the query and every `FILTER` without resetting
    /// (rule 2 in `solve_expr.rs`): a reset per query would free the trees solved before it.
    fn solve_postgres_expressions(&mut self, expr_context: *mut pg_sys::ExprContext) {
        assert!(
            !expr_context.is_null(),
            "expr_context was never initialized"
        );
        unsafe { pg_sys::MemoryContextReset((*expr_context).ecxt_per_tuple_memory) };
        self.aggregate_clause
            .query_mut()
            .solve_postgres_expressions_no_reset(expr_context);
        self.aggregate_clause
            .aggregates_mut()
            .for_each(|agg| agg.solve_postgres_expressions(expr_context));
    }
}

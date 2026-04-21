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

use crate::customscan::aggregatescan::exec::AggregationResultsRow;
use crate::customscan::aggregatescan::AggregateCSClause;
use crate::postgres::customscan::aggregatescan::join_targetlist::JoinAggregateTargetList;
use crate::postgres::customscan::aggregatescan::privdat::{DataFusionTopK, FilterExpr};
use crate::postgres::customscan::joinscan::build::{
    JoinLevelSearchPredicate, MultiTablePredicateInfo, RelNode,
};
use crate::postgres::customscan::solve_expr::SolvePostgresExpressions;
use crate::postgres::customscan::CustomScanState;
use crate::postgres::PgSearchRelation;

use arrow_array::RecordBatch;
use datafusion::physical_plan::SendableRecordBatchStream;
use pgrx::pg_sys;

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
    /// Cross-table search predicates for join-level filtering.
    pub join_level_predicates: Vec<JoinLevelSearchPredicate>,
    /// Non-@@@ cross-table predicates (descriptions for EXPLAIN).
    pub multi_table_predicates: Vec<MultiTablePredicateInfo>,
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
    /// DataFusion result stream.
    pub stream: Option<SendableRecordBatchStream>,
    /// Current batch being consumed row-by-row.
    pub current_batch: Option<RecordBatch>,
    /// Row index within current_batch.
    pub batch_row_idx: usize,
}

/// State for projecting wrapped aggregate expressions through Postgres' own
/// `ExecBuildProjectionInfo`.
///
/// When the targetlist contains aggregates wrapped in `FuncExpr` calls, we
/// build a copy of the targetlist with each `FuncExpr`'s aggregate replaced by
/// a `Const` placeholder. Before each per-row projection we mutate those
/// `Const`s in place with the live aggregate values, so the compiled projection
/// bakes in the current row's values. This follows the basescan pattern.
///
/// The `const_nodes` pointers alias into `targetlist`'s memory context — if
/// the targetlist is freed or replaced, the const pointers become dangling.
/// Bundling both into one struct keeps the lifetime invariant type-level so
/// neither half can be cleared without the other.
pub struct WrappedAggregateProjection {
    /// Targetlist copy with `Const` placeholders for each wrapped aggregate.
    pub targetlist: *mut pg_sys::List,
    /// Pointers to the `Const` nodes inside `targetlist`, indexed by target
    /// entry position (0-based). `None` for entries without a Const node.
    pub const_nodes: Vec<Option<*mut pg_sys::Const>>,
}

#[derive(Default)]
pub struct AggregateScanState {
    pub state: ExecutionState,
    pub indexrelid: pg_sys::Oid,
    pub indexrel: Option<(pg_sys::LOCKMODE, PgSearchRelation)>,
    pub execution_rti: pg_sys::Index,
    pub aggregate_clause: AggregateCSClause,

    /// DataFusion backend state. When `Some`, the DataFusion path is active
    /// and the Tantivy-specific fields above are unused.
    pub datafusion_state: Option<DataFusionAggState>,

    /// Wrapped-aggregate projection state. `Some` only when the targetlist
    /// has aggregates inside `FuncExpr` wrappers that need per-row projection.
    pub wrapped_projection: Option<WrappedAggregateProjection>,

    /// Reusable tuple slot for aggregate result rows
    /// Created once during begin_custom_scan and cleared/reused for each row
    /// to avoid per-row memory allocation and leaks
    pub scan_slot: Option<*mut pg_sys::TupleTableSlot>,

    /// Serialized `MppPlanBroadcast` bytes (version byte + serialized
    /// DataFusion logical plan + total_participants + session_profile,
    /// bincode-encoded). Populated on the leader in `begin_custom_scan`
    /// when `mpp_is_active()` and the DataFusion backend is active. The
    /// bytes are consumed by `estimate_dsm_custom_scan` (just `.len()`)
    /// and `initialize_dsm_custom_scan` (copied verbatim into DSM for
    /// workers). These are the exact bytes that land in DSM — the
    /// broadcast wrapping is done here, not at DSM-init time, so that
    /// `.len()` at estimate time matches the bytes actually written at
    /// init time (otherwise the DSM write overruns PG's `shm_toc`
    /// allocation and corrupts adjacent DSA control regions).
    ///
    /// `None` means either:
    ///   - MPP is off (the common case),
    ///   - OR the Tantivy backend is active (non-DataFusion path), OR
    ///   - plan serialization failed (logged via `mpp_log!`; MPP will
    ///     silently fall back to the non-MPP path).
    pub logical_plan_bytes: Option<bytes::Bytes>,

    /// Classified MPP shape for this query, populated alongside
    /// `logical_plan_bytes`. Drives the number of shuffle meshes the
    /// coordinator allocates in DSM and the per-shape plan builder used
    /// at exec time. `None` whenever `logical_plan_bytes` is `None`.
    pub mpp_shape: Option<crate::postgres::customscan::mpp::shape::MppPlanShape>,

    /// MPP lifecycle state. Populated by `initialize_dsm_custom_scan`
    /// (leader) or `initialize_worker_custom_scan` (worker) when
    /// `mpp_is_active()` AND the path was flagged parallel-safe. Left
    /// `None` for the serial (non-MPP) code path. Phase 4b-iv will
    /// teach `exec_datafusion_aggregate` to route through
    /// `mpp::plan_build::build_mpp_aggregate_plan` when this field is
    /// `Some`.
    ///
    /// ## Drop ordering
    ///
    /// Declared LAST in the struct so it's the last field dropped. This
    /// matters because `MppExecutionState` owns shm_mq handles pointing
    /// into PG's DSM segment. PG's `ExecEndCustomScan` tears down our
    /// state *before* `ExecParallelCleanup` detaches the DSM segment, so
    /// dropping `mpp_state` while DSM is still mapped is safe. Phase 4b-iv
    /// will also wire a `DrainHandle` into `MppExecutionState::Leader`;
    /// that handle must `join()` its drain thread before the DSM detaches,
    /// so don't move `mpp_state` earlier in this struct without revisiting
    /// the Drop chain.
    pub mpp_state: Option<crate::postgres::customscan::mpp::customscan_glue::MppExecutionState>,
}

impl AggregateScanState {
    pub fn open_relations(&mut self, lockmode: pg_sys::LOCKMODE) {
        self.indexrel = Some((
            lockmode,
            PgSearchRelation::with_lock(self.indexrelid, lockmode),
        ));
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
}

impl CustomScanState for AggregateScanState {
    fn init_exec_method(&mut self, _cstate: *mut pg_sys::CustomScanState) {
        // TODO: Unused currently. See the comment on `trait CustomScanState` regarding making this
        // more useful.
    }
}

impl SolvePostgresExpressions for AggregateScanState {
    fn has_heap_filters(&mut self) -> bool {
        if self.is_datafusion_backend() {
            return false;
        }
        self.aggregate_clause.query_mut().has_heap_filters()
            || self
                .aggregate_clause
                .aggregates_mut()
                .any(|agg| agg.has_heap_filters())
    }

    fn has_postgres_expressions(&mut self) -> bool {
        // Check both the Tantivy-path search queries and DataFusion-path
        // join-level predicates for unresolved PostgresExpression nodes
        // (prepared statement parameters like $1).
        if let Some(ref mut df) = self.datafusion_state {
            if df
                .join_level_predicates
                .iter_mut()
                .any(|p| p.query.has_postgres_expressions())
            {
                return true;
            }
        }
        self.aggregate_clause.query_mut().has_postgres_expressions()
            || self
                .aggregate_clause
                .aggregates_mut()
                .any(|agg| agg.has_postgres_expressions())
    }

    fn init_postgres_expressions(&mut self, planstate: *mut pg_sys::PlanState) {
        if let Some(ref mut df) = self.datafusion_state {
            for pred in &mut df.join_level_predicates {
                pred.query.init_postgres_expressions(planstate);
            }
        }
        self.aggregate_clause
            .query_mut()
            .init_postgres_expressions(planstate);
        self.aggregate_clause
            .aggregates_mut()
            .for_each(|agg| agg.init_postgres_expressions(planstate));
    }

    fn solve_postgres_expressions(&mut self, expr_context: *mut pg_sys::ExprContext) {
        if let Some(ref mut df) = self.datafusion_state {
            for pred in &mut df.join_level_predicates {
                pred.query.solve_postgres_expressions(expr_context);
            }
        }
        if !self.is_datafusion_backend() {
            self.aggregate_clause
                .query_mut()
                .solve_postgres_expressions(expr_context);
            self.aggregate_clause
                .aggregates_mut()
                .for_each(|agg| agg.solve_postgres_expressions(expr_context));
        }
    }
}

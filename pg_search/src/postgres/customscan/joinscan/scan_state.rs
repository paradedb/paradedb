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

//! JoinScan execution state: DataFusion plan construction, optimizer pipeline,
//! and result streaming.
//!
//! See the [JoinScan README](README.md) for the full architecture overview.
//!
//! # Parallel Partitioning Strategy & Correctness
//!
//! `JoinScan` implements parallel execution by **partitioning the first (outermost) table**
//! across workers while **replicating (fully scanning)** all subsequent tables in the join tree.
//!
//! This is equivalent to a "Broadcast Join" or "Fragment-and-Replicate" strategy in distributed
//! databases.
//!
//! ## Correctness
//!
//! This strategy relies on the distributive property of Inner Joins:
//!
//! (A_part1 JOIN B) UNION (A_part2 JOIN B) = (A_part1 UNION A_part2) JOIN B = A JOIN B
//!
//! Each worker computes a partial join result for its subset of `A`. When these partial results
//! are gathered by PostgreSQL, the union forms the complete, correct result set.
//!
//! ## SAFETY WARNING
//!
//! This strategy is partitioning-correct for:
//! 1.  **Inner Joins**: `JOIN_INNER`
//! 2.  **Left Outer Joins** (where the Left/Outer table is partitioned)
//! 3.  **Semi Joins** (where the Left table is partitioned)
//! 4.  **Anti Joins** (where the Left table is partitioned)
//!
//! The current JoinScan planner is more conservative and only enables `INNER`,
//! `SEMI`, and `ANTI` joins. `LEFT` is listed here to document the partitioning
//! constraint for future work, not current planner support.
//!
//! It is **INCORRECT** and will produce duplicate or wrong results for:
//! 1.  **Right Outer Joins**: Unmatched rows from the replicated Right table would be emitted
//!     as `(NULL, b)` by *every* worker, causing duplicates.
//! 2.  **Full Outer Joins**: Same duplicate issue for the replicated side.
//! 3.  **Aggregations**: If DataFusion were performing global aggregations (e.g. `COUNT`),
//!     each worker would emit a partial count, and PostgreSQL's `Gather` would treat them as
//!     distinct rows rather than summing them.
//!
//! **Before enabling any JoinType other than `JOIN_INNER`, you must verify that the partitioning
//! logic in `build_clause_df` respects these constraints.**

use std::sync::Arc;

use datafusion::common::{DataFusionError, Result};
use datafusion::logical_expr::{col, Expr};
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::{ExecutionPlan, ExecutionPlanProperties};
use datafusion::prelude::{DataFrame, SessionConfig, SessionContext};
use futures::future::{FutureExt, LocalBoxFuture};
use pgrx::pg_sys;
use tantivy::index::SegmentId;

use super::planning::get_source_attno_by_name;
use crate::api::{NullTestKind, OrderByFeature, SortDirection};
use crate::index::fast_fields_helper::WhichFastField;
use crate::postgres::customscan::datafusion::memory::create_memory_pool;
use crate::postgres::customscan::joinscan::build::{
    self as build, CtidColumn, JoinCSClause, JoinSource, RelNode, RelationAlias,
};
use crate::postgres::customscan::joinscan::planner::SortMergeJoinEnforcer;
use datafusion::execution::runtime_env::RuntimeEnvBuilder;
use datafusion::execution::TaskContext;
use datafusion::physical_optimizer::filter_pushdown::FilterPushdown;

use crate::index::reader::index::SearchIndexManifest;
use crate::postgres::customscan::datafusion::translator::{
    apply_join_level_filter, build_join_df_with_filter, make_col, make_source_col,
    make_source_score_col, translate_pg_node_string, ColumnMapper, CombinedMapper,
    JoinTypeAllowList, PredicateTranslator,
};
use crate::postgres::customscan::joinscan::privdat::{
    OutputColumnInfo, PrivateData, SCORE_COL_NAME,
};
use crate::postgres::customscan::CustomScanState;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::ParallelScanState;
use crate::scan::{PgSearchTableProvider, VisibilityMode};
use async_trait::async_trait;
use datafusion::execution::context::{QueryPlanner, SessionState};
use datafusion::execution::session_state::SessionStateBuilder;
use datafusion::functions_aggregate::expr_fn::min;
use datafusion::physical_planner::{DefaultPhysicalPlanner, PhysicalPlanner};

/// Resolve a Postgres `(rti, attno)` reference to a DataFusion column expression
/// by walking the join's plan sources and finding the first one that claims it.
///
/// Returns `None` if no source maps the var — the caller decides whether to
/// fall back to a literal or propagate the absence.
fn resolve_var_to_df_col(
    join_clause: &JoinCSClause,
    rti: pg_sys::Index,
    attno: pg_sys::AttrNumber,
) -> Option<Expr> {
    join_clause.plan.sources().iter().find_map(|source| {
        let mapped = source.map_var(rti, attno)?;
        let field = source.column_name(mapped)?;
        Some(make_source_col(source, &field))
    })
}

/// Adapter that lets `PredicateTranslator` resolve Vars against a
/// `JoinCSClause` by delegating to [`resolve_var_to_df_col`].
struct JoinClauseMapper<'a> {
    join_clause: &'a JoinCSClause,
}

impl<'a> ColumnMapper for JoinClauseMapper<'a> {
    fn map_var(&self, varno: pg_sys::Index, varattno: pg_sys::AttrNumber) -> Option<Expr> {
        resolve_var_to_df_col(self.join_clause, varno, varattno)
    }
}

/// Translate a `ChildProjection::Expression` via `PredicateTranslator`.
///
/// Known `pg_catalog` functions (length, upper, abs, etc.) map to native
/// DataFusion functions; unknown functions fall back to `PgExprUdf` via
/// `try_wrap_as_udf`.
unsafe fn translate_child_projection_expr(
    pg_expr_string: &str,
    join_clause: &JoinCSClause,
) -> Result<Expr> {
    let sources = join_clause.plan.sources();
    let mapper = JoinClauseMapper { join_clause };
    translate_pg_node_string(
        pg_expr_string,
        &sources,
        Box::new(mapper),
        "ChildProjection::Expression",
    )
}

/// Query planner that lowers JoinScan's custom logical nodes
/// (`LateMaterializeNode`, `VisibilityFilterNode`) into executable plans.
///
/// JoinScan uses one `SessionContext` configuration for both logical planning
/// and execution. The optimized logical plan is still serialized between those
/// steps so EXPLAIN, the leader, and workers can all reconstruct the same
/// canonical plan. Execution-only bindings are still injected separately during
/// deserialization.
#[derive(Debug, Default)]
pub struct PgSearchQueryPlanner;

#[async_trait]
impl QueryPlanner for PgSearchQueryPlanner {
    async fn create_physical_plan(
        &self,
        logical_plan: &datafusion::logical_expr::LogicalPlan,
        session_state: &SessionState,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let mut extension_planners: Vec<
            Arc<dyn datafusion::physical_planner::ExtensionPlanner + Send + Sync>,
        > = vec![Arc::new(
            crate::scan::late_materialization::LateMaterializePlanner {},
        )];
        extension_planners.push(Arc::new(
            super::visibility_filter::VisibilityExtensionPlanner::new(),
        ));
        let physical_planner = DefaultPhysicalPlanner::with_extension_planners(extension_planners);
        physical_planner
            .create_physical_plan(logical_plan, session_state)
            .await
    }
}

/// Execution state for a single base relation in a join.
pub struct RelationState {
    /// Keeps the relation open and locked during the scan.
    /// The relation is closed/unlocked when this struct is dropped.
    pub _heaprel: PgSearchRelation,
    pub visibility_checker: VisibilityChecker,
    pub fetch_slot: *mut pg_sys::TupleTableSlot,
    /// Index of the CTID column for this relation in the result RecordBatch.
    pub ctid_col_idx: Option<usize>,
}

/// The execution state for the JoinScan.
#[derive(Default)]
pub struct JoinScanState {
    /// The join clause from planning.
    pub join_clause: JoinCSClause,

    /// Map of source index (in plan.sources()) to relation execution state.
    pub relations: crate::api::HashMap<usize, RelationState>,

    /// Result tuple slot.
    pub result_slot: Option<*mut pg_sys::TupleTableSlot>,

    // === DataFusion State ===
    pub datafusion_stream: Option<datafusion::execution::SendableRecordBatchStream>,
    pub runtime: Option<tokio::runtime::Runtime>,
    pub current_batch: Option<arrow_array::RecordBatch>,
    pub batch_index: usize,

    /// Mapping of output column positions to their source (outer/inner) and original attribute numbers.
    /// Populated from PrivateData during create_custom_scan_state.
    pub output_columns: Vec<OutputColumnInfo>,

    /// Serialized DataFusion LogicalPlan from planning phase.
    pub logical_plan: Option<bytes::Bytes>,

    /// Retained executed physical plan for EXPLAIN ANALYZE metrics extraction.
    pub physical_plan: Option<Arc<dyn ExecutionPlan>>,

    /// Shared state for parallel execution.
    /// This is set by either `initialize_dsm_custom_scan` (in the leader) or
    /// `initialize_worker_custom_scan` (in a worker), and then consumed in
    /// `exec_custom_scan` to initialize the DataFusion execution plan.
    pub parallel_state: Option<*mut ParallelScanState>,

    /// Canonical segment ID sets for non-partitioning sources, in the same order the
    /// sources appear in `join_clause.plan.sources()` (partitioning source excluded).
    ///
    /// Populated by `initialize_dsm_custom_scan` (leader) or `initialize_worker_custom_scan`
    /// (worker) and injected during deserialization so that
    /// non-partitioning `PgSearchTableProvider`s open each index with
    /// `MvccSatisfies::ParallelWorker`, ensuring all workers see identical segments.
    pub non_partitioning_segments: Vec<crate::api::HashSet<SegmentId>>,

    /// Captured source manifests held by the leader. Serves two purposes:
    /// 1. Provides segment counts for DSM sizing in `estimate_dsm_custom_scan` and
    ///    segment readers for DSM population in `initialize_dsm_custom_scan`.
    /// 2. Keeps the underlying Tantivy buffer pins alive for the full duration of the
    ///    scan, preventing background merges from recycling the canonical segments.
    ///
    /// Must live on `JoinScanState` (not as a local in `initialize_dsm`) because the
    /// buffer pins must survive from DSM initialization through `exec_custom_scan`,
    /// where workers reopen the same segments via `MvccSatisfies::ParallelWorker(ids)`.
    /// Dropping manifests early would release the pins and allow segment recycling
    /// before workers can open them.
    pub source_manifests: Vec<SearchIndexManifest>,
}

impl JoinScanState {
    /// Reset the scan state for a rescan.
    pub fn reset(&mut self) {
        self.datafusion_stream = None;
        self.current_batch = None;
        self.batch_index = 0;
    }
}

impl CustomScanState for JoinScanState {
    fn init_exec_method(&mut self, _cstate: *mut pg_sys::CustomScanState) {
        // No special initialization needed for the plain exec method
    }
}

/// Selects which physical optimizer rules to install on top of the shared
/// base session for a given consumer.
///
/// JoinScan needs `SegmentedTopKRule` and the trailing `FilterPushdown` passes
/// that follow it; AggregateScan needs only a single `FilterPushdown` post-pass.
/// Exposing the difference as an explicit profile keeps the choice grep-able
/// from the call site.
#[derive(Copy, Clone, Debug)]
pub enum SessionContextProfile {
    /// JoinScan execution: enables `topk_dynamic_filter_pushdown`, includes
    /// `SegmentedTopKRule`, and adds the trailing `FilterPushdown` passes that
    /// `SegmentedTopKRule` requires.
    Join,
    /// AggregateScan execution: single `FilterPushdown` post-pass, no
    /// SegmentedTopK, no topk dynamic filter pushdown.
    Aggregate,
}

/// Build the shared core of a DataFusion [`SessionStateBuilder`] with:
/// - Visibility filtering (logical + physical)
/// - Late materialization
/// - SortMergeJoinEnforcer (when columnar sort enabled)
/// - `PgSearchQueryPlanner`
///
/// Callers append their own TopK rule and FilterPushdown passes.
pub fn build_base_session(config: SessionConfig) -> SessionStateBuilder {
    use super::visibility_filter::VisibilityFilterOptimizerRule;
    use crate::scan::visibility_ctid_resolver_rule::VisibilityCtidResolverRule;

    let mut builder = SessionStateBuilder::new()
        .with_config(config)
        .with_default_features();

    // Inject visibility before late materialization so ctid lineage is analyzed
    // while DeferredCtid columns are still present in the logical plan.
    builder = builder
        .with_optimizer_rule(Arc::new(VisibilityFilterOptimizerRule::new()))
        .with_optimizer_rule(Arc::new(
            crate::scan::late_materialization::LateMaterializationRule,
        ));

    if crate::gucs::is_columnar_sort_enabled() {
        builder = builder.with_physical_optimizer_rule(Arc::new(SortMergeJoinEnforcer::new()));
    }

    builder = builder.with_query_planner(Arc::new(PgSearchQueryPlanner));

    builder.with_physical_optimizer_rule(Arc::new(VisibilityCtidResolverRule))
}

/// Creates a DataFusion [`SessionContext`] for either JoinScan or AggregateScan.
///
/// The base session (visibility filtering, late materialization, SortMergeJoin
/// enforcement, the `PgSearchQueryPlanner`, and the visibility-ctid resolver)
/// is shared via [`build_base_session`]. The supplied [`SessionContextProfile`]
/// then layers on the physical optimizer rules each consumer needs:
///
/// - [`SessionContextProfile::Join`]: enables `topk_dynamic_filter_pushdown`,
///   conditionally injects an early `FilterPushdown` post-pass when columnar
///   sort is on (so dynamic filters reconnect after SortMergeJoin rewrites),
///   then appends `SegmentedTopKRule` followed by a trailing `FilterPushdown`
///   pass to pick up any filters `SegmentedTopKRule` injects.
/// - [`SessionContextProfile::Aggregate`]: appends a single `FilterPushdown`
///   post-pass; SegmentedTopK does not apply to aggregate-on-join queries.
pub fn create_datafusion_session_context(profile: SessionContextProfile) -> SessionContext {
    let mut config = SessionConfig::new().with_target_partitions(1);

    // Configure dynamic filter pushdown thresholds from our GUCs
    config
        .options_mut()
        .optimizer
        .hash_join_inlist_pushdown_max_size =
        crate::gucs::hash_join_inlist_pushdown_max_size() as usize;
    config
        .options_mut()
        .optimizer
        .hash_join_inlist_pushdown_max_distinct_values =
        crate::gucs::hash_join_inlist_pushdown_max_distinct_values() as usize;

    if matches!(profile, SessionContextProfile::Join) {
        config
            .options_mut()
            .optimizer
            .enable_topk_dynamic_filter_pushdown = true;
    }

    let mut builder = build_base_session(config);

    match profile {
        SessionContextProfile::Join => {
            if crate::gucs::is_columnar_sort_enabled() {
                builder = builder.with_physical_optimizer_rule(Arc::new(
                    FilterPushdown::new_post_optimization(),
                ));
            }
            builder = builder
                .with_physical_optimizer_rule(Arc::new(
                    crate::scan::segmented_topk_rule::SegmentedTopKRule,
                ))
                .with_physical_optimizer_rule(Arc::new(FilterPushdown::new_post_optimization()));
        }
        SessionContextProfile::Aggregate => {
            builder = builder
                .with_physical_optimizer_rule(Arc::new(FilterPushdown::new_post_optimization()));
        }
    }

    SessionContext::new_with_state(builder.build())
}

/// Build the DataFusion logical plan for the join.
/// Returns a LogicalPlan that can be serialized with datafusion_proto.
pub async fn build_joinscan_logical_plan(
    join_clause: &JoinCSClause,
    private_data: &PrivateData,
    custom_exprs: *mut pg_sys::List,
) -> Result<datafusion::logical_expr::LogicalPlan> {
    let ctx = create_datafusion_session_context(SessionContextProfile::Join);
    let df = build_clause_df(&ctx, join_clause, private_data, custom_exprs).await?;
    df.into_optimized_plan()
}

/// Convert a LogicalPlan to an ExecutionPlan.
///
/// The input logical plan is already fully optimized (visibility + late materialization
/// nodes injected at planning time). Physical planning reuses the shared
/// `SessionContext` configuration and lowers the stored plan after execution
/// has injected whatever runtime-only bindings are required during decode.
/// Register a [`PgSearchTableProvider`] under `alias` and return the resulting
/// [`DataFrame`].
///
/// Wraps the provider in an `Arc`, registers it on `ctx`, and awaits
/// `ctx.table(alias)`. Callers must finish configuring the provider
/// (deferred outputs, non-partitioning index, etc.) before handing it in;
/// this helper does not select or alias any columns.
///
/// Shared by JoinScan and AggregateScan `build_source_df` implementations.
pub async fn register_source_table(
    ctx: &SessionContext,
    alias: &str,
    provider: crate::scan::PgSearchTableProvider,
) -> Result<DataFrame> {
    let provider = Arc::new(provider);
    ctx.register_table(alias, provider)?;
    ctx.table(alias).await
}

/// Build a DataFusion physical plan from a logical plan.
///
/// Uses the session context's query planner and wraps multi-partition
/// output with `CoalescePartitionsExec`. Shared by JoinScan and AggregateScan.
pub async fn build_physical_plan(
    ctx: &SessionContext,
    plan: datafusion::logical_expr::LogicalPlan,
) -> Result<Arc<dyn ExecutionPlan>> {
    let state = ctx.state();

    let plan = state
        .query_planner()
        .create_physical_plan(&plan, &state)
        .await?;

    if plan.output_partitioning().partition_count() > 1 {
        Ok(Arc::new(CoalescePartitionsExec::new(plan)) as Arc<dyn ExecutionPlan>)
    } else {
        Ok(plan)
    }
}

/// Build the `TaskContext` used to execute a DataFusion physical plan.
///
/// Sizes a `PanicOnOOMMemoryPool` against the supplied `work_mem_bytes` and
/// `hash_mem_multiplier` (typically PostgreSQL's `work_mem * 1024` and
/// `hash_mem_multiplier` GUCs), bundles it into a fresh `RuntimeEnv`, and
/// pairs that with the session config from `ctx`.
///
/// Shared by JoinScan and AggregateScan; both call this immediately before
/// `physical_plan.execute(0, task_ctx)`.
pub fn build_task_context(
    ctx: &SessionContext,
    plan: &Arc<dyn ExecutionPlan>,
    work_mem_bytes: usize,
    hash_mem_multiplier: f64,
) -> Arc<TaskContext> {
    let memory_pool = create_memory_pool(plan, work_mem_bytes, hash_mem_multiplier);
    Arc::new(
        TaskContext::default()
            .with_session_config(ctx.state().config().clone())
            .with_runtime(Arc::new(
                RuntimeEnvBuilder::new()
                    .with_memory_pool(memory_pool)
                    .build()
                    .expect("Failed to create RuntimeEnv"),
            )),
    )
}

/// Context borrowed for the duration of a [`build_relnode_df`] traversal.
///
/// Bundles the references that don't change between recursive calls so the
/// recursion sites stay readable. Construct one at the entry point in
/// `build_clause_df` and pass it down by reference.
struct RelNodeBuildCtx<'a> {
    ctx: &'a SessionContext,
    partitioning_plan_position: usize,
    join_clause: &'a JoinCSClause,
    translated_exprs: &'a [Expr],
    ctid_map: &'a crate::api::HashMap<pg_sys::Index, Expr>,
    output_columns: &'a [OutputColumnInfo],
}

/// Recursively lowers a `RelNode` tree into a DataFusion `DataFrame`.
///
/// This traversal maps the abstract relation operators (Scan, Join, Filter) onto DataFusion's
/// logical planning APIs:
/// - **Scan**: Instantiates a `PgSearchTableProvider` containing the Tantivy index boundaries and
///   the set of required fields for a single relation, wrapping it in an aliased context.
/// - **Join**: Recursively executes left/right sub-trees, collecting separated `equi_keys` and
///   dynamically ensuring `Expr::eq(Expr)` assignments map left-bound columns to the left side
///   of the equality expression to avoid `SchemaError`s in DataFusion.
/// - **Filter**: Maps complex, cross-table PostgreSQL scalar expressions down to the DataFusion
///   engine using a pre-constructed `ctid_map` for row-level execution.
///
/// All references that don't change between recursive calls are bundled into
/// [`RelNodeBuildCtx`] so the recursive sites can stay terse.
fn build_relnode_df<'a>(
    rctx: &'a RelNodeBuildCtx<'a>,
    node: &'a RelNode,
) -> LocalBoxFuture<'a, Result<DataFrame>> {
    let f = async move {
        match node {
            RelNode::Scan(source) => {
                let plan_position = source.plan_position;
                // Use plan_position (globally unique) instead of heap_rti to identify
                // the partitioning source. heap_rti values are local to each
                // PlannerInfo and can collide when SubPlan-extracted sources (e.g.
                // from NOT IN subqueries) share the same range-table index as the
                // outer table.
                //
                // Invariant: plan_position is assigned as the DFS enumeration
                // index in JoinCSClause::new (build.rs), and
                // partitioning_source_index() returns the index into the same
                // DFS-ordered sources() array. Both sources() and sources_mut()
                // maintain left-first DFS order via collect_sources /
                // collect_sources_mut, so plan_position == partitioning_plan_position
                // iff this source is the chosen partitioning source.
                let is_parallel = plan_position == rctx.partitioning_plan_position;

                // Compute the position of this source among non-partitioning sources so execution
                // can retrieve the correct canonical segment IDs during decode.
                let np_idx = if !is_parallel {
                    let partitioning_plan_idx = rctx.join_clause.partitioning_source_index();
                    // Count non-partitioning sources that appear before this one in plan order.
                    let np_pos = rctx
                        .join_clause
                        .plan
                        .sources()
                        .iter()
                        .enumerate()
                        .take(plan_position)
                        .filter(|(i, _)| *i != partitioning_plan_idx)
                        .count();
                    Some(np_pos)
                } else {
                    None
                };

                let mut df = build_source_df(
                    rctx.ctx,
                    source,
                    plan_position,
                    rctx.join_clause,
                    is_parallel,
                    np_idx,
                )
                .await?;
                let alias =
                    RelationAlias::new(source.scan_info.alias.as_deref()).execution(plan_position);
                df = df.alias(&alias)?;
                Ok(df)
            }
            RelNode::Join(join) => {
                let left_df = build_relnode_df(rctx, &join.left).await?;
                let right_df = build_relnode_df(rctx, &join.right).await?;
                let mut sources = join.left.sources();
                sources.extend(join.right.sources());
                build_join_df_with_filter(
                    left_df,
                    right_df,
                    join,
                    &sources,
                    rctx.output_columns,
                    JoinTypeAllowList::All,
                )
            }
            RelNode::Filter(filter) => {
                let df = build_relnode_df(rctx, &filter.input).await?;

                // Compute per-plan_position deferred visibility. A plan_position's
                // ctid is "deferred" (packed DocAddress) if it flows through inner
                // joins or the preserved side of a left/right/semi/anti join from the leaf
                // scan. Other non-inner joins (full, etc.) trigger per-child
                // visibility barriers that resolve ctids to real heap TIDs, while
                // left/right/semi/anti joins only force the null-supplying side.
                let deferred_positions =
                    super::visibility_filter::deferred_plan_positions(&filter.input);
                let sources = filter.input.sources();
                apply_join_level_filter(
                    df,
                    &filter.predicate,
                    rctx.translated_exprs,
                    rctx.ctid_map,
                    &rctx.join_clause.join_level_predicates,
                    &deferred_positions,
                    &sources,
                    /* handle_mark = */ true,
                )
            }
        }
    };
    f.boxed_local()
}

/// Maps `(rti, attno)` to a `col_N` alias after a DISTINCT-style GROUP BY
/// rewrite. The score column uses sentinel `attno = 0`. When DISTINCT is not
/// active the map is empty and downstream stages preserve their original
/// qualified column references.
type DistinctColMap = crate::api::HashMap<(pg_sys::Index, pg_sys::AttrNumber), String>;

/// Recursively builds a DataFusion `DataFrame` for a given join clause.
///
/// This function constructs the logical plan for a join by:
/// 1. Building DataFrames for the left (outer) and right (inner) sources.
/// 2. Performing the configured join type on the specified equi-join keys.
/// 3. Applying join-level filters (both search predicates and heap conditions).
/// 4. Applying sorting and limits if specified.
/// 5. Projecting the final output columns as defined by the join's output projection.
fn build_clause_df<'a>(
    ctx: &'a SessionContext,
    join_clause: &'a JoinCSClause,
    private_data: &'a PrivateData,
    custom_exprs: *mut pg_sys::List,
) -> LocalBoxFuture<'a, Result<DataFrame>> {
    let f = async move {
        let plan_sources = join_clause.plan.sources();
        if plan_sources.len() < 2 {
            return Err(DataFusionError::Internal(
                "JoinScan requires at least 2 sources".into(),
            ));
        }

        let partitioning_plan_position = join_clause.partitioning_source_index();

        let mapper = CombinedMapper {
            sources: &plan_sources,
            output_columns: &private_data.output_columns,
        };
        let translator = PredicateTranslator::new(&plan_sources).with_mapper(Box::new(mapper));
        let translated_exprs = unsafe { translate_custom_exprs(&translator, custom_exprs)? };
        // Drop the translator (and its borrow on `plan_sources`) before downstream
        // stages re-borrow `plan_sources` for projection / output assembly.
        drop(translator);

        let mut ctid_map: crate::api::HashMap<pg_sys::Index, Expr> = Default::default();
        for (i, _) in plan_sources.iter().enumerate() {
            let ctid_name = CtidColumn::new(i).to_string();
            ctid_map.insert(i as pg_sys::Index, col(&ctid_name));
        }

        let rctx = RelNodeBuildCtx {
            ctx,
            partitioning_plan_position,
            join_clause,
            translated_exprs: &translated_exprs,
            ctid_map: &ctid_map,
            output_columns: &private_data.output_columns,
        };
        let df = build_relnode_df(&rctx, &join_clause.plan).await?;

        // 4. Apply DISTINCT via GROUP BY
        let (df, distinct_col_map) = apply_distinct_group_by(df, join_clause, &ctid_map)?;

        // 5. Apply Sort
        let df = apply_sort(df, join_clause, &distinct_col_map)?;

        // 6. Apply Limit (only when the value is statically known at planning
        // time). Parameterized LIMIT/OFFSET are injected at execution time in
        // `JoinScan::exec_custom_scan` after `EState` becomes available.
        let df = if let Some(lo) = &join_clause.limit_offset {
            if let Some(fetch) = lo.static_fetch() {
                df.limit(0, Some(fetch))?
            } else {
                df
            }
        } else {
            df
        };

        // 7. Apply Output Projection
        apply_output_projection(df, join_clause, &distinct_col_map, &plan_sources)
    };
    f.boxed_local()
}

/// Translate every clause in `custom_exprs` (a Postgres `List*`) into a
/// DataFusion `Expr` using the provided `PredicateTranslator`.
unsafe fn translate_custom_exprs(
    translator: &PredicateTranslator,
    custom_exprs: *mut pg_sys::List,
) -> Result<Vec<Expr>> {
    use pgrx::PgList;
    let mut translated = Vec::new();
    let expr_list = PgList::<pg_sys::Node>::from_pg(custom_exprs);
    for (i, expr_node) in expr_list.iter_ptr().enumerate() {
        let expr = translator.translate(expr_node).ok_or_else(|| {
            DataFusionError::Internal(format!(
                "Failed to translate custom expression at index {}",
                i
            ))
        })?;
        translated.push(expr);
    }
    Ok(translated)
}

/// Apply a DISTINCT rewrite as `GROUP BY` over `output_projection`, taking the
/// MIN of each ctid column as a stable representative. Returns the rewritten
/// `DataFrame` plus the populated [`DistinctColMap`] used by the sort and
/// projection stages to resolve column references against the new aliases.
///
/// When DISTINCT is not active (or there is no `output_projection`) the input
/// frame is returned unchanged with an empty map.
fn apply_distinct_group_by(
    df: DataFrame,
    join_clause: &JoinCSClause,
    ctid_map: &crate::api::HashMap<pg_sys::Index, Expr>,
) -> Result<(DataFrame, DistinctColMap)> {
    let mut distinct_col_map: DistinctColMap = Default::default();

    if !join_clause.has_distinct {
        return Ok((df, distinct_col_map));
    }
    let Some(projection) = &join_clause.output_projection else {
        return Ok((df, distinct_col_map));
    };

    let mut group_exprs: Vec<Expr> = Vec::new();

    for (i, proj) in projection.iter().enumerate() {
        let col_alias = format!("col_{}", i + 1);

        let (expr, map_key) = match proj {
            build::ChildProjection::Expression { pg_expr_string, .. } => {
                let e = unsafe { translate_child_projection_expr(pg_expr_string, join_clause)? };
                // Expressions don't participate in sort-step column mapping
                (e, None)
            }
            build::ChildProjection::Score { rti } => {
                let e = build_projection_expr(proj, join_clause);
                (e, Some((*rti, 0)))
            }
            build::ChildProjection::Column { rti, attno }
            | build::ChildProjection::IndexedExpression { rti, attno } => {
                let e = build_projection_expr(proj, join_clause);
                (e, Some((*rti, *attno)))
            }
        };

        group_exprs.push(expr.alias(&col_alias));

        if let Some(key) = map_key {
            distinct_col_map.insert(key, col_alias);
        }
    }

    let agg_exprs: Vec<Expr> = ctid_map
        .values()
        .map(|expr| {
            let ctid_name = match expr {
                Expr::Column(col) => col.name.clone(),
                _ => unreachable!("ctid_map always contains Column expressions"),
            };
            min(expr.clone()).alias(&ctid_name)
        })
        .collect();

    let df = df.aggregate(group_exprs, agg_exprs)?;
    Ok((df, distinct_col_map))
}

/// Resolve a column reference after the DISTINCT GROUP BY has rewritten every
/// projection into a `col_N` alias. Score lookups iterate the map (rather than
/// exact-match) because the parse-time `rti` may not survive cross-table OR
/// predicate handling. When `distinct_col_map` is empty (DISTINCT not active),
/// callers should not invoke this — `col_alias` is returned only as a fallback
/// in case the requested key is missing.
fn resolve_distinct_col(
    distinct_col_map: &DistinctColMap,
    is_score: bool,
    rti: pg_sys::Index,
    attno: pg_sys::AttrNumber,
    col_alias: &str,
) -> Expr {
    if is_score {
        distinct_col_map
            .iter()
            .find(|((_, a), _)| *a == 0)
            .map(|(_, alias)| col(alias.as_str()))
            .unwrap_or_else(|| col(col_alias))
    } else {
        distinct_col_map
            .get(&(rti, attno))
            .map(|alias| col(alias.as_str()))
            .unwrap_or_else(|| col(col_alias))
    }
}

/// Resolve a non-NullTest `OrderByFeature` to a DataFusion `Expr`.
fn resolve_orderby_feature(
    feature: &OrderByFeature,
    join_clause: &JoinCSClause,
    distinct_col_map: &DistinctColMap,
) -> Expr {
    match feature {
        OrderByFeature::Score { rti } => {
            if !distinct_col_map.is_empty() {
                resolve_distinct_col(distinct_col_map, true, 0, 0, "")
            } else {
                join_clause
                    .plan
                    .sources()
                    .iter()
                    .find(|s| s.scan_info.heap_rti == *rti)
                    .map(|source| make_source_score_col(source))
                    .unwrap_or_else(|| col("unknown_score"))
            }
        }
        OrderByFeature::Field { name, rti } => join_clause
            .plan
            .sources()
            .iter()
            .find(|s| s.contains_rti(*rti))
            .map(|source| make_source_col(source, name.as_ref()))
            .unwrap_or_else(|| {
                pgrx::warning!("JoinScan: could not find source for RTI {rti} when building sort expression for field '{name}'");
                col(name.as_ref())
            }),
        OrderByFeature::Var { rti, attno, .. } => {
            if !distinct_col_map.is_empty() {
                resolve_distinct_col(distinct_col_map, false, *rti, *attno, "")
            } else {
                resolve_var_to_df_col(join_clause, *rti, *attno)
                    .unwrap_or_else(|| col("unknown_col"))
            }
        }
        OrderByFeature::NullTest { .. } => {
            unreachable!("NullTest is handled by apply_sort directly")
        }
    }
}

/// Apply the join clause's `ORDER BY` to the data frame, choosing column
/// references from `distinct_col_map` when DISTINCT is active and from the
/// per-source resolution paths otherwise.
fn apply_sort(
    df: DataFrame,
    join_clause: &JoinCSClause,
    distinct_col_map: &DistinctColMap,
) -> Result<DataFrame> {
    if join_clause.order_by.is_empty() {
        return Ok(df);
    }

    let mut sort_exprs = Vec::new();
    for info in &join_clause.order_by {
        let expr = match &info.feature {
            OrderByFeature::NullTest {
                inner,
                nulltesttype,
            } => {
                let inner_expr = resolve_orderby_feature(inner, join_clause, distinct_col_map);
                match nulltesttype {
                    NullTestKind::IsNull => inner_expr.is_null(),
                    NullTestKind::IsNotNull => inner_expr.is_not_null(),
                }
            }
            other => resolve_orderby_feature(other, join_clause, distinct_col_map),
        };

        let asc = matches!(
            info.direction,
            SortDirection::AscNullsFirst | SortDirection::AscNullsLast
        );
        let nulls_first = matches!(
            info.direction,
            SortDirection::AscNullsFirst | SortDirection::DescNullsFirst
        );
        sort_exprs.push(expr.sort(asc, nulls_first));
    }
    df.sort(sort_exprs)
}

/// Build the final SELECT list. When `output_projection` is set, every
/// projected column is aliased to `col_{i+1}` (the convention the result
/// builder expects), and any CTID columns still present in the schema are
/// carried forward unchanged. Without an `output_projection`, the entire
/// schema is selected as-is.
fn apply_output_projection(
    df: DataFrame,
    join_clause: &JoinCSClause,
    distinct_col_map: &DistinctColMap,
    plan_sources: &[&JoinSource],
) -> Result<DataFrame> {
    let mut final_cols = Vec::new();

    if let Some(projection) = &join_clause.output_projection {
        for (i, proj) in projection.iter().enumerate() {
            let col_alias = format!("col_{}", i + 1);
            let expr = if !distinct_col_map.is_empty() {
                match proj {
                    build::ChildProjection::Expression { .. } => col(&col_alias),
                    build::ChildProjection::Score { rti } => {
                        resolve_distinct_col(distinct_col_map, true, *rti, 0, &col_alias)
                    }
                    build::ChildProjection::Column { rti, attno }
                    | build::ChildProjection::IndexedExpression { rti, attno } => {
                        resolve_distinct_col(distinct_col_map, false, *rti, *attno, &col_alias)
                    }
                }
            } else {
                build_projection_expr(proj, join_clause)
            };
            final_cols.push(expr.alias(col_alias));
        }

        // ALWAYS carry forward all CTID columns from both sides
        for (i, _) in plan_sources.iter().enumerate() {
            let ctid_name = CtidColumn::new(i).to_string();
            if df.schema().field_with_unqualified_name(&ctid_name).is_ok() {
                final_cols.push(col(&ctid_name));
            }
        }
    } else {
        for field in df.schema().fields() {
            final_cols.push(col(field.name()));
        }
    }

    df.select(final_cols)
}

/// Builds a DataFusion projection expression for a given child projection info.
///
/// This maps a `ChildProjection` (referencing an RTI and attribute number) to a DataFusion
/// column expression, taking into account aliases and special columns like scores.
fn build_projection_expr(
    proj: &crate::postgres::customscan::joinscan::build::ChildProjection,
    join_clause: &JoinCSClause,
) -> Expr {
    use crate::postgres::customscan::joinscan::build::ChildProjection;

    let plan_sources = join_clause.plan.sources();
    match proj {
        ChildProjection::Score { rti } => {
            for source in plan_sources.iter() {
                if let Some(attno) = source.map_var(*rti, 0) {
                    if let Some(name) = source.column_name(attno) {
                        return make_source_col(source, &name);
                    } else {
                        return make_source_score_col(source);
                    }
                } else if source.contains_rti(*rti) {
                    return make_source_score_col(source);
                }
            }
        }
        ChildProjection::Column { rti, attno }
        | ChildProjection::IndexedExpression { rti, attno } => {
            if let Some(expr) = resolve_var_to_df_col(join_clause, *rti, *attno) {
                return expr;
            }
        }
        ChildProjection::Expression { .. } => {
            unreachable!(
                "Expression projections are handled via PgExprUdf in the \
                 GROUP BY path, not through build_projection_expr"
            );
        }
    }
    datafusion::logical_expr::lit(datafusion::common::ScalarValue::Null)
}

/// Builds a DataFusion `DataFrame` for a given join source.
///
/// If the source is a base relation, it registers a `PgSearchTableProvider` and
/// selects the required fields, aliasing CTID and Score columns as needed.
/// If the source is another join, it recursively calls `build_clause_df`.
fn build_source_df<'a>(
    ctx: &'a SessionContext,
    source: &'a JoinSource,
    plan_position: usize,
    join_clause: &'a JoinCSClause,
    is_parallel: bool,
    np_idx: Option<usize>,
) -> LocalBoxFuture<'a, Result<DataFrame>> {
    async move {
        let scan_info = source.scan_info.clone();
        let alias = RelationAlias::new(source.scan_info.alias.as_deref()).execution(plan_position);
        let fields: Vec<WhichFastField> = source
            .scan_info
            .fields
            .iter()
            .map(|f| f.field.clone())
            .collect();

        /// Insert an ORDER BY field name into `required_early`, plus — when
        /// the same heap attno is registered under a different name — the
        /// registered name too.
        ///
        /// This matters when a column is indexed twice (once aliased, once
        /// as an unaliased expression): the ORDER BY feature carries the
        /// expression name (e.g. "company_name"), but the attno's
        /// schema-registered name may be the alias
        /// (e.g. "company_name_words"). Without also marking the registered
        /// name required-early, the table provider may defer the
        /// alias-named output that downstream DataFusion plans expect
        /// (#4850).
        fn insert_field_name_required_early(
            source: &JoinSource,
            name: &str,
            required_early: &mut crate::api::HashSet<String>,
        ) {
            required_early.insert(name.to_string());
            let attno = unsafe { get_source_attno_by_name(source, name) };
            if let Some(attno) = attno {
                if let Some(registered) = source.column_name(attno) {
                    if registered != name {
                        required_early.insert(registered);
                    }
                }
            }
        }

        let mut required_early: crate::api::HashSet<String> = Default::default();
        for jk in join_clause.plan.join_keys() {
            if source.contains_rti(jk.outer_rti) {
                if let Some(col) = source.column_name(jk.outer_attno) {
                    required_early.insert(col);
                }
            }
            if source.contains_rti(jk.inner_rti) {
                if let Some(col) = source.column_name(jk.inner_attno) {
                    required_early.insert(col);
                }
            }
        }
        // Columns referenced by `JoinNode.filter` (e.g. a disjunctive Semi/Anti
        // `PgExpression`) must also be materialized eagerly — the filter is
        // evaluated per row pair before the join emits anything.
        for (rti, attno) in join_clause.plan.filter_input_vars() {
            if source.contains_rti(rti) {
                if let Some(col) = source.column_name(attno) {
                    required_early.insert(col);
                }
            }
        }

        let mut provider =
            PgSearchTableProvider::new(scan_info.clone(), fields.clone(), is_parallel);

        // Mark non-partitioning sources so execution can retrieve the correct
        // canonical segment IDs during decode.
        if let Some(idx) = np_idx {
            provider.set_non_partitioning_index(idx);
        }

        if let Some(ref sort_order) = scan_info.sort_order {
            required_early.insert(sort_order.field_name.as_ref().to_string());
        }

        // When DISTINCT is present, PostgreSQL expands the query path-keys
        // to include all DISTINCT columns.
        if join_clause.has_distinct {
            for info in &join_clause.order_by {
                match &info.feature {
                    OrderByFeature::Field { name, rti } => {
                        if source.contains_rti(*rti) {
                            insert_field_name_required_early(
                                source,
                                name.as_ref(),
                                &mut required_early,
                            );
                        }
                    }
                    OrderByFeature::Var { rti, attno, .. } => {
                        // Only insert columns belonging to THIS source
                        if source.contains_rti(*rti) {
                            if let Some(col_name) = source.column_name(*attno) {
                                required_early.insert(col_name);
                            }
                        }
                    }
                    OrderByFeature::Score { .. } => {}
                    OrderByFeature::NullTest { inner, .. } => match inner.as_ref() {
                        OrderByFeature::Field { name, rti } if source.contains_rti(*rti) => {
                            insert_field_name_required_early(
                                source,
                                name.as_ref(),
                                &mut required_early,
                            );
                        }
                        OrderByFeature::Var { rti, attno, .. } if source.contains_rti(*rti) => {
                            if let Some(col_name) = source.column_name(*attno) {
                                required_early.insert(col_name);
                            }
                        }
                        _ => {}
                    },
                }
            }
        }

        provider.configure_deferred_outputs(
            &required_early,
            VisibilityMode::Deferred { plan_position },
        );

        let mut df = register_source_table(ctx, alias.as_str(), provider).await?;

        // Select fields AND ensure CTID is aliased uniquely
        let mut exprs = Vec::new();
        for df_field in df.schema().fields().iter() {
            let name = df_field.name();
            // NOTE: Matching on WhichFastField::Ctid specifically will fail if
            // the field list order doesn't match the DataFrame schema field order.
            let expr = match fields.iter().find(|w| w.name() == *name) {
                Some(WhichFastField::Ctid) => {
                    make_col(alias.as_str(), name).alias(CtidColumn::new(plan_position).to_string())
                }
                // Normalize score fast-field column name so all score references resolve
                // through `<execution_alias>.score`.
                Some(WhichFastField::Score) => make_col(alias.as_str(), name).alias(SCORE_COL_NAME),
                _ => make_col(alias.as_str(), name),
            };

            exprs.push(expr);
        }
        df = df.select(exprs)?;

        Ok(df)
    }
    .boxed_local()
}

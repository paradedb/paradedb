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

use crate::api::{OrderByFeature, SortDirection};
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
    apply_join_level_filter, build_join_df, make_col, CombinedMapper, JoinTypeAllowList,
    PredicateTranslator,
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

fn make_source_col(source: &JoinSource, plan_position: usize, field_name: &str) -> Expr {
    let alias = RelationAlias::new(source.scan_info.alias.as_deref()).execution(plan_position);
    make_col(&alias, field_name)
}

fn make_source_score_col(source: &JoinSource, plan_position: usize) -> Expr {
    let alias = RelationAlias::new(source.scan_info.alias.as_deref()).execution(plan_position);
    make_col(&alias, SCORE_COL_NAME)
}

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
    join_clause
        .plan
        .sources()
        .iter()
        .enumerate()
        .find_map(|(idx, source)| {
            let mapped = source.map_var(rti, attno)?;
            let field = source.column_name(mapped)?;
            Some(make_source_col(source, idx, &field))
        })
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

    let mut builder = SessionStateBuilder::new().with_config(config);

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

                let df = build_join_df(left_df, right_df, join, JoinTypeAllowList::All)?;

                if join.filter.is_some() {
                    return Err(DataFusionError::NotImplemented(
                        "Non-equi join filters are not yet implemented".into(),
                    ));
                }

                Ok(df)
            }
            RelNode::Filter(filter) => {
                let df = build_relnode_df(rctx, &filter.input).await?;

                // Compute per-plan_position deferred visibility. A plan_position's
                // ctid is "deferred" (packed DocAddress) if it flows only through
                // inner joins from the leaf scan. Non-inner joins (semi, anti, etc.)
                // trigger per-child visibility barriers that resolve ctids to real
                // heap TIDs. In mixed trees like (A INNER B) INNER (C SEMI D),
                // ctid_A and ctid_B are still packed while ctid_C is resolved.
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

        let plan = &join_clause.plan;

        let partitioning_plan_position = join_clause.partitioning_source_index();

        let mapper = CombinedMapper {
            sources: &plan_sources,
            output_columns: &private_data.output_columns,
        };

        let translator = PredicateTranslator::new(&plan_sources).with_mapper(Box::new(mapper));

        // Translate all custom_exprs first
        let mut translated_exprs = Vec::new();
        unsafe {
            use pgrx::PgList;
            let expr_list = PgList::<pg_sys::Node>::from_pg(custom_exprs);
            for (i, expr_node) in expr_list.iter_ptr().enumerate() {
                let expr = translator.translate(expr_node).ok_or_else(|| {
                    DataFusionError::Internal(format!(
                        "Failed to translate custom expression at index {}",
                        i
                    ))
                })?;
                translated_exprs.push(expr);
            }
        }

        let mut ctid_map = crate::api::HashMap::default();
        for (i, _) in plan_sources.iter().enumerate() {
            let ctid_name = CtidColumn::new(i).to_string();
            let expr = col(&ctid_name);
            ctid_map.insert(i as pg_sys::Index, expr);
        }

        let rctx = RelNodeBuildCtx {
            ctx,
            partitioning_plan_position,
            join_clause,
            translated_exprs: &translated_exprs,
            ctid_map: &ctid_map,
        };
        let mut df = build_relnode_df(&rctx, plan).await?;

        // Maps (rti, attno) → col_N alias, populated only when has_distinct is true.
        // For regular columns: (rti, attno) → col_N
        // For score columns:   (rti, 0)     → col_N  (attno=0 is the score sentinel)
        // When has_distinct is false, map is empty and sort uses existing qualified path.
        let mut distinct_col_map: crate::api::HashMap<(pg_sys::Index, pg_sys::AttrNumber), String> =
            Default::default();

        // 4. Apply DISTINCT via GROUP BY
        if join_clause.has_distinct {
            if let Some(projection) = &join_clause.output_projection {
                let mut group_exprs: Vec<Expr> = Vec::new();

                for (i, proj) in projection.iter().enumerate() {
                    let col_alias = format!("col_{}", i + 1);

                    let (expr, map_key) = match proj {
                        build::ChildProjection::Expression {
                            pg_expr_string,
                            input_vars,
                            result_type_oid,
                            ..
                        } => {
                            let udf_name = format!(
                                "{}{}",
                                crate::postgres::customscan::pg_expr_udf::PG_EXPR_UDF_PREFIX,
                                i
                            );

                            let input_exprs: Vec<Expr> = input_vars
                                .iter()
                                .filter_map(|var_info| {
                                    resolve_var_to_df_col(join_clause, var_info.rti, var_info.attno)
                                })
                                .collect();

                            if input_exprs.len() != input_vars.len() {
                                return Err(DataFusionError::Internal(format!(
                                    "PgExprUdf: could not resolve all input columns \
                                     (resolved {} of {})",
                                    input_exprs.len(),
                                    input_vars.len()
                                )));
                            }

                            let udf = crate::postgres::customscan::pg_expr_udf::PgExprUdf::new(
                                udf_name,
                                pg_expr_string.clone(),
                                input_vars.clone(),
                                *result_type_oid,
                            );

                            let e = Expr::ScalarFunction(
                                datafusion::logical_expr::expr::ScalarFunction::new_udf(
                                    Arc::new(datafusion::logical_expr::ScalarUDF::new_from_impl(
                                        udf,
                                    )),
                                    input_exprs,
                                ),
                            );
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

                df = df.aggregate(group_exprs, agg_exprs)?;
            }
        }

        // Closure to resolve a column reference after GROUP BY.
        // When distinct_col_map is populated, all columns are renamed to col_N.
        // Score uses sentinel (rti, 0) — we iterate rather than exact key match
        // because proj.rti may not match in cross-table OR predicate cases.
        let resolve_distinct_col = |is_score: bool,
                                    rti: pg_sys::Index,
                                    attno: pg_sys::AttrNumber,
                                    col_alias: &str|
         -> Expr {
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
        };

        // 5. Apply Sort
        if !join_clause.order_by.is_empty() {
            let mut sort_exprs = Vec::new();
            for info in &join_clause.order_by {
                let expr = match &info.feature {
                    OrderByFeature::Score { rti } => {
                        if !distinct_col_map.is_empty() {
                            resolve_distinct_col(true, 0, 0, "")
                        } else {
                            // TODO: this heap_rti lookup has the same collision
                            // risk as the partitioning check fixed above — if a
                            // NOT IN subquery source shares the same RTI as the
                            // outer table, the wrong score column could be
                            // selected. Low risk since ORDER BY scores typically
                            // reference outer-query tables. Fix by switching
                            // OrderByFeature::Score to carry plan_position.
                            join_clause
                                .plan
                                .sources()
                                .iter()
                                .enumerate()
                                .find(|(_, s)| s.scan_info.heap_rti == *rti)
                                .map(|(idx, source)| make_source_score_col(source, idx))
                                .unwrap_or_else(|| col("unknown_score"))
                        }
                    }
                    OrderByFeature::Field { name, rti } => join_clause
                        .plan
                        .sources()
                        .iter()
                        .enumerate()
                        .find(|(_, s)| s.contains_rti(*rti))
                        .map(|(idx, source)| make_source_col(source, idx, name.as_ref()))
                        .unwrap_or_else(|| {
                            pgrx::warning!("JoinScan: could not find source for RTI {rti} when building sort expression for field '{name}'");
                            col(name.as_ref())
                        }),
                    OrderByFeature::Var { rti, attno, .. } => {
                        if !distinct_col_map.is_empty() {
                            resolve_distinct_col(false, *rti, *attno, "")
                        } else {
                            resolve_var_to_df_col(join_clause, *rti, *attno)
                                .unwrap_or_else(|| col("unknown_col"))
                        }
                    }
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
            df = df.sort(sort_exprs)?;
        }

        // 6. Apply Limit
        if let Some(fetch) = join_clause.limit_offset.fetch() {
            df = df.limit(0, Some(fetch))?;
        }

        // 7. Apply Output Projection
        let mut final_cols = Vec::new();

        if let Some(projection) = &join_clause.output_projection {
            for (i, proj) in projection.iter().enumerate() {
                let col_alias = format!("col_{}", i + 1);
                let expr = if !distinct_col_map.is_empty() {
                    match proj {
                        build::ChildProjection::Expression { .. } => col(&col_alias),
                        build::ChildProjection::Score { rti } => {
                            resolve_distinct_col(true, *rti, 0, &col_alias)
                        }
                        build::ChildProjection::Column { rti, attno }
                        | build::ChildProjection::IndexedExpression { rti, attno } => {
                            resolve_distinct_col(false, *rti, *attno, &col_alias)
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

        df = df.select(final_cols)?;

        Ok(df)
    };
    f.boxed_local()
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
            for (i, source) in plan_sources.iter().enumerate() {
                if let Some(attno) = source.map_var(*rti, 0) {
                    if let Some(name) = source.column_name(attno) {
                        return make_source_col(source, i, &name);
                    } else {
                        return make_source_score_col(source, i);
                    }
                } else if source.contains_rti(*rti) {
                    return make_source_score_col(source, i);
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
                            required_early.insert(name.as_ref().to_string());
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
                    OrderByFeature::Score { .. } => {
                        // Score is not a late-materialized column, skip
                    }
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

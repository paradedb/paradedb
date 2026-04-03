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

use datafusion::common::{DataFusionError, JoinType, Result};
use datafusion::logical_expr::{col, Expr};
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::{ExecutionPlan, ExecutionPlanProperties};
use datafusion::prelude::{DataFrame, SessionConfig, SessionContext};
use futures::future::{FutureExt, LocalBoxFuture};
use pgrx::pg_sys;
use tantivy::index::SegmentId;

use crate::api::{OrderByFeature, SortDirection};
use crate::index::fast_fields_helper::WhichFastField;
use crate::postgres::customscan::joinscan::build::{
    CtidColumn, JoinCSClause, JoinSource, RelNode, RelationAlias,
};
use crate::postgres::customscan::joinscan::planner::SortMergeJoinEnforcer;
use datafusion::physical_optimizer::filter_pushdown::FilterPushdown;

use crate::index::reader::index::SearchIndexManifest;
use crate::postgres::customscan::joinscan::privdat::{
    OutputColumnInfo, PrivateData, SCORE_COL_NAME,
};
use crate::postgres::customscan::joinscan::translator::{make_col, CombinedMapper};
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

/// Base session config shared by all contexts.
fn base_session_config() -> SessionConfig {
    let mut config = SessionConfig::new().with_target_partitions(1);
    config
        .options_mut()
        .optimizer
        .enable_topk_dynamic_filter_pushdown = true;
    config
}

/// Adds SegmentedTopK plus the final post-optimization FilterPushdown pass.
///
/// This second `FilterPushdown(Post)` run is intentional: `SegmentedTopKRule`
/// can inject new `DynamicFilterPhysicalExpr`s that did not exist during the
/// earlier post-optimization pass.
fn add_tail_physical_rules(builder: SessionStateBuilder) -> SessionStateBuilder {
    builder
        .with_physical_optimizer_rule(Arc::new(
            crate::scan::segmented_topk_rule::SegmentedTopKRule,
        ))
        .with_physical_optimizer_rule(Arc::new(FilterPushdown::new_post_optimization()))
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

/// Creates the shared DataFusion SessionContext used for both JoinScan logical
/// planning and execution.
///
/// The same context configuration is used to:
/// 1. run logical optimization and produce the canonical serialized plan
/// 2. lower that deserialized logical plan into a physical plan at execution
pub fn create_session_context() -> SessionContext {
    let mut builder = build_base_session(base_session_config());

    // VisibilityExtensionPlanner already places visibility below any immediate
    // TantivyLookupExec chain, so only resolver wiring remains here before the
    // first post-optimization FilterPushdown pass. That pass reconnects dynamic
    // filters after SortMergeJoin rewrites; the final pass in
    // `add_tail_physical_rules` handles any new filters introduced by
    // SegmentedTopKRule later in the pipeline.
    if crate::gucs::is_columnar_sort_enabled() {
        builder =
            builder.with_physical_optimizer_rule(Arc::new(FilterPushdown::new_post_optimization()));
    }
    builder = add_tail_physical_rules(builder);

    SessionContext::new_with_state(builder.build())
}

/// Build the DataFusion logical plan for the join.
/// Returns a LogicalPlan that can be serialized with datafusion_proto.
pub async fn build_joinscan_logical_plan(
    join_clause: &JoinCSClause,
    private_data: &PrivateData,
    custom_exprs: *mut pg_sys::List,
) -> Result<datafusion::logical_expr::LogicalPlan> {
    let ctx = create_session_context();
    let df = build_clause_df(&ctx, join_clause, private_data, custom_exprs).await?;
    df.into_optimized_plan()
}

/// Convert a LogicalPlan to an ExecutionPlan.
///
/// The input logical plan is already fully optimized (visibility + late materialization
/// nodes injected at planning time). Physical planning reuses the shared
/// `SessionContext` configuration and lowers the stored plan after execution
/// has injected whatever runtime-only bindings are required during decode.
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
fn build_relnode_df<'a>(
    ctx: &'a SessionContext,
    node: &'a RelNode,
    partitioning_rti: pg_sys::Index,
    join_clause: &'a JoinCSClause,
    translated_exprs: &'a [Expr],
    ctid_map: &'a crate::api::HashMap<pg_sys::Index, Expr>,
) -> LocalBoxFuture<'a, Result<DataFrame>> {
    let f = async move {
        match node {
            RelNode::Scan(source) => {
                let is_parallel = source.scan_info.heap_rti == partitioning_rti;
                let plan_position = source.plan_position;

                // Compute the position of this source among non-partitioning sources so execution
                // can retrieve the correct canonical segment IDs during decode.
                let np_idx = if !is_parallel {
                    let partitioning_plan_idx = join_clause.partitioning_source_index();
                    // Count non-partitioning sources that appear before this one in plan order.
                    let np_pos = join_clause
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

                let mut df =
                    build_source_df(ctx, source, plan_position, join_clause, is_parallel, np_idx)
                        .await?;
                let alias =
                    RelationAlias::new(source.scan_info.alias.as_deref()).execution(plan_position);
                df = df.alias(&alias)?;
                Ok(df)
            }
            RelNode::Join(join) => {
                let left_df = build_relnode_df(
                    ctx,
                    &join.left,
                    partitioning_rti,
                    join_clause,
                    translated_exprs,
                    ctid_map,
                )
                .await?;
                let right_df = build_relnode_df(
                    ctx,
                    &join.right,
                    partitioning_rti,
                    join_clause,
                    translated_exprs,
                    ctid_map,
                )
                .await?;

                let on = super::translator::build_equi_join_exprs(join)?;

                let df_join_type = match join.join_type {
                    crate::postgres::customscan::joinscan::build::JoinType::Inner => {
                        JoinType::Inner
                    }
                    crate::postgres::customscan::joinscan::build::JoinType::Left => JoinType::Left,
                    crate::postgres::customscan::joinscan::build::JoinType::Full => JoinType::Full,
                    crate::postgres::customscan::joinscan::build::JoinType::Right => {
                        JoinType::Right
                    }
                    crate::postgres::customscan::joinscan::build::JoinType::Semi => {
                        JoinType::LeftSemi
                    }
                    crate::postgres::customscan::joinscan::build::JoinType::Anti => {
                        JoinType::LeftAnti
                    }
                    crate::postgres::customscan::joinscan::build::JoinType::LeftMark => {
                        JoinType::LeftMark
                    }
                    crate::postgres::customscan::joinscan::build::JoinType::RightMark => {
                        JoinType::RightMark
                    }
                    crate::postgres::customscan::joinscan::build::JoinType::RightSemi => {
                        JoinType::RightSemi
                    }
                    crate::postgres::customscan::joinscan::build::JoinType::RightAnti => {
                        JoinType::RightAnti
                    }
                    unsupported => {
                        panic!("Join type {} is unsupported during execution", unsupported)
                    }
                };

                let df = if on.is_empty() {
                    left_df.join(right_df, df_join_type, &[], &[], None)?
                } else {
                    left_df.join_on(right_df, df_join_type, on)?
                };

                if join.filter.is_some() {
                    return Err(DataFusionError::NotImplemented(
                        "Non-equi join filters are not yet implemented".into(),
                    ));
                }

                Ok(df)
            }
            RelNode::Filter(filter) => {
                let mut df = build_relnode_df(
                    ctx,
                    &filter.input,
                    partitioning_rti,
                    join_clause,
                    translated_exprs,
                    ctid_map,
                )
                .await?;

                // Compute per-plan_position deferred visibility. A plan_position's
                // ctid is "deferred" (packed DocAddress) if it flows only through
                // inner joins from the leaf scan. Non-inner joins (semi, anti, etc.)
                // trigger per-child visibility barriers that resolve ctids to real
                // heap TIDs. In mixed trees like (A INNER B) INNER (C SEMI D),
                // ctid_A and ctid_B are still packed while ctid_C is resolved.
                let deferred_positions =
                    super::visibility_filter::deferred_plan_positions(&filter.input);
                let sources = filter.input.sources();
                let filter_expr = unsafe {
                    crate::postgres::customscan::joinscan::translator::PredicateTranslator::translate_join_level_expr(
                        &filter.predicate,
                        translated_exprs,
                        ctid_map,
                        &join_clause.join_level_predicates,
                        &deferred_positions,
                        &sources,
                    )
                }
                .ok_or_else(|| {
                    DataFusionError::Internal(format!(
                        "Failed to translate join level expression tree: {:?}",
                        filter.predicate
                    ))
                })?;

                // For MarkOrNull filters, drop the synthetic "mark" column after filtering.
                let is_mark_filter = matches!(
                    filter.predicate,
                    crate::postgres::customscan::joinscan::build::JoinLevelExpr::MarkOrNull { .. }
                );

                df = df.filter(filter_expr)?;

                if is_mark_filter {
                    use datafusion::logical_expr::col;
                    let schema = df.schema().clone();
                    let proj_cols: Vec<datafusion::logical_expr::Expr> = schema
                        .columns()
                        .into_iter()
                        .filter(|c| c.name != "mark")
                        .map(col)
                        .collect();
                    if !proj_cols.is_empty() {
                        df = df.select(proj_cols)?;
                    }
                }
                Ok(df)
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

        let partitioning_rti = join_clause.partitioning_source().scan_info.heap_rti;

        let mapper = CombinedMapper {
            sources: &plan_sources,
            output_columns: &private_data.output_columns,
        };

        let translator =
            crate::postgres::customscan::joinscan::translator::PredicateTranslator::new(
                &plan_sources,
            )
            .with_mapper(Box::new(mapper));

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

        let mut df = build_relnode_df(
            ctx,
            plan,
            partitioning_rti,
            join_clause,
            &translated_exprs,
            &ctid_map,
        )
        .await?;

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

                    let expr = if proj.is_expression() {
                        // Expression-based DISTINCT: create a PgExprUdf call
                        let udf_name = format!("{}{}", super::pg_expr_udf::PG_EXPR_UDF_PREFIX, i);
                        let input_vars = proj.input_vars.as_ref().ok_or_else(|| {
                            DataFusionError::Internal(
                                "PgExprUdf: expression projection missing input_vars".to_string(),
                            )
                        })?;
                        let pg_expr_string = proj.pg_expr_string.as_ref().ok_or_else(|| {
                            DataFusionError::Internal(
                                "PgExprUdf: expression projection missing pg_expr_string"
                                    .to_string(),
                            )
                        })?;
                        let result_type_oid = proj.result_type_oid.unwrap_or(pg_sys::TEXTOID);

                        // Build input column expressions from the DataFusion plan
                        let plan_sources = join_clause.plan.sources();
                        let input_exprs: Vec<Expr> = input_vars
                            .iter()
                            .filter_map(|var_info| {
                                for (idx, source) in plan_sources.iter().enumerate() {
                                    if let Some(attno) =
                                        source.map_var(var_info.rti, var_info.attno)
                                    {
                                        if let Some(field_name) = source.column_name(attno) {
                                            return Some(make_source_col(source, idx, &field_name));
                                        }
                                    }
                                }
                                None
                            })
                            .collect();

                        if input_exprs.len() != input_vars.len() {
                            return Err(DataFusionError::Internal(format!(
                                "PgExprUdf: could not resolve all input columns for expression \
                                 (resolved {} of {})",
                                input_exprs.len(),
                                input_vars.len()
                            )));
                        }

                        let udf = super::pg_expr_udf::PgExprUdf::new(
                            udf_name,
                            pg_expr_string.clone(),
                            input_vars.clone(),
                            result_type_oid,
                        );

                        Expr::ScalarFunction(
                            datafusion::logical_expr::expr::ScalarFunction::new_udf(
                                Arc::new(datafusion::logical_expr::ScalarUDF::from(udf)),
                                input_exprs,
                            ),
                        )
                    } else {
                        // Simple column or score — existing logic
                        build_projection_expr(proj, join_clause)
                    };

                    group_exprs.push(expr.alias(&col_alias));

                    // Record mapping for sort step:
                    // score uses (rti, 0) sentinel, regular columns use (rti, attno)
                    let key = if proj.is_score {
                        (proj.rti, 0)
                    } else {
                        (proj.rti, proj.attno)
                    };
                    distinct_col_map.insert(key, col_alias);
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
                            join_clause
                                .plan
                                .sources()
                                .iter()
                                .enumerate()
                                .find_map(|(i, source)| {
                                    let mapped = source.map_var(*rti, *attno)?;
                                    let field = source.column_name(mapped)?;
                                    Some(make_source_col(source, i, &field))
                                })
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
                    if proj.is_expression() {
                        // Expression columns are already in the GROUP BY output
                        // as col_{i+1} — reference directly.
                        col(&col_alias)
                    } else {
                        resolve_distinct_col(proj.is_score, proj.rti, proj.attno, &col_alias)
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
    let plan_sources = join_clause.plan.sources();
    for (i, source) in plan_sources.iter().enumerate() {
        if proj.is_score {
            if let Some(attno) = source.map_var(proj.rti, 0) {
                if let Some(name) = source.column_name(attno) {
                    return make_source_col(source, i, &name);
                } else {
                    return make_source_score_col(source, i);
                }
            } else if source.contains_rti(proj.rti) {
                return make_source_score_col(source, i);
            }
        } else if let Some(attno) = source.map_var(proj.rti, proj.attno) {
            if let Some(field_name) = source.column_name(attno) {
                return make_source_col(source, i, &field_name);
            }
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

        let provider = Arc::new(provider);
        ctx.register_table(alias.as_str(), provider)?;

        let mut df = ctx.table(alias.as_str()).await?;

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

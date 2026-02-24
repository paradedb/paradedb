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
//! This strategy is **ONLY CORRECT** for:
//! 1.  **Inner Joins**: `JOIN_INNER`
//! 2.  **Left Outer Joins** (where the Left/Outer table is partitioned)
//! 3.  **Semi Joins** (where the Left table is partitioned)
//! 4.  **Anti Joins** (where the Left table is partitioned)
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

use crate::api::{OrderByFeature, SortDirection};
use crate::index::fast_fields_helper::WhichFastField;
use crate::postgres::customscan::joinscan::build::{
    JoinCSClause, JoinSource, JoinType as JoinScanJoinType,
};
use crate::postgres::customscan::joinscan::planner::SortMergeJoinEnforcer;
use datafusion::physical_optimizer::filter_pushdown::FilterPushdown;

use crate::postgres::customscan::joinscan::privdat::{
    OutputColumnInfo, PrivateData, SCORE_COL_NAME,
};
use crate::postgres::customscan::joinscan::translator::{make_col, CombinedMapper};
use crate::postgres::customscan::CustomScanState;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::ParallelScanState;
use crate::scan::PgSearchTableProvider;
use datafusion::execution::session_state::SessionStateBuilder;

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

    /// Map of range table index (RTI) to relation execution state.
    pub relations: crate::api::HashMap<pg_sys::Index, RelationState>,

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

    /// Maximum allowed memory for execution (from work_mem, in bytes).
    pub max_memory: usize,

    /// Serialized DataFusion LogicalPlan from planning phase.
    pub logical_plan: Option<bytes::Bytes>,

    /// Retained executed physical plan for EXPLAIN ANALYZE metrics extraction.
    pub physical_plan: Option<Arc<dyn ExecutionPlan>>,

    /// Shared state for parallel execution.
    /// This is set by either `initialize_dsm_custom_scan` (in the leader) or
    /// `initialize_worker_custom_scan` (in a worker), and then consumed in
    /// `exec_custom_scan` to initialize the DataFusion execution plan.
    pub parallel_state: Option<*mut ParallelScanState>,
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

/// Creates a DataFusion SessionContext with our custom SortMergeJoinEnforcer physical optimizer rule.
///
/// We set `target_partitions = 1` to ensure deterministic EXPLAIN output.
/// The `SortMergeJoinEnforcer` rule runs after the initial execution plan is built
/// and replaces `HashJoinExec` with `SortMergeJoinExec` if the inputs are already sorted
/// in a compatible way.
pub fn create_session_context() -> SessionContext {
    let mut config = SessionConfig::new().with_target_partitions(1);
    config
        .options_mut()
        .optimizer
        .enable_topk_dynamic_filter_pushdown = true;

    let mut builder = SessionStateBuilder::new().with_config(config);

    if crate::gucs::is_mixed_fast_field_sort_enabled() {
        let rule = Arc::new(SortMergeJoinEnforcer::new());
        builder = builder.with_physical_optimizer_rule(rule);
        // Re-run dynamic filter pushdown after the enforcer. The enforcer's
        // transform_up causes `with_new_children` on ancestor nodes, which in
        // SortExec's case creates a new DynamicFilterPhysicalExpr that hasn't
        // been pushed to PgSearchScan yet. This second pass establishes the
        // connection.
        //
        // NOTE: Inserting the enforcer before the default FilterPushdown(Post)
        // rule (rather than appending a second pass) was considered, but the
        // enforcer relies on detecting CoalescePartitionsExec on HashJoin
        // children — the plan structure at that earlier pipeline point differs,
        // causing missing SortPreservingMergeExec and incorrect join results.
        builder =
            builder.with_physical_optimizer_rule(Arc::new(FilterPushdown::new_post_optimization()));
    }

    let state = builder.build();
    SessionContext::new_with_state(state)
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
pub async fn build_joinscan_physical_plan(
    ctx: &SessionContext,
    plan: datafusion::logical_expr::LogicalPlan,
) -> Result<Arc<dyn ExecutionPlan>> {
    let df = ctx.execute_logical_plan(plan).await?;
    let plan = df.create_physical_plan().await?;

    if plan.output_partitioning().partition_count() > 1 {
        Ok(Arc::new(CoalescePartitionsExec::new(plan)) as Arc<dyn ExecutionPlan>)
    } else {
        Ok(plan)
    }
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
        if join_clause.sources.len() < 2 {
            return Err(DataFusionError::Internal(
                "JoinScan requires at least 2 sources".into(),
            ));
        }

        let partitioning_idx = join_clause.partitioning_source_index();
        let df_join_type = match join_clause.join_type {
            JoinScanJoinType::Inner => JoinType::Inner,
            JoinScanJoinType::Semi => JoinType::LeftSemi,
            other => {
                return Err(DataFusionError::Internal(format!(
                    "JoinScan runtime: unsupported join type {:?}",
                    other
                )));
            }
        };

        if join_clause.join_type == JoinScanJoinType::Semi {
            if join_clause.sources.len() != 2 {
                return Err(DataFusionError::Internal(
                    "JoinScan runtime: SEMI JOIN requires exactly 2 sources".into(),
                ));
            }
            if partitioning_idx != 0 {
                return Err(DataFusionError::Internal(
                    "JoinScan runtime: SEMI JOIN requires partitioning the left source".into(),
                ));
            }
        }

        // 1. Start with the first source
        let mut df = build_source_df(ctx, &join_clause.sources[0], partitioning_idx == 0).await?;
        let alias0 = join_clause.sources[0].execution_alias(0);
        df = df.alias(&alias0)?;

        // Maintain a set of RTIs that are currently in 'df' (the left side)
        let mut left_rtis = std::collections::HashSet::new();
        left_rtis.insert(join_clause.sources[0].scan_info.heap_rti);

        // 2. Iteratively join subsequent sources
        for i in 1..join_clause.sources.len() {
            let right_source = &join_clause.sources[i];
            let right_df = build_source_df(ctx, right_source, partitioning_idx == i).await?;
            let alias_right = right_source.execution_alias(i);
            let right_df = right_df.alias(&alias_right)?;

            let right_rti = right_source.scan_info.heap_rti;

            // Find join keys connecting 'df' (left) and 'right_df' (right)
            let mut on: Vec<Expr> = Vec::new();

            for jk in &join_clause.join_keys {
                // Case 1: Key connects Left(outer) -> Right(inner)
                if left_rtis.contains(&jk.outer_rti) && jk.inner_rti == right_rti {
                    let left_source = join_clause
                        .sources
                        .iter()
                        .enumerate()
                        .find(|(_, s)| s.contains_rti(jk.outer_rti));
                    if let Some((left_idx, left_src)) = left_source {
                        let left_alias = left_src.execution_alias(left_idx);
                        let left_col_name =
                            left_src.column_name(jk.outer_attno).ok_or_else(|| {
                                DataFusionError::Internal("Missing column name".into())
                            })?;

                        let right_col_name =
                            right_source.column_name(jk.inner_attno).ok_or_else(|| {
                                DataFusionError::Internal("Missing column name".into())
                            })?;

                        on.push(
                            make_col(&left_alias, &left_col_name)
                                .eq(make_col(&alias_right, &right_col_name)),
                        );
                    }
                }
                // Case 2: Key connects Left(inner) -> Right(outer) (swap)
                // The JoinKeyPair stores 'outer' and 'inner' from Postgres perspective, but for our join chain:
                // One side must be in 'left_rtis', the other must be 'right_rti'.
                else if left_rtis.contains(&jk.inner_rti) && jk.outer_rti == right_rti {
                    let left_source = join_clause
                        .sources
                        .iter()
                        .enumerate()
                        .find(|(_, s)| s.contains_rti(jk.inner_rti));
                    if let Some((left_idx, left_src)) = left_source {
                        let left_alias = left_src.execution_alias(left_idx);
                        let left_col_name =
                            left_src.column_name(jk.inner_attno).ok_or_else(|| {
                                DataFusionError::Internal("Missing column name".into())
                            })?;

                        let right_col_name =
                            right_source.column_name(jk.outer_attno).ok_or_else(|| {
                                DataFusionError::Internal("Missing column name".into())
                            })?;

                        on.push(
                            make_col(&left_alias, &left_col_name)
                                .eq(make_col(&alias_right, &right_col_name)),
                        );
                    }
                }
            }

            if on.is_empty() {
                // Fallback: cross join if no keys found?
                // But JoinScan requires equi-keys.
                // If we have (A,B,C) and A=C, B=C.
                // Step 1: A.
                // Step 2: Join B. No keys A=B? Cross join?
                // Or we rely on the planner having ordered them such that there is connectivity.
                // If not connected, it's a cross join.

                // TODO: review this
                if join_clause.join_type == JoinScanJoinType::Semi {
                    return Err(DataFusionError::Internal(
                        "JoinScan runtime: SEMI JOIN requires equi-join keys".into(),
                    ));
                }
                df = df.join(right_df, df_join_type, &[], &[], None)?;
            } else {
                df = df.join_on(right_df, df_join_type, on)?;
            }

            left_rtis.insert(right_rti);
        }

        // 3. Apply Filter
        if let Some(ref join_level_expr) = join_clause.join_level_expr {
            let mapper = CombinedMapper {
                sources: &join_clause.sources,
                output_columns: &private_data.output_columns,
            };

            let translator =
                crate::postgres::customscan::joinscan::translator::PredicateTranslator::new(
                    &join_clause.sources,
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

            // Create a map of RTI -> CTID column expression for join-level predicates
            let mut ctid_map = crate::api::HashMap::default();
            for (i, source) in join_clause.sources.iter().enumerate() {
                let alias = source.execution_alias(i);

                let mut base_relations = Vec::new();
                source.collect_base_relations(&mut base_relations);

                for base in base_relations {
                    let rti = base.heap_rti;
                    let ctid_name = format!("ctid_{}", rti);
                    let expr = make_col(&alias, &ctid_name);
                    ctid_map.insert(rti, expr);
                }
            }

            // Translate join-level expression to DataFusion filter.
            // Single-table predicates use SearchPredicateUDF which can be pushed down
            // to PgSearchTableProvider via DataFusion's filter pushdown mechanism.
            let filter_expr = unsafe {
                crate::postgres::customscan::joinscan::translator::PredicateTranslator::translate_join_level_expr(
                    join_level_expr,
                    &translated_exprs,
                    &ctid_map,
                    &join_clause.join_level_predicates,
                )
            }
            .ok_or_else(|| {
                DataFusionError::Internal(format!(
                    "Failed to translate join level expression tree: {:?}",
                    join_level_expr
                ))
            })?;

            df = df.filter(filter_expr)?;
        }

        // 4. Apply Sort
        if !join_clause.order_by.is_empty() {
            let mut sort_exprs = Vec::new();
            for info in &join_clause.order_by {
                let expr = match &info.feature {
                    OrderByFeature::Score => {
                        // For N-way, 'ordering_side_is_outer' is insufficient.
                        // We need the index of the ordering side.
                        let ordering_idx = join_clause.ordering_side_index();
                        if let Some(idx) = ordering_idx {
                            let source = &join_clause.sources[idx];
                            let alias = source.execution_alias(idx);

                            // Try to find the score column
                            // Logic similar to build_projection_expr
                            // Default to SCORE_COL_NAME
                            make_col(&alias, SCORE_COL_NAME)
                        } else {
                            // Fallback
                            col("unknown_score")
                        }
                    }
                    OrderByFeature::Field(name) => col(name.as_ref()),
                    OrderByFeature::Var {
                        rti,
                        attno,
                        name: _,
                    } => {
                        // Resolve RTI/Attno to column expression
                        let mut resolved_expr = None;
                        for (i, source) in join_clause.sources.iter().enumerate() {
                            if let Some(mapped_attno) = source.map_var(*rti, *attno) {
                                if let Some(field_name) = source.column_name(mapped_attno) {
                                    let alias = source.execution_alias(i);
                                    resolved_expr = Some(make_col(&alias, &field_name));
                                    break;
                                }
                            }
                        }
                        resolved_expr.unwrap_or_else(|| col("unknown_col"))
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

        // 5. Apply Limit
        if let Some(limit) = join_clause.limit {
            df = df.limit(0, Some(limit))?;
        }

        // 6. Apply Output Projection
        let mut final_cols = Vec::new();

        if let Some(projection) = &join_clause.output_projection {
            for (i, proj) in projection.iter().enumerate() {
                let col_alias = format!("col_{}", i + 1);
                let expr = build_projection_expr(proj, join_clause);
                final_cols.push(expr.alias(col_alias));
            }

            // ALWAYS carry forward all CTID columns from both sides
            let mut base_relations = Vec::new();
            join_clause.collect_base_relations(&mut base_relations);
            for base in base_relations {
                let rti = base.heap_rti;
                let ctid_name = format!("ctid_{}", rti);
                // Check if it already exists in df schema (it should)
                if df.schema().field_with_unqualified_name(&ctid_name).is_ok() {
                    // Carry it.
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
    for (i, source) in join_clause.sources.iter().enumerate() {
        let alias = source.execution_alias(i);

        if proj.is_score {
            if let Some(attno) = source.map_var(proj.rti, 0) {
                if let Some(name) = source.column_name(attno) {
                    return make_col(&alias, &name);
                } else {
                    return make_col(&alias, SCORE_COL_NAME);
                }
            } else if source.contains_rti(proj.rti) {
                return make_col(&alias, SCORE_COL_NAME);
            }
        } else if let Some(attno) = source.map_var(proj.rti, proj.attno) {
            if let Some(field_name) = source.column_name(attno) {
                return make_col(&alias, &field_name);
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
    is_parallel: bool,
) -> LocalBoxFuture<'a, Result<DataFrame>> {
    async move {
        let scan_info = source.scan_info.clone();
        let source_alias = source.scan_info.alias.clone();
        let alias = source_alias.as_deref().unwrap_or("base");
        let fields: Vec<WhichFastField> = source
            .scan_info
            .fields
            .iter()
            .map(|f| f.field.clone())
            .collect();
        let provider = Arc::new(PgSearchTableProvider::new(
            scan_info,
            fields.clone(),
            None,
            is_parallel,
        ));
        ctx.register_table(alias, provider)?;

        let mut df = ctx.table(alias).await?;

        // Select fields AND ensure CTID is aliased uniquely
        let mut exprs = Vec::new();
        for (df_field, field_type) in df.schema().fields().iter().zip(fields.iter()) {
            let expr = match field_type {
                WhichFastField::Ctid => {
                    let rti = source.scan_info.heap_rti;
                    make_col(alias, df_field.name()).alias(format!("ctid_{}", rti))
                }
                WhichFastField::Score => make_col(alias, df_field.name()).alias(SCORE_COL_NAME),
                _ => make_col(alias, df_field.name()),
            };
            exprs.push(expr);
        }
        df = df.select(exprs)?;

        Ok(df)
    }
    .boxed_local()
}

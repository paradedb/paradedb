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

//! DataFusion plan builder for aggregate-on-join queries.
//!
//! Builds a DataFusion logical plan from a [`RelNode`] join tree and a
//! [`JoinAggregateTargetList`], producing: scan(s) → join → aggregate.
//!
//! Key difference from JoinScan's plan builder: no CTID columns, no late
//! materialization, no SegmentedTopK — aggregates run entirely on fast fields
//! and the result is aggregate rows, not individual tuples.

use super::join_targetlist::{AggOrderByEntry, GroupingTransform};
use super::pdb_agg::{
    PdbAggFieldRef, PdbAggPlan, PdbAggRequest, PdbKeySpec, PdbMetricSpec, PdbStat,
};
use crate::api::HashMap;
use crate::index::fast_fields_helper::WhichFastField;
use crate::index::reader::index::SearchIndexManifest;
use crate::postgres::customscan::aggregatescan::join_targetlist::{
    AggKind, JoinAggregateEntry, JoinAggregateTargetList,
};
use crate::postgres::customscan::aggregatescan::privdat::{CompareOp, DataFusionTopK, FilterExpr};
use crate::postgres::customscan::datafusion::cardinality_agg::tantivy_cardinality_udaf;
use crate::postgres::customscan::datafusion::numeric_agg::{
    numeric_bytes_avg_udaf, numeric_bytes_sum_udaf, numeric64_avg_udaf, numeric64_sum_udaf,
};
use crate::postgres::customscan::datafusion::timestamp_to_date::timestamp_to_date_udf;
use crate::postgres::customscan::datafusion::translator::{
    ColumnMapper, PredicateTranslator, apply_join_level_filter, apply_relnode_unnest,
    build_join_df_with_filter, make_col, make_source_col,
};
use crate::postgres::customscan::joinscan::CtidColumn;
use crate::postgres::customscan::joinscan::build::{
    JoinSource, LateralUnnestInfo, RelNode, RelationAlias,
};
use crate::postgres::customscan::joinscan::privdat::SCORE_COL_NAME;
use crate::postgres::customscan::joinscan::scan_state::{
    create_datafusion_session_context, register_source_table,
};
use crate::scan::PgSearchTableProvider;
use crate::schema::SearchFieldType;
use arrow_schema::DataType;
use datafusion::common::{DataFusionError, Result, ScalarValue};
use datafusion::functions::core::expr_fn::coalesce;
use datafusion::functions_aggregate::array_agg::array_agg_udaf;
use datafusion::functions_aggregate::count::count_udaf;
use datafusion::functions_aggregate::expr_fn::{
    array_agg, avg, bool_and, bool_or, count, max, min, stddev, stddev_pop, sum, var_pop,
    var_sample,
};
use datafusion::functions_aggregate::string_agg::string_agg_udaf;
use datafusion::logical_expr::expr::{AggregateFunction, Sort};
use datafusion::logical_expr::{
    Aggregate, Cast, Expr, GroupingSet, LogicalPlanBuilder, LogicalPlanBuilderOptions, col, lit,
};
use datafusion::prelude::{DataFrame, SessionContext};
use futures::future::{FutureExt, LocalBoxFuture};
use pgrx::pg_sys;
use tantivy::aggregation::Key;

/// Creates a DataFusion [`SessionContext`] for aggregate-on-join workloads.
pub fn create_aggregate_session_context() -> SessionContext {
    create_datafusion_session_context()
}

/// A built aggregate plan plus what execution needs to read its output.
pub struct JoinAggregatePlan {
    pub logical: datafusion::logical_expr::LogicalPlan,
    /// Per `targetlist.group_columns` entry, its DataFusion output column.
    pub group_df_indices: Vec<usize>,
    /// Set when the query carries `pdb.agg()` calls.
    pub pdb_plan: Option<PdbAggPlan>,
    /// `HAVING` of a scalar `pdb.agg()` query, applied to the assembled root row
    /// rather than inside the plan. Grouping sets emit no root row for an empty
    /// input, so the executor has to make one and judge it itself.
    pub pdb_root_having: Option<Expr>,
}

/// Build the complete DataFusion logical plan for an aggregate-on-join query:
/// scan(s) → join → aggregate [→ sort → limit].
#[allow(clippy::too_many_arguments)]
pub async fn build_join_aggregate_plan(
    plan: &RelNode,
    targetlist: &JoinAggregateTargetList,
    topk: Option<&DataFusionTopK>,
    custom_exprs: *mut pg_sys::List,
    custom_scan_tlist: *mut pg_sys::List,
    having_filter: Option<&FilterExpr>,
    ctx: &SessionContext,
    expr_context: Option<*mut pg_sys::ExprContext>,
    planstate: Option<*mut pg_sys::PlanState>,
    mpp_manifests: Option<&[SearchIndexManifest]>,
) -> Result<JoinAggregatePlan> {
    // Step 1: Build the join DataFrame from the RelNode tree
    let df = build_relnode_df(
        ctx,
        plan,
        plan,
        custom_exprs,
        custom_scan_tlist,
        expr_context,
        planstate,
        mpp_manifests,
    )
    .await?;

    // Step 2: Build GROUP BY expressions
    // DataFusion deduplicates grouping expressions that resolve to the same
    // column name (e.g. metadata.brand). We must track which DataFusion output
    // column index corresponds to each of our original targetlist.group_columns.
    let mut group_exprs = Vec::new();
    let mut field_to_df_idx = crate::api::HashMap::default();
    let mut group_df_indices = Vec::with_capacity(targetlist.group_columns.len());

    for gc in &targetlist.group_columns {
        // Dedup key by (plan_position, field_name, transform): plan_position is the
        // unique source identity; field_name distinguishes columns within
        // a source, transform distinguishes different transformations of the same column.
        // Keying by rti would collapse rti-aliased sources from
        // sub-PlannerInfos into one DataFusion column.
        let entry = field_to_df_idx.entry((gc.plan_position, gc.field_name.clone(), gc.transform));
        let df_idx = match entry {
            std::collections::hash_map::Entry::Vacant(v) => {
                let df_idx = group_exprs.len();
                v.insert(df_idx);
                let column = make_plan_position_col(plan, gc.plan_position, &gc.field_name);

                let group_expr = match gc.transform {
                    GroupingTransform::Identity => column,
                    GroupingTransform::TimestampToDate => {
                        timestamp_to_date_udf().call(vec![column])
                    }
                };

                group_exprs.push(group_expr);
                df_idx
            }

            std::collections::hash_map::Entry::Occupied(o) => *o.get(),
        };
        group_df_indices.push(df_idx);
    }

    // Step 3: Build aggregate expressions. `pdb.agg()` entries contribute no
    // expression of their own; their lowered plan is folded in below.
    let mut pdb_entries: Vec<(usize, &PdbAggRequest, bool)> = Vec::new();
    let mut pdb_filters: HashMap<usize, Expr> = HashMap::default();
    let agg_exprs: Vec<Expr> = targetlist
        .aggregates
        .iter()
        .enumerate()
        .map(|(i, agg)| -> Result<Option<Expr>> {
            // Per-aggregate FILTER clause, shared by both kinds of entry.
            let df_filter = agg
                .filter
                .as_ref()
                .map(|filter_expr| {
                    let filter_ctx = FilterExprExecContext {
                        targetlist: None,
                        plan: Some(plan),
                    };
                    filter_expr.to_datafusion(&filter_ctx).ok_or_else(|| {
                        DataFusionError::Internal(
                            "Failed to translate aggregate FILTER clause to DataFusion".to_string(),
                        )
                    })
                })
                .transpose()?;
            if let AggKind::PdbAgg(request) = &agg.agg_kind {
                if let Some(df_filter) = df_filter {
                    pdb_filters.insert(i, df_filter);
                }
                pdb_entries.push((i, request.as_ref(), agg.filter.is_some()));
                return Ok(None);
            }
            let agg_expr = match agg.agg_kind {
                AggKind::CountStar => Ok(count(lit(1))),
                AggKind::Count => agg_field_col(agg, plan).map(count),
                AggKind::CountDistinct => {
                    let col_exprs = agg_field_cols(agg, plan)?;
                    Ok(Expr::AggregateFunction(AggregateFunction::new_udf(
                        count_udaf(),
                        col_exprs,
                        true,   // distinct
                        None,   // filter
                        vec![], // order_by
                        None,   // null_treatment
                    )))
                }
                AggKind::Sum => agg_field_col(agg, plan).map(|col| match &agg.numeric {
                    None => sum(col),
                    Some(field_type) => numeric_sum(col, field_type),
                }),
                AggKind::Avg => agg_field_col(agg, plan).map(|col| match &agg.numeric {
                    None => avg(col),
                    Some(field_type) => numeric_avg(col, field_type),
                }),
                AggKind::Min => agg_field_col(agg, plan).map(min),
                AggKind::Max => agg_field_col(agg, plan).map(max),
                AggKind::StddevSamp => agg_field_col(agg, plan).map(stddev),
                AggKind::StddevPop => agg_field_col(agg, plan).map(stddev_pop),
                AggKind::VarSamp => agg_field_col(agg, plan).map(var_sample),
                AggKind::VarPop => agg_field_col(agg, plan).map(var_pop),
                AggKind::BoolAnd => agg_field_col(agg, plan).map(bool_and),
                AggKind::BoolOr => agg_field_col(agg, plan).map(bool_or),
                AggKind::ArrayAgg => {
                    let col_expr = agg_field_col(agg, plan)?;
                    if agg.order_by.is_empty() {
                        Ok(array_agg(col_expr))
                    } else {
                        Ok(Expr::AggregateFunction(AggregateFunction::new_udf(
                            array_agg_udaf(),
                            vec![col_expr],
                            false,
                            None,
                            agg_order_by_exprs(&agg.order_by, plan),
                            None,
                        )))
                    }
                }
                AggKind::StringAgg(ref sep) => {
                    let col_expr = agg_field_col(agg, plan)?;
                    let sep_lit = lit(sep.clone());
                    if agg.order_by.is_empty() {
                        Ok(datafusion::functions_aggregate::string_agg::string_agg(
                            col_expr, sep_lit,
                        ))
                    } else {
                        Ok(Expr::AggregateFunction(AggregateFunction::new_udf(
                            string_agg_udaf(),
                            vec![col_expr, sep_lit],
                            false,
                            None,
                            agg_order_by_exprs(&agg.order_by, plan),
                            None,
                        )))
                    }
                }
                AggKind::PdbAgg(_) => unreachable!("pdb.agg entries return above"),
            }?;
            // Apply DISTINCT flag for non-CountDistinct aggregates.
            // CountDistinct already sets distinct=true via new_udf above.
            let agg_expr = if agg.distinct
                && !matches!(agg.agg_kind, AggKind::CountDistinct | AggKind::CountStar)
            {
                with_distinct(agg_expr)
            } else {
                agg_expr
            };
            // Apply per-aggregate FILTER clause if present.
            let agg_expr = match df_filter {
                Some(df_filter) => with_filter(agg_expr, df_filter),
                None => agg_expr,
            };
            // Alias for stable reference
            Ok(Some(agg_expr.alias(format!("agg_{}", i))))
        })
        .filter_map(Result::transpose)
        .collect::<Result<Vec<Expr>>>()?;

    let having_expr = having_filter
        .map(|having| {
            let having_ctx = FilterExprExecContext {
                targetlist: Some(targetlist),
                plan: None,
            };
            having.to_datafusion(&having_ctx).ok_or_else(|| {
                DataFusionError::Internal(
                    "Failed to translate HAVING clause to DataFusion expression".to_string(),
                )
            })
        })
        .transpose()?;

    // Step 4: Apply aggregate, then HAVING (post-aggregate)
    let pdb_plan = (!pdb_entries.is_empty())
        .then(|| PdbAggPlan::build(&pdb_entries, group_exprs.len(), agg_exprs.len()))
        .transpose()?;
    let (having_expr, pdb_root_having) = match (&pdb_plan, group_exprs.is_empty()) {
        (Some(_), true) => (None, having_expr),
        _ => (having_expr, None),
    };
    let mut df = match &pdb_plan {
        Some(pdb_plan) => apply_pdb_aggregate(
            df,
            pdb_plan,
            group_exprs,
            agg_exprs,
            &pdb_filters,
            having_expr,
            plan,
        )?,
        None => {
            let df = df.aggregate(group_exprs, agg_exprs)?;
            match having_expr {
                Some(expr) => df.filter(expr)?,
                None => df,
            }
        }
    };

    // Step 5: If TopK is requested, add sort + limit so DataFusion handles
    // it internally. DataFusion's built-in TopKAggregation optimizer rule
    // can then push the limit into AggregateExec for group-key and MIN/MAX
    // ordering. For COUNT/SUM/AVG ordering, SortExec(fetch=K) uses a
    // bounded TopK heap.
    if let Some(topk) = topk {
        let sort_col_name = topk.sort_target.resolve_sort_col_name(targetlist, plan);
        let sort_expr = datafusion::prelude::col(&sort_col_name)
            .sort(topk.direction.is_asc(), topk.direction.is_nulls_first());
        df = df.sort(vec![sort_expr])?;
        df = df.limit(0, Some(topk.k))?;
    }

    Ok(JoinAggregatePlan {
        logical: df.into_optimized_plan()?,
        group_df_indices,
        pdb_plan,
        pdb_root_having,
    })
}

/// Aggregate with every `pdb.agg()` level folded in as grouping sets, then
/// project to the column order [`PdbAggPlan`] documents.
///
/// `HAVING` only judges the SQL level. A bucket row of a group the clause drops
/// stays in the output, and the assembler ignores it for lack of a root row.
fn apply_pdb_aggregate(
    df: DataFrame,
    pdb_plan: &PdbAggPlan,
    group_exprs: Vec<Expr>,
    agg_exprs: Vec<Expr>,
    pdb_filters: &HashMap<usize, Expr>,
    having_expr: Option<Expr>,
    plan: &RelNode,
) -> Result<DataFrame> {
    let key_exprs: Vec<Expr> = pdb_plan
        .keys
        .iter()
        .enumerate()
        .map(|(i, key)| pdb_key_expr(key, plan).alias(format!("__pdb_k{i}")))
        .collect();
    let metric_exprs: Vec<Expr> = pdb_plan
        .metrics
        .iter()
        .enumerate()
        .map(|(j, metric)| pdb_metric_expr(metric, plan, pdb_filters).alias(format!("__pdb_m{j}")))
        .collect();

    let num_std_aggs = agg_exprs.len();
    let mut all_group_exprs = group_exprs.clone();
    all_group_exprs.extend(key_exprs);
    let grouping = if pdb_plan.has_grouping_sets() {
        let sets = pdb_plan
            .levels
            .iter()
            .map(|level| level.iter().map(|&p| all_group_exprs[p].clone()).collect())
            .collect();
        vec![Expr::GroupingSet(GroupingSet::GroupingSets(sets))]
    } else {
        group_exprs.clone()
    };
    let mut all_agg_exprs = agg_exprs;
    all_agg_exprs.extend(metric_exprs);
    // `DataFrame::aggregate` projects `__grouping_id` away; the assembler needs it,
    // so build the aggregate node directly.
    let (session_state, input) = df.into_parts();
    let options = LogicalPlanBuilderOptions::new().with_add_implicit_group_by_exprs(true);
    let aggregated = LogicalPlanBuilder::from(input)
        .with_options(options)
        .aggregate(grouping, all_agg_exprs)?
        .build()?;
    let mut df = DataFrame::new(session_state, aggregated);

    if let Some(having) = having_expr {
        let guarded = match pdb_plan.grouping_id_col() {
            Some(_) => col(Aggregate::INTERNAL_GROUPING_ID)
                .not_eq(lit(pdb_plan.root_grouping_id()))
                .or(having),
            None => having,
        };
        df = df.filter(guarded)?;
    }

    // The aggregate lays its output out as group expressions, `__grouping_id`
    // when there are grouping sets, then aggregates. Read it back by position:
    // a group key can be a call rather than a column, so it cannot be named
    // again here, and the CSE pass renames aggregates later.
    let output: Vec<Expr> = df
        .schema()
        .columns()
        .into_iter()
        .map(Expr::Column)
        .collect();
    let num_group = group_exprs.len();
    let num_keys = pdb_plan.keys.len();
    let aggs_start = output.len() - num_std_aggs - pdb_plan.metrics.len();
    let mut select = Vec::with_capacity(output.len());
    select.extend_from_slice(&output[..num_group]);
    select.extend_from_slice(&output[aggs_start..aggs_start + num_std_aggs]);
    select.extend_from_slice(&output[num_group + num_keys..aggs_start]);
    select.extend_from_slice(&output[num_group..num_group + num_keys]);
    select.extend_from_slice(&output[aggs_start + num_std_aggs..]);
    df.select(select)
}

/// The `missing` literal in the column's own Arrow type, so `coalesce` neither
/// widens the key column nor fails on a text column with a numeric literal.
fn pdb_missing_lit(missing: &Key, field: &PdbAggFieldRef) -> Expr {
    let literal = match missing {
        Key::Str(s) => lit(s.clone()),
        Key::I64(v) => lit(*v),
        Key::U64(v) => lit(*v),
        // A timestamp column takes its literal as whole microseconds.
        Key::F64(v) if field.is_datetime() => lit(*v as i64),
        Key::F64(v) => lit(*v),
    };
    Expr::Cast(Cast::new(
        Box::new(literal),
        field.field_type.arrow_data_type(),
    ))
}

fn pdb_key_expr(key: &PdbKeySpec, plan: &RelNode) -> Expr {
    let column = make_plan_position_col(plan, key.field.plan_position, &key.field.field_name);
    match &key.missing {
        None => column,
        Some(missing) => coalesce(vec![column, pdb_missing_lit(missing, &key.field)]),
    }
}

fn pdb_metric_expr(
    metric: &PdbMetricSpec,
    plan: &RelNode,
    pdb_filters: &HashMap<usize, Expr>,
) -> Expr {
    let (expr, entry_filter) = match metric {
        PdbMetricSpec::DocCount { entry_filter } => (count(lit(1)), entry_filter),
        PdbMetricSpec::Stat {
            stat,
            field,
            missing,
            entry_filter,
        } => {
            let mut column = make_plan_position_col(plan, field.plan_position, &field.field_name);
            if let Some(missing) = missing {
                column = coalesce(vec![column, pdb_missing_lit(missing, field)]);
            }
            // A sum runs in f64 like Tantivy's, which also keeps an integer sum
            // from overflowing. Timestamps have no direct f64 cast.
            let as_f64 = |column: Expr| {
                let column = if field.is_datetime() {
                    Expr::Cast(Cast::new(Box::new(column), DataType::Int64))
                } else {
                    column
                };
                Expr::Cast(Cast::new(Box::new(column), DataType::Float64))
            };
            let expr = match stat {
                PdbStat::Count => count(column),
                // NUMERIC takes the decimal accumulator the SQL aggregates use; the
                // assembler decodes its blob.
                PdbStat::Sum if field.field_type.is_numeric() => {
                    numeric_sum(column, &field.field_type)
                }
                PdbStat::Sum => sum(as_f64(column)),
                PdbStat::Min => min(column),
                PdbStat::Max => max(column),
                // Tantivy's own sketch, salted by the column type the way a
                // segment collection is.
                PdbStat::Cardinality => tantivy_cardinality_udaf().call(vec![
                    column,
                    lit(ScalarValue::UInt8(Some(field.column_type().to_code()))),
                ]),
            };
            (expr, entry_filter)
        }
    };
    match entry_filter.and_then(|i| pdb_filters.get(&i)) {
        Some(filter) => with_filter(expr, filter.clone()),
        None => expr,
    }
}

/// `SUM` over a NUMERIC column: the scaled-Int64 or the decimal-bytes accumulator,
/// by storage. The `Numeric64` UDAFs take the scale as a plan literal so it
/// survives plan serialization for parallel and MPP execution; decimal-bytes
/// values are self-describing.
fn numeric_sum(col: Expr, field_type: &SearchFieldType) -> Expr {
    match field_type {
        SearchFieldType::Numeric64(_, scale) => {
            numeric64_sum_udaf().call(vec![col, lit(*scale as i32)])
        }
        _ => numeric_bytes_sum_udaf().call(vec![col]),
    }
}

/// `AVG` over a NUMERIC column; see [`numeric_sum`].
fn numeric_avg(col: Expr, field_type: &SearchFieldType) -> Expr {
    match field_type {
        SearchFieldType::Numeric64(_, scale) => {
            numeric64_avg_udaf().call(vec![col, lit(*scale as i32)])
        }
        _ => numeric_bytes_avg_udaf().call(vec![col]),
    }
}

/// Recursively lower a [`RelNode`] tree into a DataFusion [`DataFrame`] for AggregateScan.
///
/// Handles Scan, Join, Filter, and Unnest operators. Unlike JoinScan's variant, this:
/// - Does NOT handle LIMIT, ORDER BY, DISTINCT, or output projection
///   (those are handled by the aggregate layer above)
/// - Is single-threaded (no partitioning logic)
#[allow(clippy::too_many_arguments)]
fn build_relnode_df<'a>(
    ctx: &'a SessionContext,
    node: &'a RelNode,
    top_level_plan: &'a RelNode,
    custom_exprs: *mut pg_sys::List,
    custom_scan_tlist: *mut pg_sys::List,
    expr_context: Option<*mut pg_sys::ExprContext>,
    planstate: Option<*mut pg_sys::PlanState>,
    mpp_manifests: Option<&'a [SearchIndexManifest]>,
) -> LocalBoxFuture<'a, Result<DataFrame>> {
    async move {
        match node {
            RelNode::Scan(source) => {
                let plan_position = source.plan_position;
                let df = build_source_df(
                    ctx,
                    source,
                    top_level_plan,
                    plan_position,
                    expr_context,
                    planstate,
                    mpp_manifests,
                )
                .await?;
                let alias =
                    RelationAlias::new(source.scan_info.alias.as_deref()).execution(plan_position);
                Ok(df.alias(&alias)?)
            }
            RelNode::Join(join) => {
                let left_df = build_relnode_df(
                    ctx,
                    &join.left,
                    top_level_plan,
                    custom_exprs,
                    custom_scan_tlist,
                    expr_context,
                    planstate,
                    mpp_manifests,
                )
                .await?;
                let right_df = build_relnode_df(
                    ctx,
                    &join.right,
                    top_level_plan,
                    custom_exprs,
                    custom_scan_tlist,
                    expr_context,
                    planstate,
                    mpp_manifests,
                )
                .await?;

                let mut sources = join.left.sources();
                sources.extend(join.right.sources());
                build_join_df_with_filter(left_df, right_df, join, &sources, &[], &[])
            }
            RelNode::Filter(filter) => {
                let df = build_relnode_df(
                    ctx,
                    &filter.input,
                    top_level_plan,
                    custom_exprs,
                    custom_scan_tlist,
                    expr_context,
                    planstate,
                    mpp_manifests,
                )
                .await?;

                let sources = filter.input.output_sources();

                // Translate custom_exprs (non-@@@ cross-table predicates) using
                // PredicateTranslator, mirroring JoinScan's scan_state.rs:562-576.
                // After setrefs, Vars in custom_exprs are INDEX_VAR references
                // that index into custom_scan_tlist. We need a mapper to resolve
                // them back to the correct DataFusion column names.
                let mut translated_exprs = Vec::new();
                if !custom_exprs.is_null() {
                    let lateral_unnests = filter.input.lateral_unnests();
                    let mapper = AggregateIndexVarMapper {
                        sources: &sources,
                        custom_scan_tlist,
                        lateral_unnests,
                    };
                    let translator =
                        PredicateTranslator::new(&sources).with_mapper(Box::new(mapper));
                    unsafe {
                        let expr_list = pgrx::PgList::<pg_sys::Node>::from_pg(custom_exprs);
                        for (i, expr_node) in expr_list.iter_ptr().enumerate() {
                            let expr = translator.translate(expr_node).ok_or_else(|| {
                                DataFusionError::Internal(format!(
                                    "Failed to translate aggregate custom expression at index {}",
                                    i
                                ))
                            })?;
                            translated_exprs.push(expr);
                        }
                    }
                }

                apply_join_level_filter(
                    df,
                    &filter.predicate,
                    &translated_exprs,
                    &sources,
                    /* handle_mark = */ false,
                )
            }
            RelNode::Unnest(unnest) => {
                let df = build_relnode_df(
                    ctx,
                    &unnest.input,
                    top_level_plan,
                    custom_exprs,
                    custom_scan_tlist,
                    expr_context,
                    planstate,
                    mpp_manifests,
                )
                .await?;
                apply_relnode_unnest(df, unnest)
            }
        }
    }
    .boxed_local()
}

/// Maps INDEX_VAR references (from setrefs-transformed custom_exprs) back to
/// DataFusion column names. In the aggregate scan, custom_scan_tlist mirrors
/// the plan's targetlist (plus any Vars we added for predicates), and INDEX_VAR
/// varattno indexes into it. We resolve each Var by looking up the original
/// (rti, attno) from custom_scan_tlist and finding the corresponding source.
struct AggregateIndexVarMapper<'a> {
    sources: &'a [&'a JoinSource],
    custom_scan_tlist: *mut pg_sys::List,
    lateral_unnests: Vec<&'a LateralUnnestInfo>,
}

impl<'a> ColumnMapper for AggregateIndexVarMapper<'a> {
    fn map_var(&self, varno: pg_sys::Index, varattno: pg_sys::AttrNumber) -> Option<Expr> {
        let (rti, attno) = if varno == pg_sys::INDEX_VAR as pg_sys::Index {
            // INDEX_VAR: look up the original Var from custom_scan_tlist.
            // varattno is 1-indexed into the target list.
            unsafe {
                let tlist = pgrx::PgList::<pg_sys::TargetEntry>::from_pg(self.custom_scan_tlist);
                let idx = (varattno - 1) as usize;
                let te = tlist.get_ptr(idx)?;
                if (*(*te).expr).type_ != pg_sys::NodeTag::T_Var {
                    return None;
                }
                let var = (*te).expr as *mut pg_sys::Var;
                ((*var).varno as pg_sys::Index, (*var).varattno)
            }
        } else {
            (varno, varattno)
        };

        if let Some(unnest_info) = self
            .lateral_unnests
            .iter()
            .find(|u| u.function_rti.0 == rti)
        {
            let source = self
                .sources
                .iter()
                .find(|s| s.contains_rti(unnest_info.source_rti.0))?;
            let alias = RelationAlias::new(source.scan_info.alias.as_deref())
                .execution(source.plan_position);
            return Some(datafusion::logical_expr::col(format!(
                "{}_{}",
                alias, unnest_info.field_name
            )));
        }

        let source = self.sources.iter().find(|s| s.contains_rti(rti))?;
        let field_name = source.column_name(attno)?;
        if self
            .lateral_unnests
            .iter()
            .any(|u| source.contains_rti(u.source_rti.0) && u.field_name == field_name)
        {
            let alias = RelationAlias::new(source.scan_info.alias.as_deref())
                .execution(source.plan_position);
            Some(datafusion::logical_expr::col(format!(
                "{alias}_{field_name}"
            )))
        } else {
            Some(make_source_col(source, &field_name))
        }
    }
}

/// Context for the **exec phase** — translating a [`FilterExpr`] IR into a
/// DataFusion [`Expr`].
///
/// HAVING provides `targetlist` for resolving `AggRef`/`GroupRef`;
/// FILTER provides `plan` (a `RelNode` tree) for resolving `ColumnRef`.
///
/// This is distinct from the build-phase context in `datafusion_build.rs`,
/// which carries raw planner `JoinAggSource`s instead of a `RelNode` tree.
struct FilterExprExecContext<'a> {
    targetlist: Option<&'a JoinAggregateTargetList>,
    plan: Option<&'a RelNode>,
}

impl FilterExpr {
    /// Translate this expression to a DataFusion `Expr`.
    ///
    /// Used for both HAVING (pass `targetlist`) and per-aggregate FILTER (pass `plan`).
    fn to_datafusion(&self, ctx: &FilterExprExecContext<'_>) -> Option<Expr> {
        use datafusion::logical_expr::Operator;

        match self {
            FilterExpr::AggRef(idx) => {
                let tl = ctx.targetlist?;
                if *idx < tl.aggregates.len() {
                    Some(datafusion::prelude::col(format!("agg_{}", idx)))
                } else {
                    None
                }
            }
            FilterExpr::GroupRef(field_name) => Some(datafusion::prelude::col(field_name.as_str())),
            FilterExpr::ColumnRef {
                plan_position,
                field_name,
                ..
            } => {
                let plan = ctx.plan?;
                Some(make_plan_position_col(plan, *plan_position, field_name))
            }
            FilterExpr::LitInt(v) => Some(lit(*v)),
            FilterExpr::LitFloat(v) => Some(lit(*v)),
            FilterExpr::LitBool(v) => Some(lit(*v)),
            FilterExpr::LitString(v) => Some(lit(v.clone())),
            FilterExpr::BinOp { left, op, right } => {
                let l = left.to_datafusion(ctx)?;
                let r = right.to_datafusion(ctx)?;
                let df_op = match op {
                    CompareOp::Eq => Operator::Eq,
                    CompareOp::NotEq => Operator::NotEq,
                    CompareOp::Lt => Operator::Lt,
                    CompareOp::LtEq => Operator::LtEq,
                    CompareOp::Gt => Operator::Gt,
                    CompareOp::GtEq => Operator::GtEq,
                };
                Some(Expr::BinaryExpr(datafusion::logical_expr::BinaryExpr::new(
                    Box::new(l),
                    df_op,
                    Box::new(r),
                )))
            }
            FilterExpr::And(children) => {
                let exprs: Vec<Expr> = children
                    .iter()
                    .map(|c| c.to_datafusion(ctx))
                    .collect::<Option<Vec<Expr>>>()?;
                let mut result = exprs.into_iter();
                let first = result.next()?;
                Some(result.fold(first, |acc, e| acc.and(e)))
            }
            FilterExpr::Or(children) => {
                let exprs: Vec<Expr> = children
                    .iter()
                    .map(|c| c.to_datafusion(ctx))
                    .collect::<Option<Vec<Expr>>>()?;
                let mut result = exprs.into_iter();
                let first = result.next()?;
                Some(result.fold(first, |acc, e| acc.or(e)))
            }
            FilterExpr::Not(inner) => {
                let e = inner.to_datafusion(ctx)?;
                Some(Expr::Not(Box::new(e)))
            }
            FilterExpr::IsNull(inner) => {
                let e = inner.to_datafusion(ctx)?;
                Some(e.is_null())
            }
            FilterExpr::IsNotNull(inner) => {
                let e = inner.to_datafusion(ctx)?;
                Some(e.is_not_null())
            }
        }
    }
}

/// Build a DataFusion [`DataFrame`] for a single scan source.
async fn build_source_df(
    ctx: &SessionContext,
    source: &JoinSource,
    plan: &RelNode,
    plan_position: usize,
    expr_context: Option<*mut pg_sys::ExprContext>,
    planstate: Option<*mut pg_sys::PlanState>,
    mpp_manifests: Option<&[SearchIndexManifest]>,
) -> Result<DataFrame> {
    let scan_info = source.scan_info.clone();
    let alias = RelationAlias::new(scan_info.alias.as_deref()).execution(plan_position);
    let fields: Vec<WhichFastField> = source
        .scan_info
        .fields
        .iter()
        .map(|f| f.field.clone())
        .collect();

    // Each source that solves runtime PostgreSQL expressions needs its own
    // per-tuple memory context. SearchQueryInput::solve_postgres_expressions()
    // resets that context before replacing Param/PostgresExpression nodes. If
    // two providers share one context, solving the second source invalidates
    // the Const/expression nodes retained by the first source. Generic plans
    // that parameterize predicates on both join inputs then dereference stale
    // nodes during reader construction and abort the backend.
    let source_query = scan_info.mode.query().clone();
    let needs_runtime_context =
        source_query.has_postgres_expressions() || source_query.has_parameters();
    // `.or(expr_context)` can hand every source the same context, but only the
    // EXPLAIN-only rebuild reaches it. There it is safe: `PgSearchTableProvider::scan()` returns
    // "postgres expressions have not been solved: missing planstate" before it would call
    // `solve_postgres_expressions`, so the shared context is never reset.
    let source_expr_context = if needs_runtime_context {
        unsafe {
            planstate
                .and_then(|planstate| {
                    if planstate.is_null() || (*planstate).state.is_null() {
                        None
                    } else {
                        Some(pg_sys::CreateExprContext((*planstate).state))
                    }
                })
                .or(expr_context)
        }
    } else {
        expr_context
    };

    // MPP-aware provider setup. Every source gets its segments sliced across PG
    // parallel workers via `parallel_state.checkout_segment_for_source(plan_position)`
    // when this is an MPP plan.
    let source_idx = mpp_manifests.map(|_| plan_position);
    let mut provider = PgSearchTableProvider::new(scan_info, fields.clone(), source_idx);
    // The leader claims segments out of the DSM pool the same manifests populate, so its own
    // reader is built from the source's manifest. This plan never crosses the codec that
    // injects the manifest for JoinScan, so do it here.
    if let Some(manifests) = mpp_manifests {
        let manifest = manifests.get(plan_position).unwrap_or_else(|| {
            panic!(
                "missing captured manifest for aggregate source at plan_position {plan_position}"
            )
        });
        provider.set_manifest(manifest.clone());
    }
    if let crate::scan::ScanMode::Tagged { local_queries, .. } = &source.scan_info.mode {
        for tq in local_queries {
            provider.add_match_tag_column(&tq.tag_name);
        }
    }
    // HeapFilter queries (e.g. `=` on a column indexed via a
    // `pdb.literal(...)` cast) compile to runtime Postgres expressions
    // that can only be evaluated with a live ExprContext + PlanState.
    // The provider's `scan()` reaches for them via
    // `init_postgres_expressions` / `solve_postgres_expressions` only
    // when needed, so threading them through here is what makes the
    // agg-on-join path match JoinScan and Base Scan.
    provider.set_expr_context(source_expr_context);
    provider.set_planstate(planstate);

    // Deferring an aggregate source's visibility trades an in-scan check for a
    // post-join one. On the current cost model that only pays for specific
    // shapes, so it stays off until selective late materialization can pick
    // them. With it off the source keeps eager, in-scan visibility.
    if crate::gucs::enable_aggregate_late_materialization() {
        let mut required_early: crate::api::HashSet<String> = Default::default();
        for jk in plan.join_keys() {
            if source.contains_rti(jk.outer_rti)
                && let Some(col) = source.column_name(jk.outer_attno)
            {
                required_early.insert(col);
            }
            if source.contains_rti(jk.inner_rti)
                && let Some(col) = source.column_name(jk.inner_attno)
            {
                required_early.insert(col);
            }
        }
        for (rti, attno) in plan.filter_input_vars() {
            if source.contains_rti(rti)
                && let Some(col) = source.column_name(attno)
            {
                required_early.insert(col);
            }
        }

        provider.configure_deferred_outputs(
            &required_early,
            crate::scan::VisibilityMode::Deferred { plan_position },
        );
    }

    let df = register_source_table(ctx, alias.as_str(), provider).await?;

    // Select fields AND ensure CTID and Score are aliased consistently with JoinScan
    let mut exprs = Vec::new();
    for df_field in df.schema().fields().iter() {
        let name = df_field.name();
        let expr = match fields.iter().find(|w| w.name() == *name) {
            Some(WhichFastField::Ctid) => {
                make_col(alias.as_str(), name).alias(CtidColumn::new(plan_position).to_string())
            }
            Some(WhichFastField::Score) => make_col(alias.as_str(), name).alias(SCORE_COL_NAME),
            _ => make_col(alias.as_str(), name),
        };
        exprs.push(expr);
    }

    if exprs.is_empty() {
        // No fields at all — this can happen for COUNT(*) where no columns are
        // referenced from this source. Return the raw DataFrame.
        Ok(df)
    } else {
        df.select(exprs)
    }
}

/// Build a DataFusion column expression for a targetlist ref by its
/// previously-resolved `plan_position`.
fn make_plan_position_col(plan: &RelNode, plan_position: usize, field_name: &str) -> Expr {
    let source = plan
        .source_at_plan_position(plan_position)
        .unwrap_or_else(|| panic!("no source at plan_position {plan_position}"));
    let alias =
        RelationAlias::new(source.scan_info.alias.as_deref()).execution(source.plan_position);
    if plan
        .lateral_unnests()
        .iter()
        .any(|u| source.contains_rti(u.source_rti.0) && u.field_name == field_name)
    {
        datafusion::logical_expr::col(format!("{alias}_{field_name}"))
    } else {
        make_col(&alias, field_name)
    }
}

/// Replace an `Expr::AggregateFunction` with the same call but `distinct=true`.
/// Non-aggregate-function expressions are returned unchanged.
fn with_distinct(expr: Expr) -> Expr {
    match expr {
        Expr::AggregateFunction(af) => Expr::AggregateFunction(AggregateFunction::new_udf(
            af.func,
            af.params.args,
            true,
            af.params.filter,
            af.params.order_by,
            af.params.null_treatment,
        )),
        other => other,
    }
}

/// Replace an `Expr::AggregateFunction` with the same call but `filter=Some(...)`.
/// Non-aggregate-function expressions are returned unchanged.
fn with_filter(expr: Expr, filter: Expr) -> Expr {
    match expr {
        Expr::AggregateFunction(af) => Expr::AggregateFunction(AggregateFunction::new_udf(
            af.func,
            af.params.args,
            af.params.distinct,
            Some(Box::new(filter)),
            af.params.order_by,
            af.params.null_treatment,
        )),
        other => other,
    }
}

/// Build a DataFusion column expression for an aggregate's first field reference.
fn agg_field_col(agg: &JoinAggregateEntry, plan: &RelNode) -> Result<Expr> {
    let r = agg.field_refs.first().ok_or_else(|| {
        DataFusionError::Internal("non-COUNT(*) aggregate must have a field reference".to_string())
    })?;
    Ok(make_plan_position_col(plan, r.plan_position, &r.field_name))
}

/// Convert aggregate ORDER BY entries to DataFusion `Sort` expressions.
fn agg_order_by_exprs(order_by: &[AggOrderByEntry], plan: &RelNode) -> Vec<Sort> {
    order_by
        .iter()
        .map(|entry| {
            Sort::new(
                make_plan_position_col(plan, entry.plan_position, &entry.field_name),
                entry.direction.is_asc(),
                entry.direction.is_nulls_first(),
            )
        })
        .collect()
}

/// Build DataFusion column expressions for all of an aggregate's field references.
/// Used for multi-column DISTINCT (e.g. `COUNT(DISTINCT col1, col2)`).
fn agg_field_cols(agg: &JoinAggregateEntry, plan: &RelNode) -> Result<Vec<Expr>> {
    agg.field_refs
        .iter()
        .map(|r| Ok(make_plan_position_col(plan, r.plan_position, &r.field_name)))
        .collect()
}

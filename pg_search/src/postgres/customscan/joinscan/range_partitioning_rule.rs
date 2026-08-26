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

use std::sync::Arc;

use datafusion::catalog::default_table_source::DefaultTableSource;
use datafusion::common::config::ConfigOptions;
use datafusion::common::tree_node::{Transformed, TreeNode};
use datafusion::common::{Column, DataFusionError, JoinType, Result};
use datafusion::logical_expr::{Expr, LogicalPlan, TableScan};
use datafusion::optimizer::{OptimizerConfig, OptimizerRule, optimizer::ApplyOrder};
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::joins::{HashJoinExec, PartitionMode};
use datafusion::physical_plan::{ExecutionPlan, ExecutionPlanProperties, Partitioning};
use tantivy::SegmentReader;

use crate::api::FieldName;
use crate::index::fast_fields_helper::FFType;
use crate::index::fast_fields_helper::WhichFastField;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::index::stats::persisted_split_points;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::rel::PgSearchRelation;
use crate::query::SearchQueryInput;

use crate::scan::range_partitioning::RangePartitioningSample;
use crate::scan::table_provider::PgSearchTableProvider;

/// Optimizer rule that coordinates range partitioning across a join.
///
/// It detects when both sides of an equi-join are partitioned on matching column types,
/// accesses the existing partition sample from one side, and injects that synchronized sample
/// into the `PgSearchTableProvider`s of both sides. This guarantees that both sides of
/// the join produce identical partition boundaries during MPP execution.
#[derive(Debug, Default)]
pub struct RangePartitioningRule;

impl RangePartitioningRule {
    pub fn new() -> Self {
        Self
    }
}

fn pg_search_provider_from_scan(scan: &TableScan) -> Option<&PgSearchTableProvider> {
    let source = scan.source.as_ref();
    if let Some(default_source) = source.downcast_ref::<DefaultTableSource>() {
        default_source
            .table_provider
            .downcast_ref::<PgSearchTableProvider>()
    } else {
        None
    }
}

/// Recursively searches the logical plan tree to find the underlying `PgSearchTableProvider`
/// and the original index field name that corresponds to a given `Column` expression at the top level
/// of the plan. This resolves aliases, subqueries, and projections to trace a join key back to its source.
fn find_provider_for_column<'a>(
    plan: &'a LogicalPlan,
    col: &Column,
) -> Option<(&'a PgSearchTableProvider, FieldName)> {
    match plan {
        LogicalPlan::TableScan(scan) => {
            if scan.projected_schema.has_column(col) {
                let (_, sf) = scan
                    .projected_schema
                    .qualified_field_from_column(col)
                    .ok()?;
                let provider = pg_search_provider_from_scan(scan)?;
                for field in &provider.scan_info.partition_by {
                    if sf.name() == field.as_ref() {
                        return Some((provider, field.clone()));
                    }
                }
            }
            None
        }
        LogicalPlan::Projection(proj) => {
            let idx = proj.schema.index_of_column(col).ok()?;
            let expr = &proj.expr[idx];
            let unaliased = match expr {
                Expr::Alias(alias) => alias.expr.as_ref(),
                e => e,
            };
            if let Expr::Column(c) = unaliased {
                find_provider_for_column(proj.input.as_ref(), c)
            } else {
                None
            }
        }
        LogicalPlan::Filter(filter) => find_provider_for_column(filter.input.as_ref(), col),
        LogicalPlan::Sort(sort) => find_provider_for_column(sort.input.as_ref(), col),
        LogicalPlan::Limit(limit) => find_provider_for_column(limit.input.as_ref(), col),
        LogicalPlan::SubqueryAlias(alias) => {
            let idx = alias.schema.index_of_column(col).ok()?;
            let (q, f) = alias.input.schema().qualified_field(idx);
            find_provider_for_column(alias.input.as_ref(), &Column::new(q.cloned(), f.name()))
        }
        LogicalPlan::Join(join) => {
            if join.left.schema().has_column(col) {
                find_provider_for_column(join.left.as_ref(), col)
            } else if join.right.schema().has_column(col) {
                find_provider_for_column(join.right.as_ref(), col)
            } else {
                None
            }
        }
        _ => {
            let inputs = plan.inputs();
            if inputs.len() == 1 {
                let idx = plan.schema().index_of_column(col).ok()?;
                let input_schema = inputs[0].schema();
                if idx < input_schema.fields().len() {
                    let (q, f) = input_schema.qualified_field(idx);
                    find_provider_for_column(inputs[0], &Column::new(q.cloned(), f.name()))
                } else {
                    None
                }
            } else {
                None
            }
        }
    }
}

/// Recursively traverses the logical plan to locate the specific `TableScan` node
/// that produces the given `Column`. Applies the mutating function `f` to that scan
/// and rebuilds the plan tree to reflect the transformation.
fn map_scan_for_column<F>(
    plan: LogicalPlan,
    col: &Column,
    f: &mut F,
) -> Result<Transformed<LogicalPlan>>
where
    F: FnMut(TableScan) -> Result<Transformed<LogicalPlan>>,
{
    match plan {
        LogicalPlan::TableScan(scan) => {
            if scan.projected_schema.has_column(col) {
                f(scan)
            } else {
                Ok(Transformed::no(LogicalPlan::TableScan(scan)))
            }
        }
        LogicalPlan::Projection(mut proj) => {
            if let Ok(idx) = proj.schema.index_of_column(col) {
                let expr = &proj.expr[idx];
                let unaliased = match expr {
                    Expr::Alias(alias) => alias.expr.as_ref(),
                    e => e,
                };
                if let Expr::Column(c) = unaliased {
                    let res = map_scan_for_column(Arc::unwrap_or_clone(proj.input.clone()), c, f)?;
                    if res.transformed {
                        proj.input = Arc::new(res.data);
                        return Ok(Transformed::yes(LogicalPlan::Projection(proj)));
                    }
                }
            }
            Ok(Transformed::no(LogicalPlan::Projection(proj)))
        }
        LogicalPlan::Filter(mut filter) => {
            let res = map_scan_for_column(Arc::unwrap_or_clone(filter.input.clone()), col, f)?;
            if res.transformed {
                filter.input = Arc::new(res.data);
                Ok(Transformed::yes(LogicalPlan::Filter(filter)))
            } else {
                Ok(Transformed::no(LogicalPlan::Filter(filter)))
            }
        }
        LogicalPlan::Sort(mut sort) => {
            let res = map_scan_for_column(Arc::unwrap_or_clone(sort.input.clone()), col, f)?;
            if res.transformed {
                sort.input = Arc::new(res.data);
                Ok(Transformed::yes(LogicalPlan::Sort(sort)))
            } else {
                Ok(Transformed::no(LogicalPlan::Sort(sort)))
            }
        }
        LogicalPlan::Limit(mut limit) => {
            let res = map_scan_for_column(Arc::unwrap_or_clone(limit.input.clone()), col, f)?;
            if res.transformed {
                limit.input = Arc::new(res.data);
                Ok(Transformed::yes(LogicalPlan::Limit(limit)))
            } else {
                Ok(Transformed::no(LogicalPlan::Limit(limit)))
            }
        }
        LogicalPlan::SubqueryAlias(mut alias) => {
            if let Ok(idx) = alias.schema.index_of_column(col) {
                let (q, field) = alias.input.schema().qualified_field(idx);
                let inner_col = Column::new(q.cloned(), field.name());
                let res =
                    map_scan_for_column(Arc::unwrap_or_clone(alias.input.clone()), &inner_col, f)?;
                if res.transformed {
                    alias.input = Arc::new(res.data);
                    return Ok(Transformed::yes(LogicalPlan::SubqueryAlias(alias)));
                }
            }
            Ok(Transformed::no(LogicalPlan::SubqueryAlias(alias)))
        }
        LogicalPlan::Join(mut join) => {
            if join.left.schema().has_column(col) {
                let res = map_scan_for_column(Arc::unwrap_or_clone(join.left.clone()), col, f)?;
                if res.transformed {
                    join.left = Arc::new(res.data);
                    return Ok(Transformed::yes(LogicalPlan::Join(join)));
                }
            } else if join.right.schema().has_column(col) {
                let res = map_scan_for_column(Arc::unwrap_or_clone(join.right.clone()), col, f)?;
                if res.transformed {
                    join.right = Arc::new(res.data);
                    return Ok(Transformed::yes(LogicalPlan::Join(join)));
                }
            }
            Ok(Transformed::no(LogicalPlan::Join(join)))
        }
        _ => {
            let inputs = plan.inputs();
            if inputs.len() == 1
                && let Ok(idx) = plan.schema().index_of_column(col)
            {
                let input_schema = inputs[0].schema();
                if idx < input_schema.fields().len() {
                    let (q, field) = input_schema.qualified_field(idx);
                    let inner_col = Column::new(q.cloned(), field.name());

                    return plan.map_children(|child| map_scan_for_column(child, &inner_col, f));
                }
            }
            Ok(Transformed::no(plan))
        }
    }
}

fn apply_sample_to_scan(
    mut scan: TableScan,
    sample: &RangePartitioningSample,
) -> Result<Transformed<LogicalPlan>> {
    let Some(provider) = pg_search_provider_from_scan(&scan) else {
        return Ok(Transformed::no(LogicalPlan::TableScan(scan)));
    };

    if provider.range_sample() == Some(sample) {
        return Ok(Transformed::no(LogicalPlan::TableScan(scan)));
    }

    let mut new_provider = provider.clone();
    new_provider.with_range_partitioning(Some(sample.clone()));

    let new_source = Arc::new(DefaultTableSource::new(Arc::new(new_provider)))
        as Arc<dyn datafusion::logical_expr::TableSource>;

    scan.source = new_source;
    Ok(Transformed::yes(LogicalPlan::TableScan(scan)))
}

impl OptimizerRule for RangePartitioningRule {
    fn name(&self) -> &str {
        "RangePartitioningRule"
    }

    fn apply_order(&self) -> Option<ApplyOrder> {
        None
    }

    fn rewrite(
        &self,
        plan: LogicalPlan,
        _config: &dyn OptimizerConfig,
    ) -> Result<Transformed<LogicalPlan>> {
        if !crate::gucs::enable_range_partitioned_join()
            || !crate::postgres::customscan::mpp::glue::mpp_is_active()
        {
            return Ok(Transformed::no(plan));
        }

        plan.transform_up(|node| {
            if let LogicalPlan::Join(mut join) = node {
                let mut transformed = false;
                for (l, r) in &join.on {
                    if let (Expr::Column(l_col), Expr::Column(r_col)) = (l, r) {
                        let l_res = find_provider_for_column(&join.left, l_col);
                        let r_res = find_provider_for_column(&join.right, r_col);
                        if let (
                            Some((l_provider, l_field_name)),
                            Some((r_provider, r_field_name)),
                        ) = (l_res, r_res)
                        {
                            let sample = merged_sample(
                                l_provider,
                                &l_field_name,
                                r_provider,
                                &r_field_name,
                            )?;

                            if let Some(mut shared_sample) = sample {
                                shared_sample.partition_by = l_field_name;
                                let left_res = map_scan_for_column(
                                    Arc::unwrap_or_clone(join.left.clone()),
                                    l_col,
                                    &mut |scan| apply_sample_to_scan(scan, &shared_sample),
                                )?;
                                if left_res.transformed {
                                    join.left = Arc::new(left_res.data);
                                }

                                shared_sample.partition_by = r_field_name;
                                let right_res = map_scan_for_column(
                                    Arc::unwrap_or_clone(join.right.clone()),
                                    r_col,
                                    &mut |scan| apply_sample_to_scan(scan, &shared_sample),
                                )?;
                                if right_res.transformed {
                                    join.right = Arc::new(right_res.data);
                                }
                                transformed =
                                    transformed || left_res.transformed || right_res.transformed;
                            }
                        }
                    }
                }
                if transformed {
                    return Ok(Transformed::yes(LogicalPlan::Join(join)));
                } else {
                    return Ok(Transformed::no(LogicalPlan::Join(join)));
                }
            }
            Ok(Transformed::no(node))
        })
    }
}

/// Returns the arrow type of `field` when the provider exposes it as a named fast field.
fn named_field_arrow_type(
    provider: &PgSearchTableProvider,
    field: &FieldName,
) -> Option<arrow_schema::DataType> {
    provider.fields.iter().find_map(|f| match f {
        WhichFastField::Named(name, sft) if name == field.as_ref() => Some(sft.arrow_data_type()),
        _ => None,
    })
}

/// Sorts points ascending so that `RangePartitioningSample::build` produces
/// sequential ranges.
fn sort_sample_points(points: &mut [PdbOwnedValue]) {
    points.sort_unstable_by(PdbOwnedValue::total_cmp);
}

/// Samples both sides of the join and merges the two distributions, so the split
/// points reflect the combined key space rather than one side's skew.
///
/// Returns `None` when the two key columns expose different arrow types: a shared
/// sample could not describe both sides faithfully.
fn merged_sample(
    l_provider: &PgSearchTableProvider,
    l_field: &FieldName,
    r_provider: &PgSearchTableProvider,
    r_field: &FieldName,
) -> Result<Option<RangePartitioningSample>> {
    let (Some(l_type), Some(r_type)) = (
        named_field_arrow_type(l_provider, l_field),
        named_field_arrow_type(r_provider, r_field),
    ) else {
        return Ok(None);
    };
    if l_type != r_type {
        return Ok(None);
    }

    let mut sample = sample_fast_field(l_provider, l_field)?;
    let r_sample = sample_fast_field(r_provider, r_field)?;
    sample.sample_points.extend(r_sample.sample_points);
    sort_sample_points(&mut sample.sample_points);
    // A grid describes its whole table and lines up with that side's segments, so the other
    // side's sample must not dilute it. Two grids merge the way two samples do.
    if sample.persisted_points.is_empty() {
        sample.persisted_points = r_sample.persisted_points;
    } else if !r_sample.persisted_points.is_empty() {
        sample.persisted_points.extend(r_sample.persisted_points);
        sort_sample_points(&mut sample.persisted_points);
        sample
            .persisted_points
            .dedup_by(|a, b| a.total_cmp(b) == std::cmp::Ordering::Equal);
    }
    Ok(Some(sample))
}

fn sample_fast_field(
    provider: &PgSearchTableProvider,
    partition_by: &FieldName,
) -> Result<RangePartitioningSample> {
    let index_rel = PgSearchRelation::open(provider.scan_info.indexrelid);
    // A partitioned build fixed its cell boundaries up front and stamped them on its segments.
    // They describe the whole table, unlike a sample of one segment, and line up with the
    // segments. The sample stays as the fallback for layouts the grid is too coarse for.
    let persisted_points = persisted_split_points(&index_rel, partition_by.as_ref())
        .map_err(|e| DataFusionError::Internal(format!("Failed to read segment statistics: {e}")))?
        .unwrap_or_default();
    // TODO: Reading the index during planning adds latency to DataFusion logical planning.
    // This is a temporary situation for M1. In M2, we will migrate this sampling to
    // something that happens prior to CREATE INDEX, or switch to using pg_statistic.
    let reader = SearchIndexReader::open(
        &index_rel,
        SearchQueryInput::All,
        false,
        MvccSatisfies::LargestSegment,
    )
    .map_err(|e| DataFusionError::Internal(format!("Failed to open index for sampling: {e}")))?;

    let segment_readers = reader.segment_readers();
    if segment_readers.is_empty() {
        return Ok(RangePartitioningSample {
            partition_by: partition_by.clone(),
            sample_points: vec![],
            persisted_points: persisted_points.clone(),
        });
    }

    let largest_segment: &SegmentReader =
        segment_readers.iter().max_by_key(|s| s.max_doc()).unwrap();

    let max_doc = largest_segment.max_doc();
    if max_doc == 0 {
        return Ok(RangePartitioningSample {
            partition_by: partition_by.clone(),
            sample_points: vec![],
            persisted_points: persisted_points.clone(),
        });
    }

    // TODO: Validate that all partition_by columns are fast fields at `CREATE INDEX` time.
    let ff_type = FFType::new(largest_segment.fast_fields(), partition_by.as_ref());

    let search_field_type = provider.fields.iter().find_map(|f| {
        if let WhichFastField::Named(name, sft) = f
            && name == partition_by.as_ref()
        {
            return Some(*sft);
        }
        None
    });

    let target_samples = std::cmp::min(512, max_doc as usize);
    let step = (max_doc as f64) / (target_samples as f64);

    let sample_doc_ids: Vec<u32> = (0..target_samples)
        .map(|i| (i as f64 * step) as u32)
        .collect();

    let mut sample_points = Vec::with_capacity(sample_doc_ids.len());
    for doc_id in sample_doc_ids {
        let val = ff_type.value(doc_id, search_field_type);
        sample_points.push(val.0);
    }

    sort_sample_points(&mut sample_points);

    Ok(RangePartitioningSample {
        partition_by: partition_by.clone(),
        sample_points,
        persisted_points,
    })
}

/// Physical optimizer rule that converts a `CollectLeft` inner hash join to
/// `Partitioned` mode when both inputs declare compatible `Partitioning::Range`
/// layouts on the join keys.
///
/// A `CollectLeft` join materializes the entire build side in every consumer,
/// which the distributed planner satisfies by broadcasting it across tasks. When
/// both sides are range partitioned with identical split points, build-side
/// partition `i` can only ever match probe-side partition `i`, so `Partitioned`
/// mode joins each pair task-locally and the broadcast disappears.
///
/// A separate rule because `JoinSelection` picks `CollectLeft` from the build
/// side's row and byte statistics alone. It never consults `output_partitioning`,
/// so it can't see that these inputs are already co-partitioned and that the
/// repartition it's avoiding would cost nothing here. Declaring
/// `Partitioning::Range` on the scans only helps a join that is already
/// `Partitioned`, so the mode has to be revisited after the fact. Runs after
/// `EnsureRequirements` has resolved `PartitionMode::Auto` and inserted the build
/// side's `CoalescePartitionsExec`.
#[derive(Debug, Default)]
pub struct RangeCoPartitionedJoinRule;

impl PhysicalOptimizerRule for RangeCoPartitionedJoinRule {
    fn name(&self) -> &str {
        "RangeCoPartitionedJoinRule"
    }

    fn schema_check(&self) -> bool {
        true
    }

    fn optimize(
        &self,
        plan: Arc<dyn ExecutionPlan>,
        _config: &ConfigOptions,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        if !crate::gucs::enable_range_partitioned_join() {
            return Ok(plan);
        }

        plan.transform_up(|node| {
            let Some(join) = node.downcast_ref::<HashJoinExec>() else {
                return Ok(Transformed::no(node));
            };
            // `HashJoinExec` enables `allow_range_satisfaction_for_key_partitioning`
            // only for Partitioned inner joins; flipping any other shape would make
            // `EnforceDistribution` re-introduce hash repartitions on both sides.
            if join.partition_mode() != &PartitionMode::CollectLeft
                || join.join_type() != &JoinType::Inner
            {
                return Ok(Transformed::no(node));
            }

            // EnforceDistribution collapses the build side to a single partition for
            // CollectLeft; peel that off to recover the source's declared partitioning.
            let left = match join.left().downcast_ref::<CoalescePartitionsExec>() {
                Some(coalesce) if coalesce.fetch().is_none() => Arc::clone(coalesce.input()),
                _ => Arc::clone(join.left()),
            };
            let right = Arc::clone(join.right());

            let range_partitioned = |input: &Arc<dyn ExecutionPlan>| {
                matches!(input.output_partitioning(), Partitioning::Range(_))
                    && input.output_partitioning().partition_count() > 1
            };
            if !range_partitioned(&left) || !range_partitioned(&right) {
                return Ok(Transformed::no(node));
            }

            // Deliberately not `reset_state()`: it would drop the join's handle on the
            // dynamic filter that `FilterPushdown` already pushed into the probe scan,
            // leaving the scan holding a filter that nothing ever narrows. Switching the
            // mode invalidates the cached properties on its own.
            let candidate = join
                .builder()
                .with_new_children(vec![left, right])?
                .with_partition_mode(PartitionMode::Partitioned)
                .build_exec()?;

            // Keep the flip only when DataFusion agrees the inputs are co-partitioned:
            // the join keys match the range keys through equivalences and the split
            // points are identical on both sides. Anything else keeps the CollectLeft
            // join (and its broadcast) untouched.
            let children: Vec<&dyn ExecutionPlan> = candidate
                .children()
                .into_iter()
                .map(|child| child.as_ref())
                .collect();
            let co_partitioned = candidate
                .input_distribution_requirements()
                .unsatisfied_co_partitioned_children(candidate.name(), &children)?
                .is_empty();
            if co_partitioned {
                Ok(Transformed::yes(candidate))
            } else {
                Ok(Transformed::no(node))
            }
        })
        .map(|transformed| transformed.data)
    }
}

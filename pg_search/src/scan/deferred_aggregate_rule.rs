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

//! Aggregates on term ordinals.
//!
//! A `GROUP BY` on a late-materialized string column decodes every input row before the
//! hash table sees it, and the hash table then stores, hashes and compares strings. Within
//! one segment, rows with the same string share one term ordinal, and an ordinal is twelve
//! bytes. So a partial aggregate on ordinals reduces the input to at most one row per
//! distinct term per segment before anything is decoded:
//!
//! ```text
//! AggregateExec(Single, gby=[category])          AggregateExec(Final, gby=[category])
//!   TantivyDecodeExec(decode=[category])    =>     TantivyDecodeExec(decode=[category])
//!     input                                          AggregateExec(Partial, gby=[category])
//!                                                      input
//! ```
//!
//! The partial group key is the deferred column itself: one `UInt64` per row that packs the
//! segment with the ordinal, so two rows compare equal only when both match, which is what a
//! per-segment group needs, and the hash table works on primitives. The decode then runs
//! once per partial group, and the final aggregate merges the segments' groups by string. A
//! NULL string is a NULL key, which the final aggregate merges like any other group.
//!
//! A row that reaches the partial aggregate as a doc address (State 0) would group per
//! document and reduce nothing, so a column only moves when a fetch below the aggregate,
//! or the scan itself, resolves it first. A column moves only when the aggregate reads it
//! as a plain group key and nowhere else: an aggregate argument, filter or ordering wants
//! the string, so such a column keeps its decode below. Grouping sets are left alone, since
//! their partial output carries a grouping id the final aggregate reads back.
//!
//! The rewrite pays when the groups are far fewer than the rows. The dictionaries bound the
//! groups from above (one per distinct term per segment), and the plan's row estimate says
//! how many rows reach the aggregate, so a key whose dictionaries are nearly as large as
//! the input keeps its decode below: its partial aggregate would reduce little and its
//! decode would run about as often as the scan's.
//!
//! [`DeferredPlacementRule`] runs first and asks [`ordinal_group_keys`] the same question,
//! so a join that multiplies the rows under such an aggregate keeps the decode deferred
//! instead of pushing it into the scan.
//!
//! [`DeferredPlacementRule`]: crate::scan::deferred_placement_rule::DeferredPlacementRule

use std::sync::Arc;

use datafusion::common::Result;
use datafusion::common::config::ConfigOptions;
use datafusion::common::stats::Precision;
use datafusion::physical_expr::expressions::Column;
use datafusion::physical_expr::utils::collect_columns;
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::aggregates::{AggregateExec, AggregateMode};
use datafusion::physical_plan::{
    ChildrenPropertiesMode, ExecutionPlan, ReplaceChildrenOptions, StatisticsArgs,
    StatisticsContext,
};

use crate::api::HashSet;
use crate::index::fast_fields_helper::FFType;
use crate::scan::deferred_lookup::PhysicalDeferredField;
use crate::scan::execution_plan::PgSearchScanPlan;
use crate::scan::tantivy_decode_exec::TantivyDecodeExec;
use crate::scan::tantivy_fetch_exec::TantivyFetchExec;

#[derive(Debug)]
pub struct DeferredAggregateRule;

impl PhysicalOptimizerRule for DeferredAggregateRule {
    fn optimize(
        &self,
        plan: Arc<dyn ExecutionPlan>,
        _config: &ConfigOptions,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        if !crate::gucs::enable_aggregate_late_materialization() {
            return Ok(plan);
        }
        rewrite(plan)
    }

    fn name(&self) -> &str {
        "DeferredAggregateRule"
    }

    fn schema_check(&self) -> bool {
        true
    }
}

/// Rebuilds the plan bottom-up. The two-phase shape keeps an aggregate's output schema, so
/// the ancestors only need their children swapped.
fn rewrite(node: Arc<dyn ExecutionPlan>) -> Result<Arc<dyn ExecutionPlan>> {
    let children = node.children();
    let mut new_children = Vec::with_capacity(children.len());
    let mut changed = false;
    for child in children {
        let new_child = rewrite(Arc::clone(child))?;
        changed |= !Arc::ptr_eq(child, &new_child);
        new_children.push(new_child);
    }
    let node = if changed {
        node.replace_children(
            new_children,
            ReplaceChildrenOptions::new(ChildrenPropertiesMode::Recompute),
        )?
    } else {
        node
    };
    if let Some(agg) = node.downcast_ref::<AggregateExec>()
        && let Some(two_phase) = two_phase(agg)?
    {
        return Ok(two_phase);
    }
    Ok(node)
}

/// Indexes into `decode.deferred_fields()` of the columns that `agg` reads only as plain
/// group keys, that arrive at the aggregate already resolved to term ordinals, and whose
/// dictionaries are small next to the aggregate's input.
pub(crate) fn ordinal_group_keys(agg: &AggregateExec, decode: &TantivyDecodeExec) -> Vec<usize> {
    if !matches!(agg.mode(), AggregateMode::Single | AggregateMode::Partial)
        || agg.limit_options().is_some()
        || !agg.group_expr().is_single()
    {
        return Vec::new();
    }
    let mut keys: HashSet<usize> = HashSet::default();
    let mut elsewhere: HashSet<usize> = HashSet::default();
    for (expr, _) in agg.group_expr().expr() {
        match expr.downcast_ref::<Column>() {
            Some(col) => {
                keys.insert(col.index());
            }
            None => elsewhere.extend(collect_columns(expr).iter().map(|c| c.index())),
        }
    }
    for aggr in agg.aggr_expr() {
        for expr in aggr.expressions() {
            elsewhere.extend(collect_columns(&expr).iter().map(|c| c.index()));
        }
        for sort in aggr.order_bys() {
            elsewhere.extend(collect_columns(&sort.expr).iter().map(|c| c.index()));
        }
    }
    for filter in agg.filter_expr().iter().flatten() {
        elsewhere.extend(collect_columns(filter).iter().map(|c| c.index()));
    }
    let rows = input_rows(agg);
    decode
        .deferred_fields()
        .iter()
        .enumerate()
        .filter(|(_, field)| keys.contains(&field.col_idx) && !elsewhere.contains(&field.col_idx))
        .filter(|(_, field)| resolved_below(decode, field))
        .filter(|(_, field)| reduces_enough(rows, decode, field))
        .map(|(i, _)| i)
        .collect()
}

/// The estimated number of rows entering `agg`, when the plan has one.
fn input_rows(agg: &AggregateExec) -> Option<usize> {
    let statistics = StatisticsContext::new()
        .compute(agg.input().as_ref(), &StatisticsArgs::new())
        .ok()?;
    match statistics.num_rows {
        Precision::Exact(rows) | Precision::Inexact(rows) => Some(rows),
        Precision::Absent => None,
    }
}

/// Input rows per dictionary term below which the two-phase shape is not worth its second
/// hash pass: the partial aggregate could reduce the rows by less than this factor, and
/// the decode of its groups would run about as often as the scan's.
const MIN_ROWS_PER_TERM: usize = 4;

/// Whether `rows` outnumber the terms of `field`'s dictionaries by [`MIN_ROWS_PER_TERM`].
/// An input without an estimate keeps the rewrite.
fn reduces_enough(
    rows: Option<usize>,
    decode: &TantivyDecodeExec,
    field: &PhysicalDeferredField,
) -> bool {
    let Some(rows) = rows else {
        return true;
    };
    rows >= dictionary_terms(decode, field).saturating_mul(MIN_ROWS_PER_TERM)
}

/// Whether `field` reaches `decode` as a term ordinal: a fetch directly below resolves it,
/// or every scan of its index resolves it itself.
fn resolved_below(decode: &TantivyDecodeExec, field: &PhysicalDeferredField) -> bool {
    let input = decode.children()[0];
    if let Some(fetch) = input.downcast_ref::<TantivyFetchExec>()
        && fetch
            .fetch_fields()
            .iter()
            .any(|f| f.canonical == field.canonical && f.col_idx == field.col_idx)
    {
        return true;
    }
    let mut scans = Vec::new();
    collect_scans(input, field.canonical.indexrelid, &mut scans);
    !scans.is_empty()
        && scans.iter().all(|scan| {
            scan.deferred_fields()
                .iter()
                .any(|d| d.canonical == field.canonical && d.fetch_at_scan)
        })
}

fn collect_scans<'a>(
    node: &'a Arc<dyn ExecutionPlan>,
    indexrelid: u32,
    out: &mut Vec<&'a PgSearchScanPlan>,
) {
    if let Some(scan) = node.downcast_ref::<PgSearchScanPlan>() {
        if scan.indexrelid == indexrelid {
            out.push(scan);
        }
        return;
    }
    for child in node.children() {
        collect_scans(child, indexrelid, out);
    }
}

/// Splits `agg` into a partial aggregate on ordinals, the decode of its groups, and the
/// final aggregate on strings. A `Partial` aggregate already has its final above, so it
/// only gets the decode lifted over it.
fn two_phase(agg: &AggregateExec) -> Result<Option<Arc<dyn ExecutionPlan>>> {
    let Some(decode) = agg.input().downcast_ref::<TantivyDecodeExec>() else {
        return Ok(None);
    };
    let lifted = ordinal_group_keys(agg, decode);
    if lifted.is_empty() {
        return Ok(None);
    }
    let (moved, kept): (Vec<_>, Vec<_>) = decode
        .deferred_fields()
        .iter()
        .cloned()
        .enumerate()
        .partition(|(i, _)| lifted.contains(i));
    let kept: Vec<PhysicalDeferredField> = kept.into_iter().map(|(_, f)| f).collect();
    let moved: Vec<PhysicalDeferredField> = moved.into_iter().map(|(_, f)| f).collect();

    let below: Arc<dyn ExecutionPlan> = if kept.is_empty() {
        Arc::clone(decode.children()[0])
    } else {
        Arc::new(decode.with_input_and_fields(Arc::clone(decode.children()[0]), kept)?)
    };
    let below_schema = below.schema();
    let partial = Arc::new(AggregateExec::try_new(
        AggregateMode::Partial,
        agg.group_expr().clone(),
        agg.aggr_expr().to_vec(),
        agg.filter_expr().to_vec(),
        below,
        Arc::clone(&below_schema),
    )?);

    // The partial output lays the group keys out first, in group expression order.
    let mut lifted_fields = Vec::new();
    for (i, (expr, _)) in partial.group_expr().expr().iter().enumerate() {
        if let Some(col) = expr.downcast_ref::<Column>()
            && let Some(field) = moved.iter().find(|f| f.col_idx == col.index())
        {
            let mut field = field.clone();
            field.col_idx = i;
            lifted_fields.push(field);
        }
    }
    let decoded: Arc<dyn ExecutionPlan> = Arc::new(TantivyDecodeExec::new(
        Arc::clone(&partial) as Arc<dyn ExecutionPlan>,
        lifted_fields,
        decode.ffhelpers().clone(),
    )?);
    if *agg.mode() == AggregateMode::Partial {
        return Ok(Some(decoded));
    }

    let final_agg = AggregateExec::try_new(
        AggregateMode::Final,
        partial.group_expr().as_final(),
        partial.aggr_expr().to_vec(),
        agg.filter_expr().to_vec(),
        decoded,
        below_schema,
    )?;
    if final_agg.schema() != agg.schema() {
        pgrx::debug1!(
            "DeferredAggregate: the two-phase schema differs from the aggregate's, keeping the plan"
        );
        return Ok(None);
    }
    Ok(Some(Arc::new(final_agg)))
}

/// The number of terms in `field`'s dictionaries across its index's segments, which is the
/// most partial groups the column can produce.
fn dictionary_terms(decode: &TantivyDecodeExec, field: &PhysicalDeferredField) -> usize {
    let Some(ffhelper) = decode.ffhelper(field.canonical.indexrelid) else {
        return 0;
    };
    (0..ffhelper.num_segments())
        .map(
            |segment_ord| match ffhelper.column(segment_ord as u32, field.canonical.ff_index) {
                FFType::Text(column) => column.num_terms(),
                FFType::Bytes(column) => column.num_terms(),
                _ => 0,
            },
        )
        .sum()
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::api::HashMap;
    use crate::index::fast_fields_helper::{CanonicalColumn, FFHelper};
    use crate::query::SearchQueryInput;
    use crate::scan::deferred_encode::{deferred_data_type, deferred_field};
    use crate::scan::late_materialization::DeferredField;
    use arrow_schema::{DataType, Field, Schema};
    use datafusion::functions_aggregate::count::count_udaf;
    use datafusion::physical_expr::aggregate::{AggregateExprBuilder, AggregateFunctionExpr};
    use datafusion::physical_expr::expressions::lit;
    use datafusion::physical_plan::aggregates::PhysicalGroupBy;
    use pgrx::prelude::*;

    const INDEXRELID: u32 = 42;

    /// A scan that resolves `category` to term ordinals itself, under a decode of it.
    fn decode_over_scan() -> Arc<dyn ExecutionPlan> {
        let canonical = CanonicalColumn {
            indexrelid: INDEXRELID,
            ff_index: 1,
        };
        let scan = Arc::new(PgSearchScanPlan::new(
            None,
            Arc::new(Schema::new(vec![
                Field::new("id", DataType::Int64, true),
                deferred_field("category"),
            ])),
            SearchQueryInput::All,
            None,
            vec![DeferredField {
                name: "category".into(),
                is_bytes: false,
                canonical: canonical.clone(),
                rebuild: None,
                fetch_at_scan: true,
            }],
            Some(Arc::new(FFHelper::empty())),
            INDEXRELID,
            None,
            1,
            None,
            None,
        )) as Arc<dyn ExecutionPlan>;
        let mut ffhelpers = HashMap::default();
        ffhelpers.insert(INDEXRELID, Arc::new(FFHelper::empty()));
        Arc::new(
            TantivyDecodeExec::new(
                scan,
                vec![PhysicalDeferredField {
                    col_idx: 1,
                    display_name: "category".into(),
                    is_bytes: false,
                    canonical,
                    rebuild: None,
                }],
                ffhelpers,
            )
            .unwrap(),
        )
    }

    fn count_star(input: &Arc<dyn ExecutionPlan>) -> Arc<AggregateFunctionExpr> {
        AggregateExprBuilder::new(count_udaf(), vec![lit(1i64)])
            .schema(input.schema())
            .alias("agg_0")
            .build()
            .map(Arc::new)
            .unwrap()
    }

    fn group_by_category(
        input: Arc<dyn ExecutionPlan>,
        aggr: Vec<Arc<AggregateFunctionExpr>>,
    ) -> Arc<dyn ExecutionPlan> {
        let filters = vec![None; aggr.len()];
        let schema = input.schema();
        Arc::new(
            AggregateExec::try_new(
                AggregateMode::Single,
                PhysicalGroupBy::new_single(vec![(
                    Arc::new(Column::new("category", 1)),
                    "category".into(),
                )]),
                aggr,
                filters,
                input,
                schema,
            )
            .unwrap(),
        )
    }

    #[pg_test]
    fn a_group_key_is_aggregated_on_ordinals_and_decoded_per_group() {
        let decode = decode_over_scan();
        let aggr = vec![count_star(&decode)];
        let single = group_by_category(decode, aggr);

        let rewritten = rewrite(Arc::clone(&single)).unwrap();

        let final_agg = rewritten
            .downcast_ref::<AggregateExec>()
            .expect("a final aggregate on top");
        assert_eq!(*final_agg.mode(), AggregateMode::Final);
        assert_eq!(rewritten.schema(), single.schema());
        let decode = final_agg.input();
        assert!(
            decode.is::<TantivyDecodeExec>(),
            "the decode sits under the final"
        );
        assert_eq!(decode.schema().field(0).data_type(), &DataType::Utf8View);
        let partial = decode.children()[0]
            .downcast_ref::<AggregateExec>()
            .expect("a partial aggregate under the decode");
        assert_eq!(*partial.mode(), AggregateMode::Partial);
        assert_eq!(partial.schema().field(0).data_type(), &deferred_data_type());
        assert!(
            partial.input().is::<PgSearchScanPlan>(),
            "the scan feeds the partial aggregate its ordinals"
        );
    }

    #[pg_test]
    fn a_key_the_aggregate_also_reads_stays_decoded_below() {
        let decode = decode_over_scan();
        let count_category =
            AggregateExprBuilder::new(count_udaf(), vec![Arc::new(Column::new("category", 1))])
                .schema(decode.schema())
                .alias("agg_0")
                .build()
                .map(Arc::new)
                .unwrap();
        let single = group_by_category(decode, vec![count_category]);

        let rewritten = rewrite(Arc::clone(&single)).unwrap();

        assert!(Arc::ptr_eq(&rewritten, &single), "the plan is left alone");
    }
}

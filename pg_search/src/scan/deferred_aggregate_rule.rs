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
//! The partial group key is the deferred union itself. The row format encodes a dense
//! union, and two rows compare equal only when segment and ordinal match, which is what a
//! per-segment group needs. The decode then runs once per partial group, and the final
//! aggregate merges the segments' groups by string. A NULL string is a NULL ordinal in
//! every segment, so the final aggregate merges those groups like any other.
//!
//! A row that reaches the partial aggregate as a doc address (State 0) would group per
//! document and reduce nothing. The placement leaves every column at a decode point
//! resolved, by a fetch below it or by the scan itself, so a group key that arrives
//! unresolved is a planning error and the rule fails rather than leaving the decode where
//! it is. A column moves only when the aggregate reads it as a plain group key and nowhere
//! else: an aggregate argument, filter or ordering wants the string, so such a column keeps
//! its decode below. Taking those onto ordinals as well needs the segment as an extra group
//! key, since ordinals compare only within one segment, and a second aggregate over the
//! decoded groups in place of a final one, since a merged state would hold ordinals from
//! different segments. A grouping set fills a key with a typed NULL for the sets that leave
//! it out; the partial groups on ordinals, so that NULL becomes an ordinal NULL, and the
//! grouping id passes through untouched.
//!
//! The rewrite pays when the groups are far fewer than the rows. The dictionaries bound the
//! groups from above (one per distinct term per segment), so a key whose dictionaries are
//! nearly as large as its own table keeps its decode below: its partial aggregate would
//! reduce little and its decode would run about as often as the scan's. Both sides of that
//! comparison come from the scan, since the row count a join reports is its smaller input
//! rather than its output.
//!
//! [`DeferredPlacementRule`] runs first and asks [`ordinal_group_keys`] the same question,
//! so a join that multiplies the rows under such an aggregate keeps the decode deferred
//! instead of pushing it into the scan.
//!
//! [`DeferredPlacementRule`]: crate::scan::deferred_placement_rule::DeferredPlacementRule

use std::sync::Arc;

use datafusion::common::config::ConfigOptions;
use datafusion::common::{Result, ScalarValue, internal_err};
use datafusion::physical_expr::expressions::{Column, lit};
use datafusion::physical_expr::utils::collect_columns;
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::aggregates::{AggregateExec, AggregateMode, PhysicalGroupBy};
use datafusion::physical_plan::{ChildrenPropertiesMode, ExecutionPlan, ReplaceChildrenOptions};

use crate::api::HashSet;
use crate::index::fast_fields_helper::FFType;
use crate::scan::deferred_encode::deferred_data_type;
use crate::scan::deferred_lookup::PhysicalDeferredField;
use crate::scan::deferred_placement_rule::same_columns;
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
/// group keys and whose dictionaries are small next to the aggregate's input.
///
/// A key that reaches the decode as doc addresses is an error, not a column to skip: the
/// placement resolves every deferred column below its decode point, so a missing fetch
/// means an earlier rule broke that.
pub(crate) fn ordinal_group_keys(
    agg: &AggregateExec,
    decode: &TantivyDecodeExec,
) -> Result<Vec<usize>> {
    if !matches!(agg.mode(), AggregateMode::Single | AggregateMode::Partial)
        || agg.limit_options().is_some()
    {
        return Ok(Vec::new());
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
    let mut lifted = Vec::new();
    for (i, field) in decode.deferred_fields().iter().enumerate() {
        if !keys.contains(&field.col_idx) || elsewhere.contains(&field.col_idx) {
            continue;
        }
        if !resolved_below(decode, field) {
            return internal_err!(
                "DeferredAggregate: group key '{}' reaches the aggregate as doc addresses, nothing below the decode resolves it",
                field.display_name
            );
        }
        if reduces_enough(decode, field) {
            lifted.push(i);
        }
    }
    Ok(lifted)
}

/// The rows the scan of `field`'s relation expects to emit.
///
/// The dictionaries this is weighed against are a whole-index count, so its counterpart has
/// to be one too. The rows reaching the aggregate are not that count: with no distinct count
/// on either join key, DataFusion estimates an equi-join's output as its smaller input, so a
/// join between a search-filtered side and a whole table reads as a handful of rows and every
/// key of the large side reads as "no reduction", however small its dictionary is.
fn scanned_rows(decode: &TantivyDecodeExec, field: &PhysicalDeferredField) -> Option<usize> {
    let mut scans = Vec::new();
    collect_scans(decode.children()[0], field.heap_rti, &mut scans);
    scans
        .iter()
        .map(|scan| scan.planner_estimated_rows() as usize)
        .max()
        .filter(|rows| *rows > 0)
}

/// Scanned rows per dictionary term below which the partial aggregate is not worth its own
/// hash pass. The rewrite trades one decode and one string hash per input row for one packed
/// hash per input row plus one decode and one string hash per group, and the terms bound the
/// groups. Near one row per term that is a loss: the groups' decode then walks about as much
/// of the dictionary as the scan's own decode would, with the packed pass on top. The floor
/// comes from the benchmark on a near-unique key, where the rewrite lost at one row per term
/// and paid off from a handful up. It is measured, not derived, so it is a constant and not a
/// setting nobody could size from the outside.
const MIN_ROWS_PER_TERM: usize = 4;

/// Whether the scanned rows outnumber the terms of `field`'s dictionaries by
/// [`MIN_ROWS_PER_TERM`]. A source without an estimate keeps the rewrite.
fn reduces_enough(decode: &TantivyDecodeExec, field: &PhysicalDeferredField) -> bool {
    let Some(rows) = scanned_rows(decode, field) else {
        return true;
    };
    rows >= dictionary_terms(decode, field).saturating_mul(MIN_ROWS_PER_TERM)
}

/// Whether `field` reaches `decode` as a term ordinal: a fetch below resolves it, with only
/// nodes that keep the columns in place between the two, or its own scan resolves it.
fn resolved_below(decode: &TantivyDecodeExec, field: &PhysicalDeferredField) -> bool {
    let input = decode.children()[0];
    let mut node = input;
    loop {
        if let Some(fetch) = node.downcast_ref::<TantivyFetchExec>() {
            if fetch
                .fetch_fields()
                .iter()
                .any(|f| f.canonical == field.canonical && f.col_idx == field.col_idx)
            {
                return true;
            }
            break;
        }
        let children = node.children();
        if children.len() != 1 || !same_columns(node, children[0]) {
            break;
        }
        node = children[0];
    }
    let mut scans = Vec::new();
    collect_scans(input, field.heap_rti, &mut scans);
    !scans.is_empty()
        && scans.iter().all(|scan| {
            scan.deferred_fields()
                .iter()
                .any(|d| d.canonical == field.canonical && d.fetch_at_scan)
        })
}

/// Every scan of the range table entry `field` came from. The index is not enough: a
/// self-join reads one index through two scans, and each one resolves its own columns.
fn collect_scans<'a>(
    node: &'a Arc<dyn ExecutionPlan>,
    heap_rti: u32,
    out: &mut Vec<&'a PgSearchScanPlan>,
) {
    if let Some(scan) = node.downcast_ref::<PgSearchScanPlan>() {
        if scan
            .deferred_fields()
            .iter()
            .any(|d| d.heap_rti == heap_rti)
        {
            out.push(scan);
        }
        return;
    }
    for child in node.children() {
        collect_scans(child, heap_rti, out);
    }
}

/// Splits `agg` into a partial aggregate on ordinals, the decode of its groups, and the
/// final aggregate on strings. A `Partial` aggregate already has its final above, so it
/// only gets the decode lifted over it.
fn two_phase(agg: &AggregateExec) -> Result<Option<Arc<dyn ExecutionPlan>>> {
    let Some(decode) = agg.input().downcast_ref::<TantivyDecodeExec>() else {
        return Ok(None);
    };
    let lifted = ordinal_group_keys(agg, decode)?;
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
    let group_by = agg.group_expr();
    let lifted_positions: HashSet<usize> = group_by
        .expr()
        .iter()
        .enumerate()
        .filter(|(_, (expr, _))| {
            expr.downcast_ref::<Column>()
                .is_some_and(|col| moved.iter().any(|f| f.col_idx == col.index()))
        })
        .map(|(i, _)| i)
        .collect();
    // A grouping set fills a key with a typed NULL for the sets that leave it out. The
    // partial groups on ordinals, so a lifted key's NULL has to be an ordinal NULL as well.
    let null_expr = group_by
        .null_expr()
        .iter()
        .enumerate()
        .map(|(i, (expr, name))| {
            let expr = if lifted_positions.contains(&i) {
                lit(ScalarValue::try_from(&deferred_data_type())?)
            } else {
                Arc::clone(expr)
            };
            Ok((expr, name.clone()))
        })
        .collect::<Result<Vec<_>>>()?;
    let partial = Arc::new(AggregateExec::try_new(
        AggregateMode::Partial,
        PhysicalGroupBy::new(
            group_by.expr().to_vec(),
            null_expr,
            group_by.groups().to_vec(),
            group_by.has_grouping_set(),
        ),
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

    // The stream applies filters only while it reads raw rows; a final aggregate merges
    // states and gets none.
    let final_agg = AggregateExec::try_new(
        AggregateMode::Final,
        partial.group_expr().as_final(),
        partial.aggr_expr().to_vec(),
        vec![None; partial.aggr_expr().len()],
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
    use datafusion::physical_expr::expressions::Literal;
    use datafusion::physical_plan::union::UnionExec;
    use pgrx::prelude::*;

    const INDEXRELID: u32 = 42;

    fn canonical() -> CanonicalColumn {
        CanonicalColumn {
            indexrelid: INDEXRELID,
            ff_index: 1,
        }
    }

    /// A scan of range table entry `heap_rti` with a deferred `category`, resolved to term
    /// ordinals by the scan itself when `fetch_at_scan`.
    fn scan(heap_rti: u32, fetch_at_scan: bool) -> Arc<dyn ExecutionPlan> {
        Arc::new(PgSearchScanPlan::new(
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
                canonical: canonical(),
                heap_rti,
                rebuild: None,
                fetch_at_scan,
            }],
            Some(Arc::new(FFHelper::empty())),
            INDEXRELID,
            None,
            1,
            None,
            None,
        ))
    }

    /// A decode of the `category` that came from range table entry `heap_rti`.
    fn decode_over(input: Arc<dyn ExecutionPlan>, heap_rti: u32) -> Arc<dyn ExecutionPlan> {
        let mut ffhelpers = HashMap::default();
        ffhelpers.insert(INDEXRELID, Arc::new(FFHelper::empty()));
        Arc::new(
            TantivyDecodeExec::new(
                input,
                vec![PhysicalDeferredField {
                    col_idx: 1,
                    display_name: "category".into(),
                    is_bytes: false,
                    canonical: canonical(),
                    heap_rti,
                    rebuild: None,
                }],
                ffhelpers,
            )
            .unwrap(),
        )
    }

    fn decode_over_scan() -> Arc<dyn ExecutionPlan> {
        decode_over(scan(1, true), 1)
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

    /// A grouping set's NULL literal is typed. The partial groups on ordinals, so the literal
    /// it carries for the sets without the key must be an ordinal NULL, and the grouping id
    /// keeps its place after the keys.
    #[pg_test]
    fn a_grouping_set_key_is_grouped_on_ordinals_with_an_ordinal_null() {
        let decode = decode_over_scan();
        let aggr = vec![count_star(&decode)];
        let sets = PhysicalGroupBy::new(
            vec![(Arc::new(Column::new("category", 1)), "category".into())],
            vec![(lit(ScalarValue::Utf8View(None)), "category".into())],
            vec![vec![true], vec![false]],
            true,
        );
        let schema = decode.schema();
        let partial = Arc::new(
            AggregateExec::try_new(
                AggregateMode::Partial,
                sets,
                aggr,
                vec![None],
                decode,
                schema,
            )
            .unwrap(),
        ) as Arc<dyn ExecutionPlan>;

        let rewritten = rewrite(Arc::clone(&partial)).unwrap();

        assert!(
            rewritten.is::<TantivyDecodeExec>(),
            "the decode is lifted over the partial"
        );
        assert_eq!(rewritten.schema(), partial.schema());
        let ordinal_partial = rewritten.children()[0]
            .downcast_ref::<AggregateExec>()
            .expect("a partial aggregate under the decode");
        let (null_literal, _) = &ordinal_partial.group_expr().null_expr()[0];
        let null_literal = null_literal
            .downcast_ref::<Literal>()
            .expect("the set's NULL stays a literal");
        assert_eq!(null_literal.value().data_type(), deferred_data_type());
        assert_eq!(
            ordinal_partial.schema().field(0).data_type(),
            &deferred_data_type()
        );
        assert_eq!(ordinal_partial.schema().field(1).name(), "__grouping_id");
    }

    /// Two scans of one index under a union: the key of the range table entry whose scan
    /// resolves it is lifted, and the key of the other one is a planning error. Keyed on the
    /// index, the second scan would hide the first.
    #[pg_test]
    fn keys_are_resolved_per_range_table_entry_not_per_index() {
        let union = UnionExec::try_new(vec![scan(1, true), scan(2, false)]).unwrap();

        let resolved = decode_over(Arc::clone(&union), 1);
        let single = group_by_category(Arc::clone(&resolved), vec![count_star(&resolved)]);
        let rewritten = rewrite(single).unwrap();
        assert!(
            rewritten
                .downcast_ref::<AggregateExec>()
                .is_some_and(|agg| *agg.mode() == AggregateMode::Final),
            "the resolved entry's key is grouped on ordinals"
        );

        let unresolved = decode_over(union, 2);
        let single = group_by_category(Arc::clone(&unresolved), vec![count_star(&unresolved)]);
        let err = rewrite(single).expect_err("an unresolved key is a planning error");
        assert!(err.to_string().contains("doc addresses"), "{err}");
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

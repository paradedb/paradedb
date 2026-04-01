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

use crate::api::SortDirection;
use crate::api::{FieldName, HashSet, OrderByFeature};
use crate::gucs;
use crate::postgres::customscan::aggregatescan::aggregate_type::AggregateType;
use crate::postgres::customscan::aggregatescan::filterquery::{new_filter_query, FilterQuery};
use crate::postgres::customscan::aggregatescan::orderby::OrderByClause;
use crate::postgres::customscan::aggregatescan::searchquery::SearchQueryClause;
use crate::postgres::customscan::aggregatescan::targetlist::{
    find_single_aggref_in_expr, TargetList, TargetListEntry,
};
use crate::postgres::customscan::aggregatescan::{
    AggregateScan, CustomScanBuildError, CustomScanClause,
};
use crate::postgres::customscan::aggregatescan::{GroupByClause, GroupingColumn};
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::explain::cleanup_json_for_explain;
use crate::postgres::customscan::CreateUpperPathsHookArgs;
use crate::postgres::customscan::CustomScan;
use crate::postgres::utils::sort_json_keys;
use crate::postgres::PgSearchRelation;
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;

use crate::postgres::customscan::limit_offset::LimitOffset;
use anyhow::Result;
use pgrx::pg_sys;
use pgrx::PgList;
use tantivy::aggregation::agg_req::Aggregations;
use tantivy::aggregation::agg_req::{Aggregation, AggregationVariants};
use tantivy::aggregation::bucket::{CustomOrder, OrderTarget, TermsAggregation};
use tantivy::aggregation::metric::CountAggregation;

pub trait AggregationKey {
    const NAME: &'static str;
}

pub struct DocCountKey;
impl AggregationKey for DocCountKey {
    const NAME: &'static str = "_doc_count";
}

pub struct GroupedKey;
impl AggregationKey for GroupedKey {
    const NAME: &'static str = "grouped";
}

pub struct FilterSentinelKey;
impl AggregationKey for FilterSentinelKey {
    const NAME: &'static str = "filter_sentinel";
}

/// Identifies which aggregate metric the ORDER BY targets.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AggregateMetricTarget {
    /// COUNT(*) — uses `OrderTarget::Count`. Cannot produce NULLs,
    /// so the `size=K` optimization is always safe.
    Count,
    /// A non-COUNT metric (SUM, AVG, MIN, MAX, etc.) at the given
    /// sub-aggregation position. Can produce NULLs when all grouped
    /// values are NULL, which means `size=K` may prune NULL groups
    /// that Postgres's NULLS FIRST/LAST would include — so the
    /// `size=K` limit is disabled for this variant.
    Metric(usize),
}

/// ORDER BY on an aggregate metric for TopK optimization.
/// Allows pushing LIMIT into Tantivy's TermsAggregation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AggregateOrderBy {
    pub target: AggregateMetricTarget,
    pub direction: SortDirection,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AggregateCSClause {
    targetlist: TargetList,
    orderby: OrderByClause,
    limit_offset: LimitOffset,
    quals: SearchQueryClause,
    indexrelid: pg_sys::Oid,
    is_execution_time: bool,
    aggregate_orderby: Option<AggregateOrderBy>,
}

trait CollectNested<Key: AggregationKey> {
    fn iter_leaves(&self) -> Result<impl Iterator<Item = AggregationVariants>>;

    fn collect(
        &self,
        mut aggregations: Aggregations,
        children: Aggregations,
    ) -> Result<Aggregations> {
        let groupings: Vec<_> = self.iter_leaves()?.collect();

        let nested = groupings.into_iter().rfold(children, |sub, leaf| {
            Aggregations::from_iter([(
                GroupedKey::NAME.to_string(),
                Aggregation {
                    agg: leaf,
                    sub_aggregation: sub,
                },
            )])
        });

        aggregations.extend(nested);
        Ok(aggregations)
    }
}

trait CollectFlat<Leaf, Marker> {
    fn iter_leaves(&self) -> Result<impl Iterator<Item = Leaf>>;

    fn collect(
        &self,
        mut aggregations: Aggregations,
        children: Aggregations,
    ) -> Result<Aggregations>
    where
        Leaf: Into<AggregationVariants>,
    {
        for (idx, leaf) in self.iter_leaves()?.enumerate() {
            aggregations.insert(
                idx.to_string(),
                Aggregation {
                    agg: leaf.into(),
                    sub_aggregation: children.clone(),
                },
            );
        }
        Ok(aggregations)
    }
}

pub trait CollectAggregations {
    fn collect(&self) -> Result<Aggregations>;
}

impl CollectAggregations for AggregateCSClause {
    fn collect(&self) -> Result<Aggregations> {
        // Validate that no contradicting solve_mvcc settings exist among custom aggregates
        self.mvcc_enabled();

        // Validate that all fields referenced in custom aggregates exist in the index schema
        if self.indexrelid != pg_sys::InvalidOid {
            let indexrel =
                PgSearchRelation::with_lock(self.indexrelid, pg_sys::AccessShareLock as _);
            if let Ok(schema) = SearchIndexSchema::open(&indexrel) {
                for agg in self.aggregates() {
                    if let Err(e) = agg.validate_fields(&schema) {
                        pgrx::error!("{}", e);
                    }
                }
            }
        }

        let agg = if !self.has_groupby() {
            let metrics =
                <Self as CollectFlat<AggregateType, MetricsWithoutGroupBy>>::iter_leaves(self)?;
            let filters =
                <Self as CollectFlat<Option<FilterQuery>, FiltersWithoutGroupBy>>::iter_leaves(
                    self,
                )?;

            let mut aggs = filters
                .zip(metrics)
                .enumerate()
                .map(|(idx, (filter, metric))| {
                    // For Custom aggregates, deserialize with nested aggregations
                    let metric_agg = if let AggregateType::Custom { agg_json, .. } = &metric {
                        // Tantivy's Aggregation deserializer handles nested "aggs" automatically
                        serde_json::from_value(agg_json.clone()).unwrap_or_else(|e| {
                            panic!("Failed to deserialize custom aggregate: {}", e)
                        })
                    } else {
                        Aggregation {
                            agg: metric.into(),
                            sub_aggregation: Default::default(),
                        }
                    };

                    let agg = match filter {
                        Some(filter) => Aggregation {
                            agg: filter.into(),
                            sub_aggregation: Aggregations::from_iter([(0.to_string(), metric_agg)]),
                        },
                        None => metric_agg,
                    };

                    (idx.to_string(), agg)
                })
                .collect::<Aggregations>();

            if gucs::add_doc_count_to_aggs() {
                aggs.insert(
                    DocCountKey::NAME.to_string(),
                    Aggregation {
                        agg: AggregationVariants::Count(CountAggregation {
                            field: "ctid".to_string(),
                            missing: None,
                        }),
                        sub_aggregation: Default::default(),
                    },
                );
            }

            aggs
        } else {
            let metrics = <Self as CollectFlat<AggregateType, MetricsWithGroupBy>>::collect(
                self,
                Default::default(),
                Default::default(),
            )?;
            let term_aggs =
                <Self as CollectNested<GroupedKey>>::collect(self, Default::default(), metrics)?;

            if self.has_filter() {
                let metrics =
                    <Self as CollectFlat<AggregateType, MetricsWithoutGroupBy>>::iter_leaves(self)?;
                let filters =
                    <Self as CollectFlat<FilterQuery, FiltersWithGroupBy>>::iter_leaves(self)?;

                let mut aggs = filters
                    .zip(metrics)
                    .enumerate()
                    .map(|(idx, (filter, metric))| {
                        let metric_agg = Aggregations::from_iter([(
                            0.to_string(),
                            Aggregation {
                                agg: metric.into(),
                                sub_aggregation: Default::default(),
                            },
                        )]);
                        let sub_agg = <Self as CollectNested<GroupedKey>>::collect(
                            self,
                            Default::default(),
                            metric_agg,
                        )
                        .expect("should be able to collect nested aggregations");
                        let filter_agg = Aggregation {
                            agg: filter.into(),
                            sub_aggregation: sub_agg,
                        };
                        (idx.to_string(), filter_agg)
                    })
                    .collect::<Aggregations>();

                aggs.insert(
                    FilterSentinelKey::NAME.to_string(),
                    Aggregation {
                        agg: new_filter_query(self.quals.query().clone(), self.indexrelid)?.into(),
                        sub_aggregation: term_aggs,
                    },
                );

                aggs
            } else {
                term_aggs
            }
        };

        Ok(agg)
    }
}

impl AggregateCSClause {
    pub fn aggregates(&self) -> impl Iterator<Item = &AggregateType> {
        self.targetlist.aggregates()
    }

    pub fn aggregates_mut(&mut self) -> impl Iterator<Item = &mut AggregateType> {
        self.targetlist.aggregates_mut()
    }

    pub fn can_use_doc_count(&self, agg: &AggregateType) -> bool {
        self.has_groupby() && !self.has_filter() && matches!(agg, AggregateType::CountAny { .. })
    }

    pub fn entries(&self) -> impl Iterator<Item = &TargetListEntry> {
        self.targetlist.entries()
    }

    pub fn groupby(&self) -> &GroupByClause {
        self.targetlist.groupby()
    }

    pub fn grouping_columns(&self) -> Vec<GroupingColumn> {
        self.targetlist.grouping_columns()
    }

    pub fn has_filter(&self) -> bool {
        self.targetlist
            .aggregates()
            .any(|agg| agg.filter_expr().is_some())
    }

    pub fn has_orderby(&self) -> bool {
        self.orderby.has_orderby()
    }

    pub fn has_groupby(&self) -> bool {
        !self.targetlist.grouping_columns().is_empty()
    }

    /// Determines if MVCC filtering should be enabled for this aggregate scan.
    /// Also validates that there are no contradicting solve_mvcc settings among custom aggregates.
    pub fn mvcc_enabled(&self) -> bool {
        AggregateType::resolve_mvcc_enabled(self.aggregates())
    }

    pub fn planner_should_replace_aggrefs(&self) -> bool {
        self.targetlist.grouping_columns().is_empty()
            && self.orderby.orderby_info().is_empty()
            && !self.orderby.has_orderby()
    }

    pub fn query(&self) -> &SearchQueryInput {
        self.quals.query()
    }

    pub fn query_mut(&mut self) -> &mut SearchQueryInput {
        self.quals.query_mut()
    }

    pub fn set_is_execution_time(&mut self) {
        self.is_execution_time = true;
    }

    /// Returns set of field names where the sentinel should use MIN values
    /// (i.e., NULLs should sort FIRST in the final output)
    ///
    /// We need a MIN sentinel (treating NULL as -Infinity) when:
    /// 1. Sort is ASC and we want NULLs FIRST (-Infinity < any value).
    /// 2. Sort is DESC and we want NULLs LAST (any value > -Infinity, so -Infinity comes last in DESC).
    ///
    /// Conversely, we use a MAX sentinel (treating NULL as +Infinity) when:
    /// 1. Sort is ASC and we want NULLs LAST (any value < +Infinity).
    /// 2. Sort is DESC and we want NULLs FIRST (+Infinity > any value, so +Infinity comes first in DESC).
    pub fn use_min_sentinel_fields(&self) -> HashSet<String> {
        use crate::api::{OrderByFeature, SortDirection};
        self.orderby
            .orderby_info()
            .iter()
            .filter(|info| match info.direction {
                SortDirection::AscNullsFirst | SortDirection::DescNullsLast => true,
                SortDirection::AscNullsLast | SortDirection::DescNullsFirst => false,
            })
            .filter_map(|info| match &info.feature {
                OrderByFeature::Field { name, .. } => Some(name.to_string()),
                OrderByFeature::Score { .. } | OrderByFeature::Var { .. } => None,
            })
            .collect()
    }
}

impl CustomScanClause<AggregateScan> for AggregateCSClause {
    type Args = <AggregateScan as CustomScan>::Args;

    fn add_to_custom_path(
        &self,
        builder: CustomPathBuilder<AggregateScan>,
    ) -> CustomPathBuilder<AggregateScan> {
        let mut builder = self.targetlist.add_to_custom_path(builder);
        builder = self.orderby.add_to_custom_path(builder);
        builder = self.limit_offset.add_to_custom_path(builder);
        builder = self.quals.add_to_custom_path(builder);
        builder
    }

    fn explain_output(&self) -> Box<dyn Iterator<Item = (String, String)>> {
        let aggregate =
            CollectAggregations::collect(self).expect("should be able to collect aggregations");

        let aggregate_types = std::iter::once((
            String::from("Applies to Aggregates"),
            self.targetlist
                .aggregates()
                .map(|agg| agg.clone().to_string())
                .collect::<Vec<_>>()
                .join(", "),
        ));

        let aggregate_json = {
            let mut aggregate_json =
                serde_json::to_value(&aggregate).expect("should be able to serialize aggregations");
            cleanup_json_for_explain(&mut aggregate_json);
            sort_json_keys(&mut aggregate_json);
            std::iter::once((
                String::from("Aggregate Definition"),
                serde_json::to_string(&aggregate_json)
                    .expect("should be able to serialize aggregations"),
            ))
        };

        let topk_output: Vec<(String, String)> = if let (true, Some(ref agg_order)) =
            (self.limit_offset.limit().is_some(), &self.aggregate_orderby)
        {
            let dir = match agg_order.direction {
                SortDirection::DescNullsFirst | SortDirection::DescNullsLast => "DESC",
                _ => "ASC",
            };
            let target_name = match &agg_order.target {
                AggregateMetricTarget::Count => "COUNT(*)".to_string(),
                AggregateMetricTarget::Metric(key) => {
                    let mut idx = 0usize;
                    let mut name = format!("metric_{}", key);
                    for agg in self.targetlist.aggregates() {
                        if agg.can_use_doc_count() {
                            continue;
                        }
                        if idx == *key {
                            name = agg.to_string();
                            break;
                        }
                        idx += 1;
                    }
                    name
                }
            };
            vec![(
                String::from("TopK Order"),
                format!("{} {}", target_name, dir),
            )]
        } else {
            vec![]
        };

        Box::new(
            aggregate_types
                .chain(self.groupby().explain_output())
                .chain(self.limit_offset.explain_output())
                .chain(topk_output)
                .chain(aggregate_json),
        )
    }

    fn from_pg(
        args: &Self::Args,
        heap_rti: pg_sys::Index,
        index: &PgSearchRelation,
    ) -> Result<Self, CustomScanBuildError> {
        let targetlist = TargetList::from_pg(args, heap_rti, index)?;
        // OrderBy is optional - if we can't extract it but there IS a sort clause,
        // use unpushable() to remember that ordering exists
        let orderby = match OrderByClause::from_pg(args, heap_rti, index) {
            Ok(o) => o,
            Err(_) => {
                let has_sort_clause = unsafe {
                    !args.root().parse.is_null() && !(*args.root().parse).sortClause.is_null()
                };
                if has_sort_clause {
                    OrderByClause::unpushable()
                } else {
                    OrderByClause::default()
                }
            }
        };
        // LimitOffset is optional
        let limit_offset = unsafe { LimitOffset::from_parse(args.root().parse) };
        let quals = SearchQueryClause::from_pg(args, heap_rti, index)?;

        if !gucs::enable_custom_scan_without_operator()
            && !quals.uses_our_operator()
            && !targetlist.uses_our_operator()
        {
            return Err(CustomScanBuildError::NotInteresting);
        }

        // Detect ORDER BY on aggregate for TopK optimization
        let aggregate_orderby = if orderby.has_orderby() && orderby.orderby_info().is_empty() {
            unsafe { detect_aggregate_orderby(args, &targetlist) }
        } else {
            None
        };

        Ok(Self {
            targetlist,
            orderby,
            limit_offset,
            quals,
            indexrelid: index.oid(),
            is_execution_time: false,
            aggregate_orderby,
        })
    }
}

/// Determines sort direction from a Postgres sort operator OID.
///
/// Returns `None` if the operator properties cannot be resolved (should not
/// happen for valid `SortGroupClause` operators). Callers should bail out
/// of the TopK optimization rather than guessing a direction.
#[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
pub(super) unsafe fn sort_direction_from_op(sortop: pg_sys::Oid) -> Option<SortDirection> {
    let mut opfamily = pg_sys::InvalidOid;
    let mut opcintype = pg_sys::InvalidOid;
    let mut strategy: i16 = 0;
    if pg_sys::get_ordering_op_properties(sortop, &mut opfamily, &mut opcintype, &mut strategy) {
        if strategy as u32 == pg_sys::BTGreaterStrategyNumber {
            Some(SortDirection::DescNullsFirst)
        } else {
            Some(SortDirection::AscNullsLast)
        }
    } else {
        None
    }
}

#[cfg(feature = "pg18")]
pub(super) unsafe fn sort_direction_from_op(sortop: pg_sys::Oid) -> Option<SortDirection> {
    let mut opfamily = pg_sys::InvalidOid;
    let mut opcintype = pg_sys::InvalidOid;
    let mut cmptype = pg_sys::CompareType::COMPARE_LT;
    if pg_sys::get_ordering_op_properties(sortop, &mut opfamily, &mut opcintype, &mut cmptype) {
        if cmptype == pg_sys::CompareType::COMPARE_GT {
            Some(SortDirection::DescNullsFirst)
        } else {
            Some(SortDirection::AscNullsLast)
        }
    } else {
        None
    }
}

/// Detects ORDER BY on aggregate metrics (e.g., ORDER BY COUNT(*) DESC)
/// for TopK optimization in TermsAggregation.
///
/// Returns `Some(AggregateOrderBy)` when the sort clause targets a single aggregate
/// that can be pushed down to Tantivy's TermsAggregation ordering.
unsafe fn detect_aggregate_orderby(
    args: &CreateUpperPathsHookArgs,
    targetlist: &TargetList,
) -> Option<AggregateOrderBy> {
    let parse = args.root().parse;
    if parse.is_null() || (*parse).sortClause.is_null() || (*parse).groupClause.is_null() {
        return None;
    }

    // Only support single sort clause for TopK
    let sort_clauses = PgList::<pg_sys::SortGroupClause>::from_pg((*parse).sortClause);
    if sort_clauses.len() != 1 {
        return None;
    }

    // Don't support TopK with aggregate filters (different tree structure)
    if targetlist.aggregates().any(|agg| agg.has_filter()) {
        return None;
    }

    let sort_clause_ptr = sort_clauses.get_ptr(0)?;
    let sort_expr = pg_sys::get_sortgroupclause_expr(sort_clause_ptr, (*parse).targetList);

    // The sort expression must BE an aggregate — not merely contain one.
    // e.g. ORDER BY COUNT(*) DESC is safe, but ORDER BY ABS(SUM(score)) DESC
    // is not: ABS() breaks monotonicity, so Tantivy's ordering wouldn't match.
    let aggref = find_single_aggref_in_expr(sort_expr)?;
    if aggref as *mut pg_sys::Node != sort_expr {
        return None;
    }

    // Determine sort direction; bail out if the operator is unrecognized
    // so we fall back to the un-optimized path rather than risk wrong results.
    let direction = sort_direction_from_op((*sort_clause_ptr).sortop)?;

    // Find matching position in output_rel target using structural equality
    let reltarget = args.output_rel().reltarget;
    if reltarget.is_null() {
        return None;
    }
    let target_exprs = PgList::<pg_sys::Expr>::from_pg((*reltarget).exprs);

    let mut match_pos = None;
    for (pos, target_expr) in target_exprs.iter_ptr().enumerate() {
        if pg_sys::equal(
            sort_expr as *const core::ffi::c_void,
            target_expr as *const core::ffi::c_void,
        ) {
            match_pos = Some(pos);
            break;
        }
    }

    let pos = match_pos?;

    // Check if this position is an Aggregate in our TargetList
    let entry = targetlist.entries().nth(pos)?;
    let agg = match entry {
        TargetListEntry::Aggregate(agg) => agg,
        _ => return None,
    };

    // COUNT(*) without filter uses doc_count
    if agg.can_use_doc_count() {
        return Some(AggregateOrderBy {
            target: AggregateMetricTarget::Count,
            direction,
        });
    }

    // Compute position among non-doc-count metrics (matches CollectFlat<MetricsWithGroupBy> keying)
    let mut metric_idx = 0usize;
    for (i, entry) in targetlist.entries().enumerate() {
        if i == pos {
            return Some(AggregateOrderBy {
                target: AggregateMetricTarget::Metric(metric_idx),
                direction,
            });
        }
        if let TargetListEntry::Aggregate(a) = entry {
            if !a.can_use_doc_count() {
                metric_idx += 1;
            }
        }
    }

    None
}

impl CollectNested<GroupedKey> for AggregateCSClause {
    fn iter_leaves(&self) -> Result<impl Iterator<Item = AggregationVariants>> {
        let orderby_info = self.orderby.orderby_info();
        let grouping_columns = self.targetlist.grouping_columns();

        let max_buckets = gucs::max_term_agg_buckets() as u32;

        // `size` controls how many buckets the final merge returns.
        // `segment_size` controls per-segment collection. We always keep
        // segment_size = max_buckets so every segment contributes accurate
        // counts — setting segment_size = K causes per-segment approximation
        // errors where groups distributed across segments get undercounted.
        //
        // With segment_size = max_buckets, correctness is the same as the
        // non-TopK path: exact counts as long as distinct groups ≤ max_buckets.
        // The only optimization is `size = K` limiting the merged output.
        // Postgres still adds Sort + Limit above us for final ordering.
        let size = {
            let limit = self.limit_offset.limit();
            let offset = self.limit_offset.offset();

            // We can use LIMIT-based size optimization when:
            // 1. There's exactly one grouping column (multiple columns need all combinations)
            // 2. Either no ORDER BY, or ORDER BY targets a COUNT aggregate
            //
            // We intentionally exclude Metric targets (SUM, AVG, MIN, MAX) from
            // size=K: these can produce NULL when all values in a group are NULL.
            // Tantivy may prune NULL groups from the top-K, but Postgres's NULLS
            // FIRST (default for DESC) would rank them first. Keeping size=max_buckets
            // ensures NULL groups are always returned to Postgres for correct ordering.
            let can_limit_buckets = grouping_columns.len() == 1
                && (!self.orderby.has_orderby()
                    || matches!(
                        self.aggregate_orderby,
                        Some(AggregateOrderBy {
                            target: AggregateMetricTarget::Count,
                            ..
                        })
                    ));

            if can_limit_buckets {
                if let Some(limit) = limit {
                    (limit + offset.unwrap_or(0)).min(max_buckets)
                } else {
                    max_buckets
                }
            } else {
                max_buckets
            }
        };

        // Only apply aggregate ordering for single grouping column
        let aggregate_orderby = if grouping_columns.len() == 1 {
            self.aggregate_orderby.clone()
        } else {
            None
        };

        Ok(grouping_columns.into_iter().map(move |column| {
            let orderby = orderby_info.iter().find(|info| {
                if let OrderByFeature::Field {
                    name: field_name, ..
                } = &info.feature
                {
                    field_name == &FieldName::from(column.field_name.clone())
                } else {
                    false
                }
            });

            let mut terms_agg = TermsAggregation {
                field: column.field_name.clone(),
                size: Some(size),
                // Always collect all buckets per segment for accurate counts.
                // Only the final merge output is limited to `size`.
                segment_size: Some(max_buckets),
                ..Default::default()
            };

            if let Some(ref agg_order) = aggregate_orderby {
                // ORDER BY on aggregate metric — use Count or SubAggregation target.
                //
                // NULL handling: when all values in a group are NULL, aggregates
                // like SUM/AVG/MIN/MAX produce NULL. Tantivy's TermsAggregation
                // sorts NULLs according to its own rules (not Postgres's NULLS
                // FIRST/LAST), but since Postgres adds a Sort node above us the
                // final NULL ordering is determined by Postgres, not Tantivy.
                let target = match &agg_order.target {
                    AggregateMetricTarget::Count => OrderTarget::Count,
                    AggregateMetricTarget::Metric(idx) => {
                        OrderTarget::SubAggregation(idx.to_string())
                    }
                };
                terms_agg.order = Some(CustomOrder {
                    target,
                    order: agg_order.direction.into(),
                });
            } else if let Some(orderby) = orderby {
                terms_agg.order = Some(CustomOrder {
                    target: OrderTarget::Key,
                    order: orderby.direction.into(),
                });
            }

            AggregationVariants::Terms(terms_agg)
        }))
    }
}

pub struct MetricsWithGroupBy;
pub struct MetricsWithoutGroupBy;
pub struct FiltersWithGroupBy;
pub struct FiltersWithoutGroupBy;

impl CollectFlat<AggregateType, MetricsWithoutGroupBy> for AggregateCSClause {
    fn iter_leaves(&self) -> Result<impl Iterator<Item = AggregateType>> {
        Ok(self.targetlist.aggregates().cloned())
    }
}

impl CollectFlat<AggregateType, MetricsWithGroupBy> for AggregateCSClause {
    fn iter_leaves(&self) -> Result<impl Iterator<Item = AggregateType>> {
        Ok(self.targetlist.aggregates().filter_map(|agg| {
            if !agg.can_use_doc_count() {
                Some(agg.clone())
            } else {
                None
            }
        }))
    }
}

impl CollectFlat<Option<FilterQuery>, FiltersWithoutGroupBy> for AggregateCSClause {
    fn iter_leaves(&self) -> Result<impl Iterator<Item = Option<FilterQuery>>> {
        Ok(self.targetlist.aggregates().map(|agg| {
            agg.filter_expr()
                .as_ref()
                .map(|q| new_filter_query(q.clone(), agg.indexrelid()).expect("filter query"))
        }))
    }
}

impl CollectFlat<FilterQuery, FiltersWithGroupBy> for AggregateCSClause {
    fn iter_leaves(&self) -> Result<impl Iterator<Item = FilterQuery>> {
        Ok(self.targetlist.aggregates().map(|agg| {
            let query = match agg.filter_expr() {
                Some(q) => q.clone(),
                None => SearchQueryInput::All,
            };
            new_filter_query(query, agg.indexrelid()).expect("filter query")
        }))
    }
}

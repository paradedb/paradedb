// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::api::{FieldName, HashSet, MvccVisibility, OrderByFeature};
use crate::gucs;
use crate::postgres::customscan::aggregatescan::aggregate_type::AggregateType;
use crate::postgres::customscan::aggregatescan::filterquery::FilterQuery;
use crate::postgres::customscan::aggregatescan::limit_offset::LimitOffsetClause;
use crate::postgres::customscan::aggregatescan::orderby::OrderByClause;
use crate::postgres::customscan::aggregatescan::searchquery::SearchQueryClause;
use crate::postgres::customscan::aggregatescan::targetlist::{TargetList, TargetListEntry};
use crate::postgres::customscan::aggregatescan::{AggregateScan, CustomScanClause};
use crate::postgres::customscan::aggregatescan::{GroupByClause, GroupingColumn};
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::CustomScan;
use crate::postgres::utils::sort_json_keys;
use crate::postgres::PgSearchRelation;
use crate::query::SearchQueryInput;

use anyhow::Result;
use pgrx::pg_sys;
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

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AggregateCSClause {
    targetlist: TargetList,
    orderby: OrderByClause,
    limit_offset: LimitOffsetClause,
    quals: SearchQueryClause,
    indexrelid: pg_sys::Oid,
    is_execution_time: bool,
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
            Aggregations::from([(
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
        // Validate that no custom aggregate has solve_mvcc=false in GROUP BY context.
        // solve_mvcc=false is only allowed in TopN (window function) context.
        for agg in self.aggregates() {
            if let AggregateType::Custom {
                mvcc_visibility, ..
            } = agg
            {
                if *mvcc_visibility == MvccVisibility::Disabled {
                    pgrx::error!(
                        "pdb.agg() with solve_mvcc=false is only supported in window function context \
                         (with OVER clause). GROUP BY aggregates always use MVCC filtering for correctness. \
                         Remove the second argument or use solve_mvcc=true."
                    );
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
                            sub_aggregation: Aggregations::new(),
                        }
                    };

                    let agg = match filter {
                        Some(filter) => Aggregation {
                            agg: filter.into(),
                            sub_aggregation: Aggregations::from([(0.to_string(), metric_agg)]),
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
                        sub_aggregation: Aggregations::new(),
                    },
                );
            }

            aggs
        } else {
            let metrics = <Self as CollectFlat<AggregateType, MetricsWithGroupBy>>::collect(
                self,
                Aggregations::new(),
                Aggregations::new(),
            )?;
            let term_aggs =
                <Self as CollectNested<GroupedKey>>::collect(self, Aggregations::new(), metrics)?;

            if self.has_filter() {
                let metrics =
                    <Self as CollectFlat<AggregateType, MetricsWithoutGroupBy>>::iter_leaves(self)?;
                let filters =
                    <Self as CollectFlat<FilterQuery, FiltersWithGroupBy>>::iter_leaves(self)?;

                let mut aggs = filters
                    .zip(metrics)
                    .enumerate()
                    .map(|(idx, (filter, metric))| {
                        let metric_agg = Aggregations::from([(
                            0.to_string(),
                            Aggregation {
                                agg: metric.into(),
                                sub_aggregation: Aggregations::new(),
                            },
                        )]);
                        let sub_agg = <Self as CollectNested<GroupedKey>>::collect(
                            self,
                            Aggregations::new(),
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
                        agg: FilterQuery::new(
                            self.quals.query().clone(),
                            self.indexrelid,
                            self.is_execution_time,
                        )?
                        .into(),
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
    /// The logic accounts for both the NULLS FIRST/LAST setting and the sort direction:
    /// - For ASC: MIN sentinel → appears first, MAX sentinel → appears last
    /// - For DESC: MAX sentinel → appears first (reversed), MIN sentinel → appears last (reversed)
    ///
    /// So we need MIN sentinel when: (nulls_first && ASC) || (!nulls_first && DESC)
    /// Which simplifies to: nulls_first == (direction == ASC)
    pub fn use_min_sentinel_fields(&self) -> HashSet<String> {
        use crate::api::{OrderByFeature, SortDirection};
        self.orderby
            .orderby_info()
            .iter()
            .filter(|info| {
                let is_asc = matches!(info.direction, SortDirection::Asc);
                // Use MIN sentinel when nulls should appear first in final output
                // accounting for direction reversal
                info.nulls_first == is_asc
            })
            .filter_map(|info| match &info.feature {
                OrderByFeature::Field(name) => Some(name.to_string()),
                OrderByFeature::Score => None,
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
            sort_json_keys(&mut aggregate_json);
            std::iter::once((
                String::from("Aggregate Definition"),
                serde_json::to_string(&aggregate_json)
                    .expect("should be able to serialize aggregations"),
            ))
        };

        Box::new(
            aggregate_types
                .chain(self.groupby().explain_output())
                .chain(self.limit_offset.explain_output())
                .chain(aggregate_json),
        )
    }

    fn from_pg(
        args: &Self::Args,
        heap_rti: pg_sys::Index,
        index: &PgSearchRelation,
    ) -> Option<Self> {
        let targetlist = TargetList::from_pg(args, heap_rti, index)?;
        let orderby = OrderByClause::from_pg(args, heap_rti, index)?;
        let limit_offset = LimitOffsetClause::from_pg(args, heap_rti, index)?;
        let quals = SearchQueryClause::from_pg(args, heap_rti, index)?;

        if !gucs::enable_custom_scan_without_operator()
            && !quals.uses_our_operator()
            && !targetlist.uses_our_operator()
        {
            return None;
        }

        Some(Self {
            targetlist,
            orderby,
            limit_offset,
            quals,
            indexrelid: index.oid(),
            is_execution_time: false,
        })
    }
}

impl CollectNested<GroupedKey> for AggregateCSClause {
    fn iter_leaves(&self) -> Result<impl Iterator<Item = AggregationVariants>> {
        let orderby_info = self.orderby.orderby_info();

        let size = {
            let limit = self.limit_offset.limit();
            let offset = self.limit_offset.offset();
            if let Some(limit) = limit {
                (limit + offset.unwrap_or(0)).min(gucs::max_term_agg_buckets() as u32)
            } else {
                gucs::max_term_agg_buckets() as u32
            }
        };

        let grouping_columns = self.targetlist.grouping_columns();

        Ok(grouping_columns.into_iter().map(move |column| {
            let orderby = orderby_info.iter().find(|info| {
                if let OrderByFeature::Field(field_name) = &info.feature {
                    field_name == &FieldName::from(column.field_name.clone())
                } else {
                    false
                }
            });

            let mut terms_agg = TermsAggregation {
                field: column.field_name.clone(),
                size: Some(size),
                segment_size: Some(size),
                ..Default::default()
            };

            if let Some(orderby) = orderby {
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
            agg.filter_expr().as_ref().map(|filter_expr| {
                FilterQuery::new(
                    filter_expr.clone(),
                    agg.indexrelid(),
                    self.is_execution_time,
                )
                .expect("should be able to create filter query")
            })
        }))
    }
}

impl CollectFlat<FilterQuery, FiltersWithGroupBy> for AggregateCSClause {
    fn iter_leaves(&self) -> Result<impl Iterator<Item = FilterQuery>> {
        Ok(self.targetlist.aggregates().map(|agg| {
            if let Some(filter_expr) = agg.filter_expr() {
                FilterQuery::new(
                    filter_expr.clone(),
                    agg.indexrelid(),
                    self.is_execution_time,
                )
                .expect("should be able to create filter query")
            } else {
                FilterQuery::new(
                    SearchQueryInput::All,
                    agg.indexrelid(),
                    self.is_execution_time,
                )
                .expect("should be able to create filter query")
            }
        }))
    }
}

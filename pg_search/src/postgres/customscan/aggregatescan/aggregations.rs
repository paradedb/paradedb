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

use crate::api::{FieldName, OrderByFeature};
use crate::gucs;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::aggregatescan::limit_offset::LimitOffsetClause;
use crate::postgres::customscan::aggregatescan::orderby::OrderByClause;
use crate::postgres::customscan::aggregatescan::query::SearchQueryClause;
use crate::postgres::customscan::aggregatescan::targetlist::{TargetList, TargetListEntry};
use crate::postgres::customscan::aggregatescan::{AggregateScan, CustomScanClause};
use crate::postgres::customscan::aggregatescan::{AggregateType, GroupByClause, GroupingColumn};
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::explain::ExplainFormat;
use crate::postgres::customscan::CustomScan;
use crate::postgres::utils::{sort_json_keys, ExprContextGuard};
use crate::postgres::PgSearchRelation;
use crate::query::SearchQueryInput;

use anyhow::{bail, Result};
use pgrx::pg_sys;
use std::collections::BTreeMap;
use std::ptr::NonNull;
use std::sync::{Arc, OnceLock};
use tantivy::aggregation::agg_req::Aggregations;
use tantivy::aggregation::agg_req::{Aggregation, AggregationVariants};
use tantivy::aggregation::bucket::{
    CustomOrder, FilterAggregation, OrderTarget, SerializableQuery, TermsAggregation,
};
use tantivy::aggregation::metric::{
    AverageAggregation, CountAggregation, MaxAggregation, MinAggregation, SumAggregation,
};
use tantivy::query::{AllQuery, EnableScoring, Query, QueryParser, Weight};

pub(crate) trait AggregationKey {
    const NAME: &'static str;
}

pub(crate) struct GroupedKey;
impl AggregationKey for GroupedKey {
    const NAME: &'static str = "grouped";
}

pub(crate) struct FilterSentinelKey;
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
}

trait CollectNested<Key: AggregationKey> {
    fn into_iter(&self) -> Result<impl Iterator<Item = AggregationVariants>>;

    fn collect(
        &self,
        mut aggregations: Aggregations,
        children: Aggregations,
    ) -> Result<Aggregations> {
        let groupings: Vec<_> = self.into_iter()?.collect();

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
    fn into_iter(&self) -> Result<impl Iterator<Item = Leaf>>;

    fn collect(
        &self,
        mut aggregations: Aggregations,
        children: Aggregations,
    ) -> Result<Aggregations>
    where
        Leaf: Into<AggregationVariants>,
    {
        for (idx, leaf) in self.into_iter()?.enumerate() {
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
        let agg = if !self.has_groupby() {
            let metrics =
                <Self as CollectFlat<AggregateType, MetricsWithoutGroupBy>>::into_iter(self)?;
            let filters =
                <Self as CollectFlat<Option<FilterQuery>, FiltersWithoutGroupBy>>::into_iter(self)?;

            filters
                .zip(metrics)
                .enumerate()
                .map(|(idx, (filter, metric))| {
                    let metric_agg = Aggregation {
                        agg: metric.into(),
                        sub_aggregation: Aggregations::new(),
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
                .collect::<Aggregations>()
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
                    <Self as CollectFlat<AggregateType, MetricsWithoutGroupBy>>::into_iter(self)?;
                let filters =
                    <Self as CollectFlat<FilterQuery, FiltersWithGroupBy>>::into_iter(self)?;

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
                        agg: FilterQuery::new(self.quals.query().clone(), self.indexrelid)?.into(),
                        sub_aggregation: term_aggs,
                    },
                );

                aggs
            } else {
                term_aggs
            }
        };

        // pgrx::info!("request: {:?} \n", agg);

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

        Some(Self {
            targetlist,
            orderby,
            limit_offset,
            quals,
            indexrelid: index.oid(),
        })
    }
}

impl CollectNested<GroupedKey> for AggregateCSClause {
    fn into_iter(&self) -> Result<impl Iterator<Item = AggregationVariants>> {
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

            let mut terms_agg = TermsAggregation::default();
            terms_agg.field = column.field_name.clone();
            terms_agg.size = Some(size);
            terms_agg.segment_size = Some(size);

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
    fn into_iter(&self) -> Result<impl Iterator<Item = AggregateType>> {
        Ok(self.targetlist.aggregates().map(|agg| agg.clone().into()))
    }
}

impl CollectFlat<AggregateType, MetricsWithGroupBy> for AggregateCSClause {
    fn into_iter(&self) -> Result<impl Iterator<Item = AggregateType>> {
        Ok(self.targetlist.aggregates().filter_map(|agg| {
            if !agg.can_use_doc_count() {
                Some(agg.clone().into())
            } else {
                None
            }
        }))
    }
}

impl CollectFlat<Option<FilterQuery>, FiltersWithoutGroupBy> for AggregateCSClause {
    fn into_iter(&self) -> Result<impl Iterator<Item = Option<FilterQuery>>> {
        Ok(self.targetlist.aggregates().map(|agg| {
            if let Some(filter_expr) = agg.filter_expr() {
                Some(
                    FilterQuery::new(filter_expr.clone(), agg.indexrelid())
                        .expect("should be able to create filter query"),
                )
            } else {
                None
            }
        }))
    }
}

impl CollectFlat<FilterQuery, FiltersWithGroupBy> for AggregateCSClause {
    fn into_iter(&self) -> Result<impl Iterator<Item = FilterQuery>> {
        Ok(self.targetlist.aggregates().map(|agg| {
            if let Some(filter_expr) = agg.filter_expr() {
                FilterQuery::new(filter_expr.clone(), agg.indexrelid())
                    .expect("should be able to create filter query")
            } else {
                FilterQuery::new(SearchQueryInput::All, agg.indexrelid())
                    .expect("should be able to create filter query")
            }
        }))
    }
}

impl From<AggregateType> for AggregationVariants {
    fn from(val: AggregateType) -> Self {
        match val {
            AggregateType::CountAny { .. } => AggregationVariants::Count(CountAggregation {
                field: "ctid".to_string(),
                missing: None,
            }),
            AggregateType::Count { field, missing, .. } => {
                AggregationVariants::Count(CountAggregation { field, missing })
            }
            AggregateType::Sum { field, missing, .. } => {
                AggregationVariants::Sum(SumAggregation { field, missing })
            }
            AggregateType::Avg { field, missing, .. } => {
                AggregationVariants::Average(AverageAggregation { field, missing })
            }
            AggregateType::Min { field, missing, .. } => {
                AggregationVariants::Min(MinAggregation { field, missing })
            }
            AggregateType::Max { field, missing, .. } => {
                AggregationVariants::Max(MaxAggregation { field, missing })
            }
        }
    }
}

#[derive(Debug)]
struct FilterQuery {
    query: SearchQueryInput,
    indexrelid: pg_sys::Oid,
    tantivy_query: Box<dyn Query>,
}

impl From<FilterQuery> for AggregationVariants {
    fn from(val: FilterQuery) -> Self {
        AggregationVariants::Filter(FilterAggregation::new_with_query(Box::new(val)))
    }
}

impl Clone for FilterQuery {
    fn clone(&self) -> Self {
        Self {
            query: self.query.clone(),
            indexrelid: self.indexrelid,
            tantivy_query: self.tantivy_query.box_clone(),
        }
    }
}

impl Query for FilterQuery {
    fn weight(&self, enable_scoring: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
        self.tantivy_query.weight(enable_scoring)
    }
}

impl SerializableQuery for FilterQuery {
    fn clone_box(&self) -> Box<dyn SerializableQuery> {
        Box::new(self.clone())
    }
}

impl serde::Serialize for FilterQuery {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let raw = self.query.explain_format();
        serde_json::from_str::<serde_json::Value>(&raw)
            .expect("should be able to serialize searchqueryinput")
            .serialize(serializer)
    }
}

impl FilterQuery {
    pub fn new(query: SearchQueryInput, indexrelid: pg_sys::Oid) -> Result<Self> {
        let standalone_context = ExprContextGuard::new();
        let index = PgSearchRelation::with_lock(indexrelid, pg_sys::AccessShareLock as _);
        let schema = index.schema()?;
        let reader = SearchIndexReader::open_with_context(
            &index,
            query.clone(),
            false,
            MvccSatisfies::Snapshot,
            NonNull::new(standalone_context.as_ptr()),
            None,
        )?;
        let parser = || {
            QueryParser::for_index(
                reader.searcher().index(),
                schema.fields().map(|(f, _)| f).collect(),
            )
        };
        let heap_oid = index.heap_relation().map(|r| r.oid());
        // pgrx::info!("returning tantivy query");
        let tantivy_query = Box::new(query.clone().into_tantivy_query(
            &schema,
            &parser,
            reader.searcher(),
            index.oid(),
            heap_oid,
            NonNull::new(standalone_context.as_ptr()),
            None,
        )?);

        Ok(Self {
            query,
            indexrelid,
            tantivy_query,
        })
    }
}

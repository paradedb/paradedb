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
use crate::customscan::aggregatescan::AggregateType;
use crate::customscan::aggregatescan::GroupingColumn;
use crate::gucs;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::aggregatescan::limit_offset::LimitOffsetClause;
use crate::postgres::customscan::aggregatescan::orderby::OrderByClause;
use crate::postgres::customscan::aggregatescan::quals::SearchQueryClause;
use crate::postgres::customscan::aggregatescan::targetlist::{TargetList, TargetListEntry};
use crate::postgres::customscan::aggregatescan::{AggregateScan, CustomScanClause};
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::CustomScan;
use crate::postgres::utils::ExprContextGuard;
use crate::postgres::PgSearchRelation;
use crate::query::SearchQueryInput;

use anyhow::{bail, Result};
use pgrx::pg_sys;
use std::ptr::NonNull;
use std::sync::OnceLock;
use tantivy::aggregation::agg_req::Aggregations;
use tantivy::aggregation::agg_req::{Aggregation, AggregationVariants};
use tantivy::aggregation::bucket::{CustomOrder, FilterAggregation, OrderTarget, TermsAggregation};
use tantivy::aggregation::metric::{
    AverageAggregation, CountAggregation, MaxAggregation, MinAggregation, SumAggregation,
};
use tantivy::query::{EnableScoring, Query, QueryParser, Weight};

trait AggregationKey {
    const NAME: &'static str;
}

struct GroupedKey;
impl AggregationKey for GroupedKey {
    const NAME: &'static str = "grouped";
}

struct FilterKey;
impl AggregationKey for FilterKey {
    const NAME: &'static str = "filter";
}

struct FilterAggUngroupedKey;
impl AggregationKey for FilterAggUngroupedKey {
    const NAME: &'static str = "filter_agg";
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AggregateCSClause {
    targetlist: TargetList,
    orderby: OrderByClause,
    limit_offset: LimitOffsetClause,
    quals: SearchQueryClause,
    indexrelid: pg_sys::Oid,
}

struct FilterAggregationGroupedQual(Aggregations);
struct FilterAggregationUngroupedQual(Aggregations);

trait CollectNested<Key: AggregationKey> {
    fn into_iter(&self) -> Result<impl Iterator<Item = AggregationVariants>>;

    fn collect(&self, mut aggregations: Aggregations) -> Result<Aggregations> {
        for leaf in self.into_iter()? {
            let aggregation = Aggregation {
                agg: leaf,
                sub_aggregation: aggregations.clone(),
            };
            aggregations.insert(Key::NAME.to_string(), aggregation);
        }
        Ok(aggregations)
    }
}
trait CollectFlat<Leaf, Marker>
where
    Leaf: Into<Aggregation>,
{
    fn into_iter(&self) -> Result<impl Iterator<Item = Leaf>>;

    fn collect(&self, mut aggregations: Aggregations) -> Result<Aggregations> {
        for (idx, leaf) in self.into_iter()?.enumerate() {
            aggregations.insert(idx.to_string(), leaf.into());
        }
        Ok(aggregations)
    }
}

pub trait CollectAggregations {
    fn collect(&self) -> Result<Aggregations>;
}

impl CollectAggregations for AggregateCSClause {
    fn collect(&self) -> Result<Aggregations> {
        if !self.has_groupby() {
            Ok(<Self as CollectFlat<AggregateType, NoGroupBy>>::collect(
                self,
                Aggregations::new(),
            )?)
        } else {
            let metrics = <Self as CollectFlat<AggregateType, HasGroupBy>>::collect(
                self,
                Aggregations::new(),
            )?;
            Ok(<Self as CollectNested<GroupedKey>>::collect(self, metrics)?)
        }

        // <Self as CollectFlat<MetricAggregations>>::collect(self, &mut aggs)?;

        // if self.has_groupby() {
        //     let metrics = std::mem::take(&mut aggs);
        //     <Self as CollectNested<TermsAggregation, GroupedKey>>::collect(self, &mut aggs)?;

        //     if let Some(Aggregation { agg, .. }) = aggs.remove(GroupedKey::NAME) {
        //         aggs.insert(
        //             GroupedKey::NAME.to_string(),
        //             Aggregation {
        //                 agg,
        //                 sub_aggregation: metrics,
        //             },
        //         );
        //     }
        // }
        // let has_terms_aggregations = !terms_aggregations.is_empty();

        // let metric_aggregations = <Self as IterFlat<MetricAggregations>>::into_iter(self)?;
        // for (idx, metric_agg) in metric_aggregations.enumerate() {
        //     aggregations.insert(idx.to_string(), metric_agg.into());
        // }

        // for (idx, term_agg) in terms_aggregations.into_iter().enumerate() {
        //     aggregations.insert(idx.to_string(), term_agg.into());
        // }
        // if has_terms_aggregations {
        //     let sub_aggregations =
        //         <Self as IterFlat<FilterAggregationGroupedQual>>::into_iter(self)?;
        //     add_filter_aggregations(&mut aggregations, filter_aggregations, sub_aggregations);
        // } else {
        //     let sub_aggregations =
        //         <Self as IterFlat<FilterAggregationUngroupedQual>>::into_iter(self)?;
        //     add_filter_aggregations(&mut aggregations, filter_aggregations, sub_aggregations);
        // }
    }
}

impl AggregateCSClause {
    pub fn aggregates(&self) -> impl Iterator<Item = &AggregateType> {
        self.targetlist.aggregates()
    }

    pub fn aggregates_mut(&mut self) -> impl Iterator<Item = &mut AggregateType> {
        self.targetlist.aggregates_mut()
    }

    pub fn entries(&self) -> impl Iterator<Item = &TargetListEntry> {
        self.targetlist.entries()
    }

    pub fn grouping_columns(&self) -> Vec<GroupingColumn> {
        self.targetlist.grouping_columns()
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

pub struct HasGroupBy;
pub struct NoGroupBy;

impl CollectFlat<AggregateType, NoGroupBy> for AggregateCSClause {
    fn into_iter(&self) -> Result<impl Iterator<Item = AggregateType>> {
        Ok(self.targetlist.aggregates().map(|agg| agg.clone().into()))
    }
}

impl CollectFlat<AggregateType, HasGroupBy> for AggregateCSClause {
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

impl From<FilterAggregationUngroupedQual> for Aggregations {
    fn from(val: FilterAggregationUngroupedQual) -> Self {
        val.0
    }
}

// impl IterFlat<FilterAggregationUngroupedQual> for AggregateCSClause {
//     fn into_iter(&self) -> Result<impl Iterator<Item = FilterAggregationUngroupedQual>> {
//         Ok(self
//             .aggregates
//             .aggregates()
//             .into_iter()
//             .enumerate()
//             .map(|(idx, agg)| {
//                 let agg = agg.to_tantivy_agg().expect(&format!(
//                     "{:?} should be converted to a Tantivy aggregation",
//                     agg
//                 ));
//                 let sub_agg = Aggregations::from([(FilterAggUngroupedKey::NAME.to_string(), agg)]);
//                 FilterAggregationUngroupedQual(sub_agg)
//             }))
//     }
// }

impl From<FilterAggregationGroupedQual> for Aggregations {
    fn from(val: FilterAggregationGroupedQual) -> Self {
        val.0
    }
}

// impl IterFlat<FilterAggregationGroupedQual> for AggregateCSClause {
//     fn into_iter(&self) -> Result<impl Iterator<Item = FilterAggregationGroupedQual>> {
//         Ok(self
//             .aggregates
//             .aggregates()
//             .into_iter()
//             .enumerate()
//             .map(|(idx, agg)| {
//                 let metric_agg = agg.to_tantivy_agg().expect(&format!(
//                     "{:?} should be converted to a Tantivy aggregation",
//                     agg
//                 ));
//                 let sub_agg = Aggregations::from([(idx.to_string(), metric_agg)]);
//                 let terms_agg = <Self as CollectNested<TermsAggregation, GroupedKey>>::collect(
//                     self,
//                     Aggregations::from(sub_agg),
//                 )
//                 .unwrap();

//                 FilterAggregationGroupedQual(terms_agg)
//             }))
//     }
// }

impl From<AggregateType> for Aggregation {
    fn from(val: AggregateType) -> Self {
        let can_use_doc_count = val.can_use_doc_count();
        let filter_query = val.filter_expr().as_ref().map(|query| {
            AggregationVariants::Filter(FilterAggregation::new_with_query(
                to_tantivy_query(query.clone(), val.indexrelid())
                    .expect("should be able to convert to Box<dyn Query>"),
            ))
        });
        let metrics_query = match val {
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
        };

        match (filter_query, metrics_query) {
            (Some(filter_query), metrics_query) => Aggregation {
                agg: filter_query,
                sub_aggregation: Aggregations::from([(
                    0.to_string(),
                    Aggregation {
                        agg: metrics_query,
                        sub_aggregation: Aggregations::new(),
                    },
                )]),
            },
            (None, metrics_query) => Aggregation {
                agg: metrics_query,
                sub_aggregation: Aggregations::new(),
            },
        }
    }
}

#[inline]
fn add_filter_aggregations<Agg, SubAgg>(
    aggregations: &mut Aggregations,
    aggs: impl Iterator<Item = Agg>,
    sub_aggs: impl Iterator<Item = SubAgg>,
) where
    Agg: Into<FilterAggregation>,
    SubAgg: Into<Aggregations>,
{
    for (idx, (aggregation, sub_aggregation)) in aggs.zip(sub_aggs).enumerate() {
        let agg = Aggregation {
            agg: AggregationVariants::Filter(aggregation.into()),
            sub_aggregation: sub_aggregation.into(),
        };
        aggregations.insert(format!("{}_{}", FilterKey::NAME, idx), agg);
    }
}

#[inline]
fn to_tantivy_query(query: SearchQueryInput, indexrelid: pg_sys::Oid) -> Result<Box<dyn Query>> {
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
    Ok(Box::new(query.clone().into_tantivy_query(
        &schema,
        &parser,
        reader.searcher(),
        index.oid(),
        heap_oid,
        NonNull::new(standalone_context.as_ptr()),
        None,
    )?))
}

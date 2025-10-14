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
use crate::postgres::customscan::aggregatescan::groupby::GroupByClause;
use crate::postgres::customscan::aggregatescan::limit_offset::LimitOffsetClause;
use crate::postgres::customscan::aggregatescan::orderby::OrderByClause;
use crate::postgres::customscan::aggregatescan::quals::WhereClause;
use crate::postgres::customscan::aggregatescan::targetlist::TargetList;
use crate::postgres::customscan::aggregatescan::{AggregateClause, AggregateScan};
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::CustomScan;
use crate::postgres::utils::ExprContextGuard;
use crate::postgres::var::{find_one_var_and_fieldname, find_var_relation, VarContext};
use crate::postgres::PgSearchRelation;

use pgrx::pg_sys;
use pgrx::PgList;
use std::collections::HashMap;
use std::ptr::NonNull;
use tantivy::aggregation::agg_req::Aggregations;
use tantivy::aggregation::agg_req::{Aggregation, AggregationVariants};
use tantivy::aggregation::bucket::{CustomOrder, FilterAggregation, OrderTarget, TermsAggregation};
use tantivy::aggregation::intermediate_agg_result::IntermediateAggregationResults;
use tantivy::aggregation::{AggregationLimitsGuard, DistributedAggregationCollector};
use tantivy::query::{AllQuery, Query, QueryParser};

pub(crate) struct AggregationsClause {
    aggregates: TargetList,
    groupby: GroupByClause,
    orderby: OrderByClause,
    limit_offset: LimitOffsetClause,
    quals: WhereClause,
    indexrelid: pg_sys::Oid,
}

impl AggregateClause<AggregateScan> for AggregationsClause {
    type Args = <AggregateScan as CustomScan>::Args;

    fn add_to_custom_path(
        &self,
        builder: CustomPathBuilder<AggregateScan>,
    ) -> CustomPathBuilder<AggregateScan> {
        let mut builder = self.groupby.add_to_custom_path(builder);
        builder = self.aggregates.add_to_custom_path(builder);
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
        let groupby = GroupByClause::from_pg(args, heap_rti, index)?;
        let aggregates = TargetList::from_pg(args, heap_rti, index)?;
        let orderby = OrderByClause::from_pg(args, heap_rti, index)?;
        let limit_offset = LimitOffsetClause::from_pg(args, heap_rti, index)?;
        let quals = WhereClause::from_pg(args, heap_rti, index)?;

        Some(Self {
            groupby,
            aggregates,
            orderby,
            limit_offset,
            quals,
            indexrelid: index.oid(),
        })
    }
}

pub(crate) struct TantivyAggregations(Aggregations);

impl TantivyAggregations {
    pub fn new(aggregations: Aggregations) -> Self {
        Self(aggregations)
    }
}

impl TryFrom<AggregationsClause> for TantivyAggregations {
    type Error = anyhow::Error;

    fn try_from(clause: AggregationsClause) -> Result<Self, Self::Error> {
        let mut aggregations = Aggregations::new();
        let standalone_context = ExprContextGuard::new();

        let index = PgSearchRelation::with_lock(clause.indexrelid, pg_sys::AccessShareLock as _);
        let schema = index.schema()?;
        let reader = SearchIndexReader::open_with_context(
            &index,
            clause.quals.query().clone(),
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

        let qual = FilterAggregation::new_with_query(Box::new(
            clause.quals.query().clone().into_tantivy_query(
                &schema,
                &parser,
                reader.searcher(),
                index.oid(),
                heap_oid,
                NonNull::new(standalone_context.as_ptr()),
                None,
            )?,
        ));

        let filter_aggs = clause.aggregates.aggregates().iter().map(|agg| {
            if let Some(agg) = agg.filter_expr() {
                FilterAggregation::new_with_query(Box::new(
                    agg.clone()
                        .into_tantivy_query(
                            &schema,
                            &parser,
                            reader.searcher(),
                            index.oid(),
                            heap_oid,
                            NonNull::new(standalone_context.as_ptr()),
                            None,
                        )
                        .unwrap(),
                ))
            } else {
                FilterAggregation::new_with_query(Box::new(AllQuery))
            }
        });
        todo!()
    }
}

trait AggregationKey {
    const NAME: &'static str;
}

struct GroupedKey;
impl AggregationKey for GroupedKey {
    const NAME: &'static str = "grouped";
}

struct QualKey;
impl AggregationKey for QualKey {
    const NAME: &'static str = "filter_sentinel";
}

struct FilterKey;
impl AggregationKey for FilterKey {
    const NAME: &'static str = "filtered_agg";
}

trait CollectAggregations<Leaf, Key: AggregationKey> {
    fn aggregation_variant(&self, leaf: Leaf) -> AggregationVariants;
    fn iter_leaves(&self) -> impl Iterator<Item = Leaf>;

    fn collect(&self, sub_aggregations: Aggregations) -> Aggregations {
        let mut aggregations = sub_aggregations;
        for leaf in self.iter_leaves() {
            let aggregation = Aggregation {
                agg: self.aggregation_variant(leaf),
                sub_aggregation: aggregations,
            };
            aggregations = HashMap::from([(Key::NAME.to_string(), aggregation)]);
        }
        aggregations
    }
}

impl CollectAggregations<TermsAggregation, GroupedKey> for AggregationsClause {
    fn aggregation_variant(&self, leaf: TermsAggregation) -> AggregationVariants {
        AggregationVariants::Terms(leaf)
    }

    fn iter_leaves(&self) -> impl Iterator<Item = TermsAggregation> {
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

        let grouping_columns = self.groupby.grouping_columns();
        grouping_columns.into_iter().map(move |column| {
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

            terms_agg
        })
    }
}

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

//! Builder for Tantivy aggregation queries
//!
//! This module consolidates the logic for building Tantivy `Aggregations` structures
//! from ParadeDB's SQL aggregation parameters.

use super::tantivy_keys::{FILTERED_AGG, FILTER_SENTINEL, GROUPED, HIDDEN_DOC_COUNT, SORT_KEY};
use super::QueryContext;
use crate::aggregate::agg_spec::AggregationSpec;
use crate::aggregate::tantivy_keys::{
    filter_key, CTID, FIELD, MISSING, ORDER, SEGMENT_SIZE, SIZE, TERMS, VALUE_COUNT,
};
use crate::api::{FieldName, OrderByFeature, OrderByInfo};
use crate::postgres::customscan::aggregatescan::privdat::AggregateType;
use crate::postgres::utils::sort_json_keys;
use crate::query::SearchQueryInput;
use std::collections::HashMap;
use std::error::Error;
use tantivy::aggregation::agg_req::{Aggregation, AggregationVariants, Aggregations};
use tantivy::aggregation::bucket::FilterAggregation;

/// Builder for constructing Tantivy aggregation queries
pub struct AggQueryBuilder<'a> {
    pub base_query: &'a SearchQueryInput,
    pub agg_spec: &'a AggregationSpec,
    pub orderby_info: &'a [OrderByInfo],
    pub limit: &'a Option<u32>,
    pub offset: &'a Option<u32>,
}

impl<'a> AggQueryBuilder<'a> {
    pub fn new(
        base_query: &'a SearchQueryInput,
        agg_spec: &'a AggregationSpec,
        orderby_info: &'a [OrderByInfo],
        limit: &'a Option<u32>,
        offset: &'a Option<u32>,
    ) -> Self {
        Self {
            base_query,
            agg_spec,
            orderby_info,
            limit,
            offset,
        }
    }

    /// Build Tantivy aggregations from SearchQueryInput (execution path)
    pub fn build_tantivy_query(&self, qctx: &QueryContext) -> Result<Aggregations, Box<dyn Error>> {
        self.build_tantivy_query_for(|query_input| {
            Self::to_tantivy_query(qctx, query_input).map(FilterAggregation::new_with_query)
        })
    }

    /// Build aggregation JSON for EXPLAIN output (no QueryContext needed)
    /// Uses query strings ("*") instead of Query objects, making the output serializable
    pub fn build_tantivy_query_for_explain(&self) -> Result<String, Box<dyn Error>> {
        let aggregations = self
            .build_tantivy_query_for(|_query_input| Ok(FilterAggregation::new("*".to_string())))?;

        // Serialize to JSON and sort keys for deterministic output
        let mut json_value = serde_json::to_value(&aggregations)?;
        sort_json_keys(&mut json_value);
        Ok(serde_json::to_string(&json_value)?)
    }

    /// Build aggregations with a custom FilterAggregation factory
    ///
    /// This is the core aggregation building logic, parameterized by how to create FilterAggregations.
    /// Used by both execution path (with Query objects) and EXPLAIN path (with placeholder strings).
    fn build_tantivy_query_for<F>(&self, mut make_filter: F) -> Result<Aggregations, Box<dyn Error>>
    where
        F: FnMut(Option<&SearchQueryInput>) -> Result<FilterAggregation, Box<dyn Error>>,
    {
        if !self.has_filters() {
            // Optimized path: no FILTER clauses, build simpler aggregation structure
            return self.build_without_filters();
        }

        // FilterAggregation path: some aggregates have FILTER clauses
        let base_filter = make_filter(Some(self.base_query))?;

        // Convert filter queries to FilterAggregations
        let filter_aggregations: Result<Vec<FilterAggregation>, Box<dyn Error>> = self
            .agg_spec
            .aggs
            .iter()
            .map(|agg| make_filter(agg.filter_expr().as_ref()))
            .collect();

        self.build_with_filters(base_filter, filter_aggregations?)
    }

    /// Build aggregations for queries without FILTER clauses
    /// Direct aggregation structure without FilterAggregation wrappers
    fn build_without_filters(&self) -> Result<Aggregations, Box<dyn Error>> {
        let mut result = HashMap::new();

        // Build metrics
        let metrics = self.build_metrics()?;

        if !self.agg_spec.groupby.is_empty() {
            // GROUP BY: build nested terms with metrics at the leaf level
            let nested_terms = self.build_nested_terms(metrics)?;
            result.extend(nested_terms);
        } else {
            // Simple aggregation: metrics at top level
            result.extend(metrics);
        }

        Ok(Aggregations::from(result))
    }

    /// Build aggregations for queries with FILTER clauses
    /// Uses FilterAggregation wrappers for each filtered aggregate
    fn build_with_filters(
        &self,
        base_filter: FilterAggregation,
        filter_aggregations: Vec<FilterAggregation>,
    ) -> Result<Aggregations, Box<dyn Error>> {
        let mut result = HashMap::new();

        // Build nested terms structure if we have grouping columns
        let nested_terms = if !self.agg_spec.groupby.is_empty() {
            Some(self.build_nested_terms(HashMap::new())?)
        } else {
            None
        };

        // Sentinel filter: always present, ensures we get ALL groups (or single row for simple aggs)
        // Uses base_query to match all documents in the WHERE clause
        result.insert(
            FILTER_SENTINEL.to_string(),
            Aggregation {
                agg: AggregationVariants::Filter(base_filter),
                sub_aggregation: Aggregations::from(nested_terms.clone().unwrap_or_default()),
            },
        );

        // Each aggregate: FilterAggregation(filter) -> grouped/filtered_agg -> metric
        for (idx, agg) in self.agg_spec.aggs.iter().enumerate() {
            let filter_agg = filter_aggregations
                .get(idx)
                .ok_or_else(|| format!("Missing filter aggregation for aggregate {}", idx))?;
            let base = AggregateType::to_tantivy_agg(agg)?;

            let sub_aggs = if nested_terms.is_some() {
                // GROUP BY: filter -> grouped -> buckets -> metric
                let mut metric_leaf = HashMap::default();
                metric_leaf.insert(idx.to_string(), base);
                self.build_nested_terms(metric_leaf)?
            } else {
                // No GROUP BY: filter -> filtered_agg (metric)
                let mut aggs = HashMap::default();
                aggs.insert(FILTERED_AGG.to_string(), base);
                aggs
            };

            result.insert(
                filter_key(idx),
                Aggregation {
                    agg: AggregationVariants::Filter(filter_agg.clone()),
                    sub_aggregation: Aggregations::from(sub_aggs),
                },
            );
        }

        Ok(Aggregations::from(result))
    }

    /// Build metric aggregations for all aggregate types
    fn build_metrics(&self) -> Result<HashMap<String, Aggregation>, Box<dyn Error>> {
        let mut metrics = HashMap::new();

        for (idx, agg) in self.agg_spec.aggs.iter().enumerate() {
            if self.agg_spec.needs_explicit_metric(agg) {
                metrics.insert(idx.to_string(), AggregateType::to_tantivy_agg(agg)?);
            }
        }

        // For simple (non-GROUP BY) aggregations, add a hidden _doc_count to detect empty results
        // This is needed for correct NULL handling (SUM/AVG/MIN/MAX return NULL on empty sets)
        if self.agg_spec.needs_hidden_doc_count() {
            metrics.insert(
                HIDDEN_DOC_COUNT.to_string(),
                Aggregation {
                    agg: serde_json::from_value(serde_json::json!({
                        VALUE_COUNT: {FIELD: CTID, MISSING: null}
                    }))?,
                    sub_aggregation: Aggregations::default(),
                },
            );
        }

        Ok(metrics)
    }

    /// Build nested TermsAggregation structure for GROUP BY columns
    ///
    /// Creates a chain of nested TermsAggregations, one for each grouping column.
    /// The leaf level contains the provided metric aggregations.
    ///
    /// Example for 2 grouping columns:
    /// ```json
    /// {
    ///   "grouped": {
    ///     "terms": { "field": "category", "size": ... },
    ///     "aggs": {
    ///       "grouped": {
    ///         "terms": { "field": "status", "size": ... },
    ///         "aggs": { "0": {...}, "1": {...} }  // metrics
    ///       }
    ///     }
    ///   }
    /// }
    /// ```
    fn build_nested_terms(
        &self,
        leaf_metrics: HashMap<String, Aggregation>,
    ) -> Result<HashMap<String, Aggregation>, Box<dyn Error>> {
        if self.agg_spec.groupby.is_empty() {
            return Ok(HashMap::default());
        }

        // Calculate the size for each level based on LIMIT/OFFSET
        let max_buckets = crate::gucs::max_term_agg_buckets() as u32;
        let size = match (self.limit, self.offset) {
            (Some(lim), Some(off)) => std::cmp::min(lim + off, max_buckets),
            (Some(lim), None) => std::cmp::min(*lim, max_buckets),
            (None, Some(off)) => std::cmp::min(max_buckets, max_buckets.saturating_add(*off)),
            (None, None) => max_buckets,
        };

        // Build from the innermost (last) grouping column outward
        let mut current_aggs = leaf_metrics;

        for (depth, column) in self.agg_spec.groupby.iter().enumerate().rev() {
            let order = self.get_order_for_depth(depth);

            // Build terms JSON, conditionally including order if specified
            let mut terms_json = serde_json::json!({
                FIELD: column.field_name,
                SIZE: size,
                SEGMENT_SIZE: size,
            });

            if let Some(order_value) = order {
                terms_json[ORDER] = order_value;
            }

            let terms_agg = Aggregation {
                agg: serde_json::from_value(serde_json::json!({
                    TERMS: terms_json
                }))?,
                sub_aggregation: Aggregations::from(current_aggs.clone()),
            };

            let mut next_level = HashMap::default();
            next_level.insert(GROUPED.to_string(), terms_agg);
            current_aggs = next_level;
        }

        Ok(current_aggs)
    }

    /// Get ORDER BY specification for a specific grouping depth
    /// Returns None if no ORDER BY is specified for this depth
    fn get_order_for_depth(&self, depth: usize) -> Option<serde_json::Value> {
        self.orderby_info
            .iter()
            .find(|order| {
                if let OrderByFeature::Field(field_name) = &order.feature {
                    self.agg_spec
                        .groupby
                        .get(depth)
                        .map(|col| field_name == &FieldName::from(col.field_name.clone()))
                        .unwrap_or(false)
                } else {
                    false
                }
            })
            .map(|order| {
                serde_json::json!({
                    SORT_KEY: order.direction.as_ref()
                })
            })
    }

    /// Convert SearchQueryInput to Tantivy Query, or AllQuery if None
    fn to_tantivy_query(
        qctx: &QueryContext,
        filter: Option<&crate::query::SearchQueryInput>,
    ) -> Result<Box<dyn tantivy::query::Query>, Box<dyn std::error::Error>> {
        Ok(match filter {
            Some(query) => query.clone().into_tantivy_query(
                qctx.schema,
                &|| {
                    tantivy::query::QueryParser::for_index(
                        qctx.reader.searcher().index(),
                        qctx.schema.fields().map(|(f, _)| f).collect(),
                    )
                },
                qctx.reader.searcher(),
                qctx.index.oid(),
                qctx.index.heap_relation().map(|r| r.oid()),
                std::ptr::NonNull::new(qctx.context.as_ptr()),
                None,
            )?,
            None => Box::new(tantivy::query::AllQuery),
        })
    }

    /// Check if any aggregates have FILTER clauses
    pub fn has_filters(&self) -> bool {
        self.agg_spec
            .aggs
            .iter()
            .any(|agg| agg.filter_expr().is_some())
    }
}

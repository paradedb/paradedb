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

//! Strategy pattern for aggregation building
//!
//! Separates the logic for building aggregations with and without FILTER clauses.

use super::errors::AggregationError;
use super::tantivy_keys::{FILTERED_AGG, FILTER_SENTINEL, GROUPED};
use super::AggQueryBuilder;
use crate::postgres::customscan::aggregatescan::privdat::AggregateType;
use crate::query::SearchQueryInput;
use std::collections::HashMap;
use tantivy::aggregation::agg_req::{Aggregation, AggregationVariants, Aggregations};
use tantivy::aggregation::bucket::FilterAggregation;

/// Strategy for building Tantivy aggregations
pub trait AggregationStrategy {
    /// Build aggregations from the query builder
    fn build(&self, builder: &AggQueryBuilder) -> Result<Aggregations, AggregationError>;
}

/// Strategy for direct aggregations (no FILTER clauses)
///
/// Builds a simpler aggregation structure without FilterAggregation wrappers.
/// For GROUP BY queries, metrics are nested within TermsAggregation.
/// For simple aggregations, metrics are at the top level.
pub struct DirectAggregationStrategy;

impl AggregationStrategy for DirectAggregationStrategy {
    fn build(&self, builder: &AggQueryBuilder) -> Result<Aggregations, AggregationError> {
        let mut result = HashMap::new();

        // Build metrics
        let metrics = builder.build_metrics()?;

        if !builder.agg_spec.groupby.is_empty() {
            // GROUP BY: build nested terms with metrics at the leaf level
            let nested_terms = builder.build_nested_terms(metrics)?;
            result.extend(nested_terms);
        } else {
            // Simple aggregation: metrics at top level
            result.extend(metrics);
        }

        Ok(Aggregations::from(result))
    }
}

/// Strategy for filtered aggregations (with FILTER clauses)
///
/// Uses FilterAggregation wrappers for each filtered aggregate.
/// Includes a sentinel filter to ensure all groups are generated.
pub struct FilteredAggregationStrategy<F> {
    /// Factory function to create FilterAggregation instances
    pub filter_factory: F,
}

impl<F> AggregationStrategy for FilteredAggregationStrategy<F>
where
    F: Fn(Option<&SearchQueryInput>) -> Result<FilterAggregation, AggregationError>,
{
    fn build(&self, builder: &AggQueryBuilder) -> Result<Aggregations, AggregationError> {
        let mut result = HashMap::new();

        // Create base filter for sentinel
        let base_filter = (self.filter_factory)(Some(builder.base_query))?;

        // Build nested terms structure if we have grouping columns
        let nested_terms = if !builder.agg_spec.groupby.is_empty() {
            Some(builder.build_nested_terms(HashMap::new())?)
        } else {
            None
        };

        // Sentinel filter: always present, ensures we get ALL groups (or single row for simple aggs)
        // Uses base_query to match all documents in the WHERE clause
        result.insert(
            FILTER_SENTINEL.to_string(),
            Aggregation {
                agg: AggregationVariants::Filter(base_filter),
                sub_aggregation: if let Some(nested) = nested_terms {
                    Aggregations::from(nested)
                } else {
                    Aggregations::from(HashMap::new())
                },
            },
        );

        // Build FilterAggregations for each filtered aggregate
        for (i, agg) in builder.agg_spec.aggs.iter().enumerate() {
            let filter = (self.filter_factory)(agg.filter_expr().as_ref())?;
            let metric_agg = AggregateType::to_tantivy_agg(agg)
                .map_err(|e| AggregationError::BuildError(e.to_string()))?;

            let sub_agg = if !builder.agg_spec.groupby.is_empty() {
                // GROUP BY: wrap metric in nested terms
                let mut terms_with_metric = builder.build_nested_terms(HashMap::new())?;

                // Insert metric at the deepest level
                Self::insert_metric_at_leaf(&mut terms_with_metric, i.to_string(), metric_agg);

                Aggregations::from(terms_with_metric)
            } else {
                // Simple aggregation: metric directly under filter
                let mut sub = HashMap::new();
                sub.insert(FILTERED_AGG.to_string(), metric_agg);
                Aggregations::from(sub)
            };

            result.insert(
                super::tantivy_keys::filter_key(i),
                Aggregation {
                    agg: AggregationVariants::Filter(filter),
                    sub_aggregation: sub_agg,
                },
            );
        }

        Ok(Aggregations::from(result))
    }
}

impl<F> FilteredAggregationStrategy<F> {
    /// Insert a metric at the deepest level of nested TermsAggregation
    fn insert_metric_at_leaf(
        aggregations: &mut HashMap<String, Aggregation>,
        key: String,
        metric: Aggregation,
    ) {
        // Find the deepest GROUPED aggregation
        if let Some(grouped_agg) = aggregations.get_mut(GROUPED) {
            if grouped_agg.sub_aggregation.is_empty() {
                // This is the deepest level, insert here
                grouped_agg.sub_aggregation.insert(key, metric);
            } else {
                // Recurse into sub-aggregations
                let mut sub_map = grouped_agg.sub_aggregation.clone();
                Self::insert_metric_at_leaf(&mut sub_map, key, metric);
                grouped_agg.sub_aggregation = Aggregations::from(sub_map);
            }
        }
    }
}

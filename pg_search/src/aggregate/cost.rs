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

use super::AggregateRequest;
use crate::postgres::customscan::aggregatescan::AggregateType;
use crate::schema::SearchIndexSchema;
use tantivy::aggregation::agg_req::{Aggregation, AggregationVariants};
use tantivy::aggregation::metric::{IntermediateExtendedStats, IntermediateStats};

// Relative weights multiplied by cpu_operator_cost, not calibrated CPU timings.
const GROUP_KEY_OPS_PER_ROW: f64 = 3.0;
const GROUP_KEY_OPS_PER_GROUP: f64 = 4.0;
const STATS_OPS_PER_VALUE: f64 = 4.0;
const EXTENDED_STATS_OPS_PER_VALUE: f64 = 8.0;
const CARDINALITY_OPS_PER_VALUE: f64 = 8.0;
const PERCENTILES_OPS_PER_VALUE: f64 = 16.0;
const TOP_HITS_OPS_PER_HEAP_LEVEL: f64 = 2.0;
const BUCKET_OPS_PER_VALUE: f64 = 3.0;

// Approximate variable-state sizes; fixed metric state uses Tantivy's type sizes below.
const GROUP_KEY_STATE_BYTES: f64 = 16.0;
const CARDINALITY_STATE_BYTES: f64 = 2.0 * 1024.0;
const PERCENTILES_STATE_BYTES: f64 = 32.0 * 1024.0;
const TOP_HITS_BYTES_PER_FIELD: f64 = 16.0;
const TOP_HITS_OVERHEAD_BYTES_PER_HIT: f64 = 32.0;
const BUCKET_STATE_BYTES: f64 = 24.0;

// Charge one parallel_tuple_cost per this many estimated partial-state bytes.
pub(super) const TRANSFER_BYTES_PER_TUPLE_COST: f64 = 64.0;

#[derive(Default)]
pub(super) struct AggregateCost {
    pub operations: f64,
    pub state_bytes: f64,
}

impl AggregateCost {
    pub fn estimate_request(
        request: &AggregateRequest,
        rows: Option<u64>,
        participants: f64,
        schema: &SearchIndexSchema,
    ) -> Option<Self> {
        if matches!(request, AggregateRequest::Sql(clause) if clause.is_bare_doc_count()) {
            return Some(Self {
                operations: 0.0,
                state_bytes: size_of::<u64>() as f64,
            });
        }
        let rows = rows? as f64;
        let mut cost = Self::default();
        match request {
            AggregateRequest::Sql(clause) => {
                if clause.has_filter() {
                    return None;
                }
                let columns = clause.grouping_columns();
                let groups = if columns.is_empty() {
                    1.0
                } else {
                    let groups = clause.estimated_groups()?;
                    if !groups.is_finite() || groups < 0.0 {
                        return None;
                    }
                    for column in &columns {
                        scalar_field(schema, &column.field_name)?;
                    }
                    groups.min(rows).max(1.0)
                };
                let partial_groups = groups.min((rows / participants).max(1.0));
                let keys = columns.len() as f64;
                cost.operations =
                    rows * keys * GROUP_KEY_OPS_PER_ROW + groups * keys * GROUP_KEY_OPS_PER_GROUP;
                cost.state_bytes = partial_groups * keys * GROUP_KEY_STATE_BYTES;
                for aggregate in clause.aggregates() {
                    if clause.can_use_doc_count(aggregate) {
                        continue;
                    }
                    let metric = if matches!(aggregate, AggregateType::CountAny { .. }) {
                        Self {
                            operations: rows,
                            state_bytes: size_of::<u64>() as f64,
                        }
                    } else {
                        Self::estimate_tantivy_tree(
                            &aggregate.clone().into(),
                            rows / groups,
                            schema,
                        )?
                    };
                    cost.operations += metric.operations * groups;
                    cost.state_bytes += metric.state_bytes * partial_groups;
                }
            }
            AggregateRequest::Json(aggregations) => {
                for aggregation in aggregations.values() {
                    let metric = Self::estimate_tantivy_tree(aggregation, rows, schema)?;
                    cost.operations += metric.operations;
                    cost.state_bytes += metric.state_bytes;
                }
            }
        }
        (cost.operations.is_finite() && cost.state_bytes.is_finite()).then_some(cost)
    }

    fn estimate_tantivy_tree(
        aggregation: &Aggregation,
        rows: f64,
        schema: &SearchIndexSchema,
    ) -> Option<Self> {
        use AggregationVariants::*;
        for field in aggregation.agg.get_fast_field_names() {
            scalar_field(schema, field)?;
        }
        let (ops_per_value, state_bytes_per_bucket, buckets) = match &aggregation.agg {
            // These metrics share Tantivy's stats collector and intermediate state.
            Count(_) | Sum(_) | Average(_) | Min(_) | Max(_) | Stats(_) => (
                STATS_OPS_PER_VALUE,
                size_of::<IntermediateStats>() as f64,
                1.0,
            ),
            ExtendedStats(_) => (
                EXTENDED_STATS_OPS_PER_VALUE,
                size_of::<IntermediateExtendedStats>() as f64,
                1.0,
            ),
            Cardinality(_) => (CARDINALITY_OPS_PER_VALUE, CARDINALITY_STATE_BYTES, 1.0),
            Percentiles(_) => (PERCENTILES_OPS_PER_VALUE, PERCENTILES_STATE_BYTES, 1.0),
            TopHits(request) => {
                let json = serde_json::to_value(request).ok()?;
                let retained =
                    json["size"].as_u64()? as f64 + json["from"].as_u64().unwrap_or(0) as f64;
                let fields = request.field_names().len() as f64;
                let heap_levels = retained.max(2.0).log2();
                let bytes_per_hit =
                    fields * TOP_HITS_BYTES_PER_FIELD + TOP_HITS_OVERHEAD_BYTES_PER_HIT;
                (
                    TOP_HITS_OPS_PER_HEAP_LEVEL * heap_levels,
                    retained.min(rows) * bytes_per_hit,
                    1.0,
                )
            }
            Range(request) => {
                let max_boundaries = request.ranges.len() * 2;
                (
                    BUCKET_OPS_PER_VALUE,
                    BUCKET_STATE_BYTES,
                    (max_boundaries + 1) as f64,
                )
            }
            Histogram(request) => {
                let bounds = request.hard_bounds.as_ref()?;
                if request.interval <= 0.0 || !request.interval.is_finite() {
                    return None;
                }
                (
                    BUCKET_OPS_PER_VALUE,
                    BUCKET_STATE_BYTES,
                    ((bounds.max - bounds.min) / request.interval)
                        .ceil()
                        .max(0.0)
                        + 1.0,
                )
            }
            // No cheap group-count or filter-work estimate is available for these requests.
            Terms(_) | MultiTerms(_) | Composite(_) | DateHistogram(_) | Filter(_) => return None,
        };
        let mut cost = Self {
            operations: rows * ops_per_value,
            state_bytes: buckets * state_bytes_per_bucket,
        };
        for child in aggregation.sub_aggregation.values() {
            let child = Self::estimate_tantivy_tree(child, rows / buckets, schema)?;
            cost.operations += child.operations * buckets;
            cost.state_bytes += child.state_bytes * buckets;
        }
        Some(cost)
    }
}

fn scalar_field(schema: &SearchIndexSchema, field: &str) -> Option<()> {
    let field = schema.search_field(field)?;
    schema
        .categorized_fields()
        .iter()
        .any(|(candidate, data)| {
            candidate.field_name().root() == field.field_name().root()
                && !data.is_array
                && !data.is_json
        })
        .then_some(())
}

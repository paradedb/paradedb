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

pub(super) fn estimate_collector_operations(
    request: &AggregateRequest,
    rows: Option<u64>,
    schema: &SearchIndexSchema,
) -> Option<f64> {
    let mut updates_per_doc = 0;
    match request {
        AggregateRequest::Sql(clause) => {
            if clause.is_bare_doc_count() {
                return Some(0.0);
            }
            if clause.has_filter() {
                return None;
            }
            for column in clause.grouping_columns() {
                scalar_field(schema, &column.field_name)?;
                updates_per_doc += 1;
            }
            for aggregate in clause.aggregates() {
                if clause.can_use_doc_count(aggregate) {
                    continue;
                }
                updates_per_doc += if matches!(aggregate, AggregateType::CountAny { .. }) {
                    1
                } else {
                    count_tantivy_collectors(&aggregate.clone().into(), schema)?
                };
            }
        }
        AggregateRequest::Json(aggregations) => {
            for aggregation in aggregations.values() {
                updates_per_doc += count_tantivy_collectors(aggregation, schema)?;
            }
        }
    }
    Some(rows? as f64 * updates_per_doc as f64)
}

fn count_tantivy_collectors(
    aggregation: &Aggregation,
    schema: &SearchIndexSchema,
) -> Option<usize> {
    use AggregationVariants::*;
    if matches!(
        aggregation.agg,
        Terms(_) | MultiTerms(_) | Composite(_) | DateHistogram(_) | Filter(_)
    ) {
        return None;
    }
    for field in aggregation.agg.get_fast_field_names() {
        scalar_field(schema, field)?;
    }
    let mut collectors = 1;
    for child in aggregation.sub_aggregation.values() {
        collectors += count_tantivy_collectors(child, schema)?;
    }
    Some(collectors)
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

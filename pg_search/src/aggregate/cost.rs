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

impl AggregateRequest {
    pub(super) fn updates_per_doc(&self, schema: &SearchIndexSchema) -> Option<usize> {
        let mut updates_per_doc = 0;
        match self {
            AggregateRequest::Sql(clause) => {
                if clause.is_bare_doc_count() {
                    return Some(0);
                }
                if clause.has_filter() {
                    return None;
                }
                for column in clause.grouping_columns() {
                    if !schema.is_scalar_field(&column.field_name) {
                        return None;
                    }
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
        Some(updates_per_doc)
    }
}

fn count_tantivy_collectors(
    aggregation: &Aggregation,
    schema: &SearchIndexSchema,
) -> Option<usize> {
    // Filters evaluate another query whose traversal cost is not included here.
    if matches!(aggregation.agg, AggregationVariants::Filter(_)) {
        return None;
    }
    for field in aggregation.agg.get_fast_field_names() {
        if !schema.is_scalar_field(field) {
            return None;
        }
    }
    let mut collectors = 1;
    for child in aggregation.sub_aggregation.values() {
        collectors += count_tantivy_collectors(child, schema)?;
    }
    Some(collectors)
}

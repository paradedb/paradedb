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

use crate::api::operator::anyelement_query_input_opoid;
use crate::api::{
    agg_funcoid, agg_with_solve_mvcc_funcoid, extract_solve_mvcc_from_const, HashSet,
    MvccVisibility,
};
use crate::customscan::builders::custom_path::RestrictInfoType;
use crate::customscan::solve_expr::SolvePostgresExpressions;
use crate::nodecast;
use crate::postgres::customscan::qual_inspect::{extract_quals, PlannerContext, QualExtractState};
use crate::postgres::types::{ConstNode, TantivyValue};
use crate::postgres::var::fieldname_from_var;
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;
use pgrx::pg_sys::{
    F_AVG_FLOAT4, F_AVG_FLOAT8, F_AVG_INT2, F_AVG_INT4, F_AVG_INT8, F_AVG_NUMERIC, F_COUNT_,
    F_COUNT_ANY, F_MAX_DATE, F_MAX_FLOAT4, F_MAX_FLOAT8, F_MAX_INT2, F_MAX_INT4, F_MAX_INT8,
    F_MAX_NUMERIC, F_MAX_TIME, F_MAX_TIMESTAMP, F_MAX_TIMESTAMPTZ, F_MAX_TIMETZ, F_MIN_DATE,
    F_MIN_FLOAT4, F_MIN_FLOAT8, F_MIN_INT2, F_MIN_INT4, F_MIN_INT8, F_MIN_MONEY, F_MIN_NUMERIC,
    F_MIN_TIME, F_MIN_TIMESTAMP, F_MIN_TIMESTAMPTZ, F_MIN_TIMETZ, F_SUM_FLOAT4, F_SUM_FLOAT8,
    F_SUM_INT2, F_SUM_INT4, F_SUM_INT8, F_SUM_NUMERIC,
};
use pgrx::prelude::*;
use pgrx::PgList;
use tantivy::aggregation::agg_req::AggregationVariants;
use tantivy::aggregation::metric::{
    AverageAggregation, CountAggregation, MaxAggregation, MinAggregation, SingleMetricResult,
    SumAggregation,
};
use tantivy::schema::OwnedValue;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AggregateType {
    CountAny {
        filter: Option<SearchQueryInput>,
        indexrelid: pg_sys::Oid,
    },
    Count {
        field: String,
        missing: Option<f64>,
        filter: Option<SearchQueryInput>,
        indexrelid: pg_sys::Oid,
    },
    Sum {
        field: String,
        missing: Option<f64>,
        filter: Option<SearchQueryInput>,
        indexrelid: pg_sys::Oid,
    },
    Avg {
        field: String,
        missing: Option<f64>,
        filter: Option<SearchQueryInput>,
        indexrelid: pg_sys::Oid,
    },
    Min {
        field: String,
        missing: Option<f64>,
        filter: Option<SearchQueryInput>,
        indexrelid: pg_sys::Oid,
    },
    Max {
        field: String,
        missing: Option<f64>,
        filter: Option<SearchQueryInput>,
        indexrelid: pg_sys::Oid,
    },
    Custom {
        agg_json: serde_json::Value,
        filter: Option<SearchQueryInput>,
        indexrelid: pg_sys::Oid,
        mvcc_visibility: MvccVisibility,
    },
}

impl SolvePostgresExpressions for AggregateType {
    fn has_heap_filters(&mut self) -> bool {
        self.filter_expr_mut()
            .as_mut()
            .is_some_and(|filter| filter.has_heap_filters())
    }

    fn has_postgres_expressions(&mut self) -> bool {
        self.filter_expr_mut()
            .as_mut()
            .is_some_and(|filter| filter.has_postgres_expressions())
    }

    fn init_postgres_expressions(&mut self, planstate: *mut pg_sys::PlanState) {
        if let Some(filter) = self.filter_expr_mut() {
            filter.init_postgres_expressions(planstate);
        }
    }

    fn solve_postgres_expressions(&mut self, expr_context: *mut pg_sys::ExprContext) {
        if let Some(filter) = self.filter_expr_mut() {
            filter.solve_postgres_expressions(expr_context);
        }
    }
}

impl AggregateType {
    pub unsafe fn try_from(
        aggref: *mut pg_sys::Aggref,
        heaprelid: pg_sys::Oid,
        bm25_index: &crate::postgres::PgSearchRelation,
        root: *mut pg_sys::PlannerInfo,
        heap_rti: pg_sys::Index,
        qual_state: &mut QualExtractState,
    ) -> Option<Self> {
        let aggfnoid = (*aggref).aggfnoid.to_u32();

        let args = PgList::<pg_sys::TargetEntry>::from_pg((*aggref).args);

        let filter_expr = if (*aggref).aggfilter.is_null() {
            None
        } else {
            let context = PlannerContext::from_planner(root);
            extract_quals(
                &context,
                heap_rti,
                (*aggref).aggfilter as *mut pg_sys::Node,
                anyelement_query_input_opoid(),
                RestrictInfoType::BaseRelation,
                bm25_index,
                false,
                qual_state,
                true,
            )
        };
        let filter_query = filter_expr.map(|qual| SearchQueryInput::from(&qual));

        // Check for pdb.agg() custom aggregate (both overloads)
        let agg_oid = agg_funcoid().to_u32();
        let agg_with_mvcc_oid = agg_with_solve_mvcc_funcoid().to_u32();

        if aggfnoid == agg_oid || aggfnoid == agg_with_mvcc_oid {
            // Extract JSON argument (first arg)
            let arg = args.get_ptr(0)?;
            let expr = (*arg).expr;
            let json_value = if let Some(const_node) = nodecast!(Const, T_Const, expr) {
                let json_datum = (*const_node).constvalue;
                pgrx::JsonB::from_datum(json_datum, false)?.0
            } else {
                return None;
            };

            // Extract solve_mvcc bool argument (second arg) if using the two-arg overload
            let solve_mvcc = if aggfnoid == agg_with_mvcc_oid {
                args.get_ptr(1)
                    .and_then(|mvcc_arg| nodecast!(Const, T_Const, (*mvcc_arg).expr))
                    .map(|const_node| extract_solve_mvcc_from_const(const_node))
                    .unwrap_or(true)
            } else {
                true // Single-arg overload: default to solve_mvcc = true
            };

            let mvcc_visibility = if solve_mvcc {
                MvccVisibility::Enabled
            } else {
                MvccVisibility::Disabled
            };

            return Some(AggregateType::Custom {
                agg_json: json_value,
                filter: filter_query,
                indexrelid: bm25_index.oid(),
                mvcc_visibility,
            });
        }

        if aggfnoid == F_COUNT_ && (*aggref).aggstar {
            return Some(AggregateType::CountAny {
                filter: filter_query,
                indexrelid: bm25_index.oid(),
            });
        }

        if args.is_empty() {
            return None;
        }

        let first_arg = args.get_ptr(0)?;
        let (field, missing) = parse_aggregate_field(first_arg, heaprelid)?;
        let agg_type =
            create_aggregate_from_oid(aggfnoid, field, missing, filter_query, bm25_index.oid())?;

        Some(agg_type)
    }

    pub fn can_use_doc_count(&self) -> bool {
        matches!(self, AggregateType::CountAny { .. }) && !self.has_filter()
    }

    /// Get the field name for field-based aggregates (None for COUNT and Custom)
    pub fn field_name(&self) -> Option<String> {
        match self {
            AggregateType::CountAny { .. } => None,
            AggregateType::Count { field, .. } => Some(field.clone()),
            AggregateType::Sum { field, .. } => Some(field.clone()),
            AggregateType::Avg { field, .. } => Some(field.clone()),
            AggregateType::Min { field, .. } => Some(field.clone()),
            AggregateType::Max { field, .. } => Some(field.clone()),
            AggregateType::Custom { .. } => None,
        }
    }

    pub fn indexrelid(&self) -> pg_sys::Oid {
        match self {
            AggregateType::CountAny { indexrelid, .. } => *indexrelid,
            AggregateType::Count { indexrelid, .. } => *indexrelid,
            AggregateType::Sum { indexrelid, .. } => *indexrelid,
            AggregateType::Avg { indexrelid, .. } => *indexrelid,
            AggregateType::Min { indexrelid, .. } => *indexrelid,
            AggregateType::Max { indexrelid, .. } => *indexrelid,
            AggregateType::Custom { indexrelid, .. } => *indexrelid,
        }
    }

    pub fn missing(&self) -> Option<f64> {
        match self {
            AggregateType::CountAny { .. } => None,
            AggregateType::Count { missing, .. } => *missing,
            AggregateType::Sum { missing, .. } => *missing,
            AggregateType::Avg { missing, .. } => *missing,
            AggregateType::Min { missing, .. } => *missing,
            AggregateType::Max { missing, .. } => *missing,
            AggregateType::Custom { .. } => None,
        }
    }

    pub fn nullish(&self) -> SingleMetricResult {
        match self {
            AggregateType::CountAny { .. } | AggregateType::Count { .. } => {
                SingleMetricResult { value: Some(0.0) }
            }
            AggregateType::Sum { .. }
            | AggregateType::Avg { .. }
            | AggregateType::Min { .. }
            | AggregateType::Max { .. }
            | AggregateType::Custom { .. } => SingleMetricResult { value: None },
        }
    }

    /// Check if this aggregate has a filter
    pub fn has_filter(&self) -> bool {
        match self {
            AggregateType::CountAny { filter, .. } => filter.is_some(),
            AggregateType::Count { filter, .. } => filter.is_some(),
            AggregateType::Sum { filter, .. } => filter.is_some(),
            AggregateType::Avg { filter, .. } => filter.is_some(),
            AggregateType::Min { filter, .. } => filter.is_some(),
            AggregateType::Max { filter, .. } => filter.is_some(),
            AggregateType::Custom { filter, .. } => filter.is_some(),
        }
    }

    /// Get the filter expression if present
    pub fn filter_expr(&self) -> &Option<SearchQueryInput> {
        match self {
            AggregateType::CountAny { filter, .. } => filter,
            AggregateType::Count { filter, .. } => filter,
            AggregateType::Sum { filter, .. } => filter,
            AggregateType::Avg { filter, .. } => filter,
            AggregateType::Min { filter, .. } => filter,
            AggregateType::Max { filter, .. } => filter,
            AggregateType::Custom { filter, .. } => filter,
        }
    }

    pub fn filter_expr_mut(&mut self) -> &mut Option<SearchQueryInput> {
        match self {
            AggregateType::CountAny { filter, .. } => filter,
            AggregateType::Count { filter, .. } => filter,
            AggregateType::Sum { filter, .. } => filter,
            AggregateType::Avg { filter, .. } => filter,
            AggregateType::Min { filter, .. } => filter,
            AggregateType::Max { filter, .. } => filter,
            AggregateType::Custom { filter, .. } => filter,
        }
    }

    /// Get the MVCC visibility setting for this aggregate.
    /// Only Custom aggregates (pdb.agg) can have non-default MVCC settings.
    /// All standard SQL aggregates (COUNT, SUM, etc.) use the default (Enabled).
    pub fn mvcc_visibility(&self) -> MvccVisibility {
        match self {
            AggregateType::Custom {
                mvcc_visibility, ..
            } => *mvcc_visibility,
            // Standard SQL aggregates always use default MVCC behavior
            _ => MvccVisibility::default(),
        }
    }

    pub fn result_type_oid(&self) -> pg_sys::Oid {
        match &self {
            AggregateType::CountAny { .. } | AggregateType::Count { .. } => pg_sys::INT8OID,
            AggregateType::Sum { .. }
            | AggregateType::Avg { .. }
            | AggregateType::Min { .. }
            | AggregateType::Max { .. } => pg_sys::FLOAT8OID,
            AggregateType::Custom { .. } => pg_sys::JSONBOID,
        }
    }

    /// Validate that all fields referenced in a Custom aggregate exist in the index schema.
    /// Returns an error if any field is invalid.
    /// TODO: remove this once the Tantivy aggregation validation issue is fixed.
    /// https://github.com/quickwit-oss/tantivy/issues/2767
    pub fn validate_fields(&self, schema: &SearchIndexSchema) -> Result<(), String> {
        if let AggregateType::Custom { agg_json, .. } = self {
            let fields = extract_fields_from_agg_json(agg_json);
            let indexed_fields: HashSet<String> = schema
                .fields()
                .map(|(_, entry)| entry.name().to_string())
                .collect();

            for field in &fields {
                if !indexed_fields.contains(field) {
                    // Build a sorted list of available fields for the error message
                    let mut available: Vec<_> = indexed_fields
                        .iter()
                        .filter(|f| *f != "ctid") // Don't show internal ctid field
                        .cloned()
                        .collect();
                    available.sort();
                    return Err(format!(
                        "pdb.agg() references invalid field '{}'. Available indexed fields are: [{}]",
                        field,
                        available.join(", ")
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Recursively extract all "field" values from an aggregation JSON structure.
/// Handles nested aggregations via the "aggs" key.
fn extract_fields_from_agg_json(json: &serde_json::Value) -> HashSet<String> {
    let mut fields = HashSet::default();
    extract_fields_recursive(json, &mut fields);
    fields
}

fn extract_fields_recursive(json: &serde_json::Value, fields: &mut HashSet<String>) {
    match json {
        serde_json::Value::Object(map) => {
            // Check for a "field" key at this level
            if let Some(serde_json::Value::String(field_name)) = map.get("field") {
                fields.insert(field_name.clone());
            }

            // Recurse into all values
            for (key, value) in map {
                extract_fields_recursive(value, fields);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                extract_fields_recursive(item, fields);
            }
        }
        _ => {}
    }
}

impl std::fmt::Display for AggregateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AggregateType::CountAny { .. } => write!(f, "COUNT(*)"),
            AggregateType::Count { .. } => write!(f, "COUNT({})", self.field_name().unwrap()),
            AggregateType::Sum { .. } => write!(f, "SUM({})", self.field_name().unwrap()),
            AggregateType::Avg { .. } => write!(f, "AVG({})", self.field_name().unwrap()),
            AggregateType::Min { .. } => write!(f, "MIN({})", self.field_name().unwrap()),
            AggregateType::Max { .. } => write!(f, "MAX({})", self.field_name().unwrap()),
            AggregateType::Custom { agg_json, .. } => write!(f, "CUSTOM_AGG({})", agg_json),
        }
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
            AggregateType::Custom { agg_json, .. } => {
                // For Custom aggregates, deserialize the JSON directly into AggregationVariants
                serde_json::from_value(agg_json)
                    .unwrap_or_else(|e| panic!("Failed to deserialize custom aggregate: {}", e))
            }
        }
    }
}

trait F64Lossless {
    fn to_f64_lossless(self) -> Option<f64>;
}

impl F64Lossless for u64 {
    fn to_f64_lossless(self) -> Option<f64> {
        let f = self as f64;
        if f as u64 == self {
            Some(f)
        } else {
            None
        }
    }
}

impl F64Lossless for i64 {
    fn to_f64_lossless(self) -> Option<f64> {
        let f = self as f64;
        if f as i64 == self {
            Some(f)
        } else {
            None
        }
    }
}

/// Parse field name and missing value from aggregate argument
unsafe fn parse_aggregate_field(
    first_arg: *mut pg_sys::TargetEntry,
    heaprelid: pg_sys::Oid,
) -> Option<(String, Option<f64>)> {
    let (var, missing) =
        if let Some(coalesce_node) = nodecast!(CoalesceExpr, T_CoalesceExpr, (*first_arg).expr) {
            parse_coalesce_expression(coalesce_node)?
        } else if let Some(var) = nodecast!(Var, T_Var, (*first_arg).expr) {
            (var, None)
        } else {
            return None;
        };

    let field = fieldname_from_var(heaprelid, var, (*var).varattno)?.into_inner();
    Some((field, missing))
}

/// Parse COALESCE expression to extract variable and missing value
pub unsafe fn parse_coalesce_expression(
    coalesce_node: *mut pg_sys::CoalesceExpr,
) -> Option<(*mut pg_sys::Var, Option<f64>)> {
    let args = PgList::<pg_sys::Node>::from_pg((*coalesce_node).args);
    if args.is_empty() {
        return None;
    }

    let var = nodecast!(Var, T_Var, args.get_ptr(0)?)?;
    let const_node = ConstNode::try_from(args.get_ptr(1)?)?;
    let missing = match TantivyValue::try_from(const_node) {
        Ok(TantivyValue(OwnedValue::U64(missing))) => missing.to_f64_lossless(),
        Ok(TantivyValue(OwnedValue::I64(missing))) => missing.to_f64_lossless(),
        Ok(TantivyValue(OwnedValue::F64(missing))) => Some(missing),
        Ok(TantivyValue(OwnedValue::Null)) => None,
        _ => return None,
    };

    Some((var, missing))
}

/// Create appropriate AggregateType from function OID
pub fn create_aggregate_from_oid(
    aggfnoid: u32,
    field: String,
    missing: Option<f64>,
    filter: Option<SearchQueryInput>,
    indexrelid: pg_sys::Oid,
) -> Option<AggregateType> {
    match aggfnoid {
        F_COUNT_ANY => Some(AggregateType::Count {
            field,
            missing,
            filter,
            indexrelid,
        }),
        F_AVG_INT8 | F_AVG_INT4 | F_AVG_INT2 | F_AVG_NUMERIC | F_AVG_FLOAT4 | F_AVG_FLOAT8 => {
            Some(AggregateType::Avg {
                field,
                missing,
                filter,
                indexrelid,
            })
        }
        F_SUM_INT8 | F_SUM_INT4 | F_SUM_INT2 | F_SUM_FLOAT4 | F_SUM_FLOAT8 | F_SUM_NUMERIC => {
            Some(AggregateType::Sum {
                field,
                missing,
                filter,
                indexrelid,
            })
        }
        F_MAX_INT8 | F_MAX_INT4 | F_MAX_INT2 | F_MAX_FLOAT4 | F_MAX_FLOAT8 | F_MAX_DATE
        | F_MAX_TIME | F_MAX_TIMETZ | F_MAX_TIMESTAMP | F_MAX_TIMESTAMPTZ | F_MAX_NUMERIC => {
            Some(AggregateType::Max {
                field,
                missing,
                filter,
                indexrelid,
            })
        }
        F_MIN_INT8 | F_MIN_INT4 | F_MIN_INT2 | F_MIN_FLOAT4 | F_MIN_FLOAT8 | F_MIN_DATE
        | F_MIN_TIME | F_MIN_TIMETZ | F_MIN_MONEY | F_MIN_TIMESTAMP | F_MIN_TIMESTAMPTZ
        | F_MIN_NUMERIC => Some(AggregateType::Min {
            field,
            missing,
            filter,
            indexrelid,
        }),
        _ => {
            pgrx::debug1!("Unknown aggregate function OID: {}", aggfnoid);
            None
        }
    }
}

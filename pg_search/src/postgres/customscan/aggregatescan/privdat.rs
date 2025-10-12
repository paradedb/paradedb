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

use crate::api::{AsCStr, OrderByInfo};
use crate::customscan::aggregatescan::GroupingColumn;
use crate::customscan::solve_expr::SolvePostgresExpressions;
use crate::nodecast;
use crate::postgres::customscan::explain::ExplainFormat;
use crate::postgres::customscan::qual_inspect::QualExtractState;
use crate::postgres::types::{ConstNode, TantivyValue};
use crate::postgres::var::fieldname_from_var;
use crate::query::SearchQueryInput;
use pgrx::pg_sys::AsPgCStr;
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
use serde::Deserialize;
use tantivy::schema::OwnedValue;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AggregateType {
    CountAny {
        filter: Option<SearchQueryInput>,
    },
    Count {
        field: String,
        missing: Option<f64>,
        filter: Option<SearchQueryInput>,
    },
    Sum {
        field: String,
        missing: Option<f64>,
        filter: Option<SearchQueryInput>,
    },
    Avg {
        field: String,
        missing: Option<f64>,
        filter: Option<SearchQueryInput>,
    },
    Min {
        field: String,
        missing: Option<f64>,
        filter: Option<SearchQueryInput>,
    },
    Max {
        field: String,
        missing: Option<f64>,
        filter: Option<SearchQueryInput>,
    },
}

impl AggregateType {
    pub fn empty_value(&self) -> AggregateValue {
        match self {
            // COUNT of empty set is 0
            AggregateType::CountAny { .. } | AggregateType::Count { .. } => AggregateValue::Int(0),
            // All other aggregates (SUM, AVG, MIN, MAX) return NULL for empty sets
            _ => AggregateValue::Null,
        }
    }

    /// Create base Tantivy aggregation from AggregateType (without filter wrapper)
    pub fn to_tantivy_agg(
        &self,
    ) -> Result<tantivy::aggregation::agg_req::Aggregation, Box<dyn std::error::Error>> {
        let unfiltered = if self.filter_expr().is_some() {
            self.convert_filtered_aggregate_to_unfiltered()
        } else {
            self.clone()
        };
        Ok(serde_json::from_value(unfiltered.to_json())?)
    }
    /// Helper function to convert a single filtered aggregate to unfiltered
    fn convert_filtered_aggregate_to_unfiltered(&self) -> Self {
        match self {
            Self::CountAny { .. } => Self::CountAny { filter: None },
            Self::Count { field, missing, .. } => Self::Count {
                field: field.clone(),
                missing: *missing,
                filter: None,
            },
            Self::Sum { field, missing, .. } => Self::Sum {
                field: field.clone(),
                missing: *missing,
                filter: None,
            },
            Self::Avg { field, missing, .. } => Self::Avg {
                field: field.clone(),
                missing: *missing,
                filter: None,
            },
            Self::Min { field, missing, .. } => Self::Min {
                field: field.clone(),
                missing: *missing,
                filter: None,
            },
            Self::Max { field, missing, .. } => Self::Max {
                field: field.clone(),
                missing: *missing,
                filter: None,
            },
        }
    }

    fn format_aggregate(&self) -> String {
        let base = match self {
            AggregateType::CountAny { .. } => "COUNT(*)".to_string(),
            AggregateType::Count { field, .. } => format!("COUNT({field})"),
            AggregateType::Sum { field, .. } => format!("SUM({field})"),
            AggregateType::Avg { field, .. } => format!("AVG({field})"),
            AggregateType::Min { field, .. } => format!("MIN({field})"),
            AggregateType::Max { field, .. } => format!("MAX({field})"),
        };

        match self.filter_expr() {
            Some(filter) => {
                format!("{base} FILTER (WHERE {})", filter.explain_format())
            }
            None => base,
        }
    }

    pub fn format_aggregates(aggregate_types: &[AggregateType], indices: &[usize]) -> String {
        indices
            .iter()
            .map(|&idx| aggregate_types[idx].format_aggregate())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl ExplainFormat for AggregateType {
    fn explain_format(&self) -> String {
        self.format_aggregate()
    }
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

#[derive(Debug, Clone, PartialEq, Default)]
pub enum AggregateValue {
    #[default]
    Null,
    Int(i64),
    Float(f64),
}

/// Enum to specify how numbers should be converted for different aggregate types
#[derive(Debug, Clone, Copy)]
enum NumberConversionMode {
    /// For COUNT: validate integer, check range, always return Int
    ToInt,
    /// For SUM/MIN/MAX: preserve original type (Int or Float)
    Preserve,
    /// For AVG: always convert to Float
    ToFloat,
}

/// Represents an aggregate result that can be either a direct value or wrapped in a "value" object
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum AggregateResult {
    /// Direct numeric value (e.g., `42`)
    DirectValue(serde_json::Number),
    /// Object with "value" field (e.g., `{"value": 42}`)
    ValueObject { value: Option<serde_json::Number> },
    /// Null value
    Null,
}

impl AggregateResult {
    /// Extract the numeric value, returning None if null
    pub fn extract_number(&self) -> Option<&serde_json::Number> {
        match self {
            AggregateResult::DirectValue(num) => Some(num),
            AggregateResult::ValueObject { value } => value.as_ref(),
            AggregateResult::Null => None,
        }
    }
}

// TODO: We should use Tantivy's native aggregate types (CountAggregation, SumAggregation, etc.)
// which implement serde, but the current fork/version produces incorrect JSON structure.
// The Tantivy types serialize to {"field": "name", "missing": null} instead of
// the expected {"aggregation_type": {"field": "name"}} format.
// https://docs.rs/tantivy/latest/tantivy/aggregation/metric/struct.CountAggregation.html
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

        let filter_expr = crate::postgres::customscan::aggregatescan::extract_filter_clause(
            (*aggref).aggfilter,
            bm25_index,
            root,
            heap_rti,
            qual_state,
        );

        if aggfnoid == F_COUNT_ && (*aggref).aggstar {
            return Some(AggregateType::CountAny {
                filter: filter_expr,
            });
        }

        if args.is_empty() {
            return None;
        }

        let first_arg = args.get_ptr(0)?;
        let (field, missing) = parse_aggregate_field(first_arg, heaprelid)?;
        let agg_type = create_aggregate_from_oid(aggfnoid, field, missing, filter_expr)?;

        Some(agg_type)
    }

    /// Get the field name for field-based aggregates (None for COUNT)
    pub fn field_name(&self) -> Option<String> {
        match self {
            AggregateType::CountAny { .. } => None,
            AggregateType::Count { field, .. } => Some(field.clone()),
            AggregateType::Sum { field, .. } => Some(field.clone()),
            AggregateType::Avg { field, .. } => Some(field.clone()),
            AggregateType::Min { field, .. } => Some(field.clone()),
            AggregateType::Max { field, .. } => Some(field.clone()),
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
        }
    }

    /// Check if this aggregate has a filter
    pub fn has_filter(&self) -> bool {
        match self {
            AggregateType::CountAny { filter } => filter.is_some(),
            AggregateType::Count { filter, .. } => filter.is_some(),
            AggregateType::Sum { filter, .. } => filter.is_some(),
            AggregateType::Avg { filter, .. } => filter.is_some(),
            AggregateType::Min { filter, .. } => filter.is_some(),
            AggregateType::Max { filter, .. } => filter.is_some(),
        }
    }

    /// Get the filter expression if present
    pub fn filter_expr(&self) -> &Option<SearchQueryInput> {
        match self {
            AggregateType::CountAny { filter } => filter,
            AggregateType::Count { filter, .. } => filter,
            AggregateType::Sum { filter, .. } => filter,
            AggregateType::Avg { filter, .. } => filter,
            AggregateType::Min { filter, .. } => filter,
            AggregateType::Max { filter, .. } => filter,
        }
    }

    pub fn filter_expr_mut(&mut self) -> &mut Option<SearchQueryInput> {
        match self {
            AggregateType::CountAny { filter } => filter,
            AggregateType::Count { filter, .. } => filter,
            AggregateType::Sum { filter, .. } => filter,
            AggregateType::Avg { filter, .. } => filter,
            AggregateType::Min { filter, .. } => filter,
            AggregateType::Max { filter, .. } => filter,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        let (key, field) = match self {
            AggregateType::CountAny { .. } => ("value_count", "ctid"),
            AggregateType::Count { field, .. } => ("value_count", field.as_str()),
            AggregateType::Sum { field, .. } => ("sum", field.as_str()),
            AggregateType::Avg { field, .. } => ("avg", field.as_str()),
            AggregateType::Min { field, .. } => ("min", field.as_str()),
            AggregateType::Max { field, .. } => ("max", field.as_str()),
        };

        if let Some(missing) = self.missing() {
            serde_json::json!({
                key: {
                    "field": field,
                    "missing": missing,
                }
            })
        } else {
            serde_json::json!({
                key: {
                    "field": field,
                }
            })
        }
    }

    /// Convert AggregateResult to AggregateValue with empty result set handling.
    ///
    /// This method handles the interaction between aggregate types, document counts,
    /// and number processing to ensure correct behavior across all scenarios:
    ///
    /// ## Empty Result Set Handling
    /// When `doc_count` is provided and equals 0, this indicates an empty result set:
    /// - **COUNT**: Returns 0 (counting zero documents is valid and equals 0)
    /// - **SUM**: Returns NULL (sum of empty set is undefined/NULL in SQL standard)
    /// - **AVG/MIN/MAX**: Processed normally (will return NULL from empty AggregateResult)
    ///
    /// ## Document Count Usage by Aggregate Type
    /// - **SUM**: Uses `doc_count` to distinguish truly empty buckets (doc_count=0) from
    ///   buckets containing documents with zero/null field values
    /// - **Other aggregates**: Ignore `doc_count` as they can determine emptiness from
    ///   the aggregate result itself
    ///
    /// ## Number Processing and Type Conversion
    /// Once a valid numeric result is extracted, it undergoes type-specific processing:
    /// - **COUNT**: Validates integer values, checks range, always returns Int
    /// - **SUM/MIN/MAX**: Preserves original type (Int or Float) from the result
    /// - **AVG**: Always converts to Float (division result should be floating-point)
    ///
    /// ## Parameters
    /// - `result`: The raw aggregate result from the search engine (JSON format)
    /// - `doc_count`: Optional document count for the bucket/result set being processed
    ///
    /// ## Returns
    /// `AggregateValue` which can be Int, Float, or Null depending on the aggregate type
    /// and the input data.
    pub fn result_from_aggregate_with_doc_count(
        &self,
        result: AggregateResult,
        doc_count: Option<i64>,
    ) -> AggregateValue {
        // Handle empty result sets (doc_count = 0) based on SQL standard behavior:
        // - COUNT(*) and COUNT(field) return 0 for empty sets
        // - All other aggregates (SUM, AVG, MIN, MAX) return NULL for empty sets
        if doc_count == Some(0) {
            return self.empty_value();
        }

        // Extract the numeric value from the aggregate result
        // This handles both direct values and {"value": ...} wrapped objects
        match result.extract_number() {
            None => AggregateValue::Null,
            Some(num) => {
                // Determine the appropriate number conversion mode based on aggregate type
                let processing_type = match self {
                    AggregateType::CountAny { .. } => NumberConversionMode::ToInt,
                    AggregateType::Count { .. } => NumberConversionMode::ToInt,
                    AggregateType::Sum { .. } => NumberConversionMode::Preserve,
                    AggregateType::Avg { .. } => NumberConversionMode::ToFloat,
                    AggregateType::Min { .. } => NumberConversionMode::Preserve,
                    AggregateType::Max { .. } => NumberConversionMode::Preserve,
                };

                // Process and convert the number according to the aggregate type requirements
                Self::process_number(num, processing_type)
            }
        }
    }

    /// Process a number value based on the aggregate type requirements
    fn process_number(
        num: &serde_json::Number,
        processing_type: NumberConversionMode,
    ) -> AggregateValue {
        match processing_type {
            NumberConversionMode::ToInt => {
                let f64_val = num.as_f64().expect("invalid COUNT result");
                if f64_val.fract() != 0.0 {
                    panic!("COUNT should not have a fractional result");
                }
                if f64_val < (i64::MIN as f64) || (i64::MAX as f64) < f64_val {
                    panic!("COUNT value was out of range");
                }
                AggregateValue::Int(f64_val as i64)
            }
            NumberConversionMode::Preserve => {
                if let Some(int_val) = num.as_i64() {
                    AggregateValue::Int(int_val)
                } else if let Some(f64_val) = num.as_f64() {
                    AggregateValue::Float(f64_val)
                } else {
                    panic!("Numeric result should be a valid number");
                }
            }
            NumberConversionMode::ToFloat => {
                let f64_val = num.as_f64().expect("invalid float result");
                AggregateValue::Float(f64_val)
            }
        }
    }
}

impl AggregateValue {
    pub fn into_datum(self) -> pg_sys::Datum {
        match self {
            AggregateValue::Int(val) => val.into_datum().unwrap(),
            AggregateValue::Float(val) => val.into_datum().unwrap(),
            AggregateValue::Null => pg_sys::Datum::null(),
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, AggregateValue::Null)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TargetListEntry {
    GroupingColumn(usize), // Index into grouping_columns vec
    Aggregate(usize),      // Index into aggregate_types vec
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrivateData {
    pub aggregate_types: Vec<AggregateType>,
    pub indexrelid: pg_sys::Oid,
    pub heap_rti: pg_sys::Index,
    pub query: SearchQueryInput,
    pub grouping_columns: Vec<GroupingColumn>,
    pub orderby_info: Vec<OrderByInfo>,
    pub target_list_mapping: Vec<TargetListEntry>, // Maps target list position to data type
    pub has_order_by: bool,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub maybe_truncated: bool,
    pub filter_groups: Vec<super::FilterGroup>,
}

impl From<*mut pg_sys::List> for PrivateData {
    fn from(list: *mut pg_sys::List) -> Self {
        unsafe {
            let list = PgList::<pg_sys::Node>::from_pg(list);
            let node = list.get_ptr(0).unwrap();
            let content = node
                .as_c_str()
                .unwrap()
                .to_str()
                .expect("string node should be valid utf8");
            serde_json::from_str(content).unwrap()
        }
    }
}

impl From<PrivateData> for *mut pg_sys::List {
    fn from(value: PrivateData) -> Self {
        let content = serde_json::to_string(&value).unwrap();
        unsafe {
            let mut ser = PgList::new();
            ser.push(pg_sys::makeString(content.as_pg_cstr()).cast::<pg_sys::Node>());
            ser.into_pg()
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
unsafe fn parse_coalesce_expression(
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
fn create_aggregate_from_oid(
    aggfnoid: u32,
    field: String,
    missing: Option<f64>,
    filter: Option<SearchQueryInput>,
) -> Option<AggregateType> {
    match aggfnoid {
        F_COUNT_ANY => Some(AggregateType::Count {
            field,
            missing,
            filter,
        }),
        F_AVG_INT8 | F_AVG_INT4 | F_AVG_INT2 | F_AVG_NUMERIC | F_AVG_FLOAT4 | F_AVG_FLOAT8 => {
            Some(AggregateType::Avg {
                field,
                missing,
                filter,
            })
        }
        F_SUM_INT8 | F_SUM_INT4 | F_SUM_INT2 | F_SUM_FLOAT4 | F_SUM_FLOAT8 | F_SUM_NUMERIC => {
            Some(AggregateType::Sum {
                field,
                missing,
                filter,
            })
        }
        F_MAX_INT8 | F_MAX_INT4 | F_MAX_INT2 | F_MAX_FLOAT4 | F_MAX_FLOAT8 | F_MAX_DATE
        | F_MAX_TIME | F_MAX_TIMETZ | F_MAX_TIMESTAMP | F_MAX_TIMESTAMPTZ | F_MAX_NUMERIC => {
            Some(AggregateType::Max {
                field,
                missing,
                filter,
            })
        }
        F_MIN_INT8 | F_MIN_INT4 | F_MIN_INT2 | F_MIN_FLOAT4 | F_MIN_FLOAT8 | F_MIN_DATE
        | F_MIN_TIME | F_MIN_TIMETZ | F_MIN_MONEY | F_MIN_TIMESTAMP | F_MIN_TIMESTAMPTZ
        | F_MIN_NUMERIC => Some(AggregateType::Min {
            field,
            missing,
            filter,
        }),
        _ => {
            pgrx::debug1!("Unknown aggregate function OID: {}", aggfnoid);
            None
        }
    }
}

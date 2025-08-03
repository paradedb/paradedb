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

use crate::api::AsCStr;
use crate::postgres::customscan::builders::custom_path::OrderByInfo;
use crate::query::SearchQueryInput;
use pgrx::pg_sys::AsPgCStr;
use pgrx::prelude::*;
use pgrx::PgList;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AggregateType {
    Count,
    Sum { field: String },
    Avg { field: String },
    Min { field: String },
    Max { field: String },
    Stats { field: String },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum AggregateValue {
    #[default]
    Null,
    Int(i64),
    Float(f64),
}

// TODO: We should likely directly using tantivy's aggregate types, which all derive serde.
// https://docs.rs/tantivy/latest/tantivy/aggregation/metric/struct.CountAggregation.html
impl AggregateType {
    /// Get the field name for field-based aggregates (None for COUNT)
    pub fn field_name(&self) -> Option<String> {
        match self {
            AggregateType::Count => None,
            AggregateType::Sum { field } => Some(field.clone()),
            AggregateType::Avg { field } => Some(field.clone()),
            AggregateType::Min { field } => Some(field.clone()),
            AggregateType::Max { field } => Some(field.clone()),
            AggregateType::Stats { field } => Some(field.clone()),
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        match self {
            AggregateType::Count => {
                serde_json::json!({
                    "value_count": {
                        "field": "ctid"
                    }
                })
            }
            AggregateType::Sum { field } => {
                serde_json::json!({
                    "sum": {
                        "field": field
                    }
                })
            }
            AggregateType::Avg { field } => {
                serde_json::json!({
                    "avg": {
                        "field": field
                    }
                })
            }
            AggregateType::Min { field } => {
                serde_json::json!({
                    "min": {
                        "field": field
                    }
                })
            }
            AggregateType::Max { field } => {
                serde_json::json!({
                    "max": {
                        "field": field
                    }
                })
            }
            AggregateType::Stats { field } => {
                serde_json::json!({
                    "stats": {
                        "field": field
                    }
                })
            }
        }
    }

    #[allow(unreachable_patterns)]
    pub fn to_json_for_group(&self, idx: usize) -> Option<(String, serde_json::Value)> {
        match self {
            AggregateType::Count => None, // 'terms' bucket already has a 'doc_count'
            _ => Some((format!("agg_{idx}"), self.to_json())),
        }
    }

    pub fn result_from_json(&self, result: &serde_json::Value) -> AggregateValue {
        match self {
            AggregateType::Count => match Self::extract_value(result, "COUNT") {
                None => AggregateValue::Null,
                Some(value) => {
                    if let Some(num) = value.as_number() {
                        Self::process_count_number(num)
                    } else {
                        panic!("COUNT result value should be a number or null, got: {value:?}");
                    }
                }
            },
            AggregateType::Sum { .. } => match Self::extract_value(result, "SUM") {
                None => AggregateValue::Null,
                Some(value) => {
                    if let Some(num) = value.as_number() {
                        Self::process_numeric_number(num)
                    } else {
                        panic!("SUM result value should be a number or null, got: {value:?}");
                    }
                }
            },
            AggregateType::Avg { .. } => match Self::extract_value(result, "AVG") {
                None => AggregateValue::Null,
                Some(value) => {
                    if let Some(num) = value.as_number() {
                        Self::process_float_number(num)
                    } else {
                        panic!("AVG result value should be a number or null, got: {value:?}");
                    }
                }
            },
            AggregateType::Min { .. } | AggregateType::Max { .. } => {
                let agg_name = if matches!(self, AggregateType::Min { .. }) {
                    "MIN"
                } else {
                    "MAX"
                };
                match Self::extract_value(result, agg_name) {
                    None => AggregateValue::Null,
                    Some(value) => {
                        if let Some(num) = value.as_number() {
                            Self::process_numeric_number(num)
                        } else {
                            panic!("{agg_name} result value should be a number or null, got: {value:?}");
                        }
                    }
                }
            }
            AggregateType::Stats { .. } => {
                if result.is_null() {
                    AggregateValue::Null
                } else if let Some(obj) = result.as_object() {
                    // Check if the entire object is a {"value": null} wrapper
                    if let Some(value) = obj.get("value") {
                        if value.is_null() {
                            AggregateValue::Null
                        } else if let Some(value_obj) = value.as_object() {
                            Self::process_stats_object(value_obj)
                        } else {
                            panic!("STATS result value should be an object, got: {value:?}");
                        }
                    } else {
                        // Normal STATS object with count directly
                        Self::process_stats_object(obj)
                    }
                } else {
                    panic!("STATS result should be an object or null, got: {result:?}");
                }
            }
        }
    }

    /// Extract the actual value from a JSON result, handling both direct values and {"value": ...} wrapper objects
    fn extract_value<'a>(
        result: &'a serde_json::Value,
        aggregate_name: &str,
    ) -> Option<&'a serde_json::Value> {
        if result.is_null() {
            None
        } else if result.is_number() {
            Some(result)
        } else if let Some(obj) = result.as_object() {
            if let Some(value) = obj.get("value") {
                if value.is_null() {
                    None
                } else {
                    Some(value)
                }
            } else {
                panic!("{aggregate_name} result object missing 'value' field: {result:?}");
            }
        } else {
            panic!("{aggregate_name} result should be a number, null, or object with value field, got: {result:?}");
        }
    }

    /// Process a number value for COUNT aggregates (integer validation and range checking)
    fn process_count_number(num: &serde_json::Number) -> AggregateValue {
        let f64_val = num.as_f64().expect("invalid COUNT result");
        if f64_val.fract() != 0.0 {
            panic!("COUNT should not have a fractional result");
        }
        if f64_val < (i64::MIN as f64) || (i64::MAX as f64) < f64_val {
            panic!("COUNT value was out of range");
        }
        AggregateValue::Int(f64_val as i64)
    }

    /// Process a number value for numeric aggregates (can be int or float)
    fn process_numeric_number(num: &serde_json::Number) -> AggregateValue {
        if let Some(int_val) = num.as_i64() {
            AggregateValue::Int(int_val)
        } else if let Some(f64_val) = num.as_f64() {
            AggregateValue::Float(f64_val)
        } else {
            panic!("Numeric result should be a valid number");
        }
    }

    /// Process a number value for float aggregates (always convert to f64)
    fn process_float_number(num: &serde_json::Number) -> AggregateValue {
        let f64_val = num.as_f64().expect("invalid float result");
        AggregateValue::Float(f64_val)
    }

    /// Process a STATS result object to extract the count
    fn process_stats_object(obj: &serde_json::Map<String, serde_json::Value>) -> AggregateValue {
        let count = obj
            .get("count")
            .and_then(|v| v.as_number())
            .and_then(|n| n.as_i64())
            .expect("STATS result should contain count");
        AggregateValue::Int(count)
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
pub struct GroupingColumn {
    pub field_name: String,
    pub attno: pg_sys::AttrNumber,
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
    pub order_by_info: Vec<OrderByInfo>,
    pub target_list_mapping: Vec<TargetListEntry>, // Maps target list position to data type
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

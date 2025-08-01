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

#[derive(Debug, Clone, PartialEq)]
pub enum AggregateValue {
    Int(i64),
    Float(f64),
    Null,
}

impl Default for AggregateValue {
    fn default() -> Self {
        AggregateValue::Null
    }
}

impl AggregateValue {
    pub fn to_datum(self) -> Option<pgrx::pg_sys::Datum> {
        match self {
            AggregateValue::Int(i) => i.into_datum(),
            AggregateValue::Float(f) => f.into_datum(),
            AggregateValue::Null => None,
        }
    }
}

// TODO: We should likely directly using tantivy's aggregate types, which all derive serde.
// https://docs.rs/tantivy/latest/tantivy/aggregation/metric/struct.CountAggregation.html
impl AggregateType {
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
            AggregateType::Count => {
                let num = result.as_number().expect("COUNT result should be a number");
                let f64_val = num.as_f64().expect("invalid aggregate result size");

                if f64_val.fract() != 0.0 {
                    panic!("COUNT should not have a fractional result");
                }
                if f64_val < (i64::MIN as f64) || (i64::MAX as f64) < f64_val {
                    panic!("COUNT value was out of range");
                }
                AggregateValue::Int(f64_val as i64)
            }
            AggregateType::Sum { .. } => {
                let num = result.as_number().expect("SUM result should be a number");
                if let Some(int_val) = num.as_i64() {
                    AggregateValue::Int(int_val)
                } else if let Some(f64_val) = num.as_f64() {
                    AggregateValue::Float(f64_val)
                } else {
                    panic!("SUM result should be a valid number");
                }
            }
            AggregateType::Avg { .. } => {
                let f64_val = result
                    .as_number()
                    .and_then(|n| n.as_f64())
                    .expect("AVG result should be a float");
                AggregateValue::Float(f64_val)
            }
            AggregateType::Min { .. } | AggregateType::Max { .. } => {
                // Handle null values for MIN/MAX when there are no rows
                if result.is_null() {
                    AggregateValue::Null
                } else {
                    let num = result
                        .as_number()
                        .expect("MIN/MAX result should be a number or null");
                    if let Some(int_val) = num.as_i64() {
                        AggregateValue::Int(int_val)
                    } else if let Some(f64_val) = num.as_f64() {
                        AggregateValue::Float(f64_val)
                    } else {
                        panic!("MIN/MAX result should be a valid number");
                    }
                }
            }
            AggregateType::Stats { .. } => {
                // Stats returns an object with multiple values, for now we'll return the count
                // In the future we might want to return a more complex structure
                let count = result
                    .get("count")
                    .and_then(|v| v.as_number())
                    .and_then(|n| n.as_i64())
                    .expect("STATS result should contain count");
                AggregateValue::Int(count)
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

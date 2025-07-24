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
use crate::query::SearchQueryInput;
use pgrx::pg_sys::AsPgCStr;
use pgrx::prelude::*;
use pgrx::PgList;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AggregateType {
    Count,
}

// TODO: We should likely directly using tantivy's aggregate types, which all derive serde.
// https://docs.rs/tantivy/latest/tantivy/aggregation/metric/struct.CountAggregation.html
impl AggregateType {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::from_str(r#"{"value_count": {"field": "ctid"}}"#).unwrap()
    }

    pub fn to_json_for_group(&self, idx: usize) -> (String, serde_json::Value) {
        match self {
            AggregateType::Count => (
                format!("agg_{idx}"),
                serde_json::from_str(r#"{"value_count": {"field": "ctid"}}"#).unwrap(),
            ),
        }
    }

    pub fn result_from_json(&self, result: &serde_json::Number) -> i64 {
        let f64_val = result.as_f64().expect("invalid aggregate result size");

        if f64_val.fract() != 0.0 {
            panic!("COUNT should not have a fractional result");
        }
        if f64_val < (i64::MIN as f64) || (i64::MAX as f64) < f64_val {
            panic!("COUNT value was out of range");
        }
        f64_val as i64
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GroupingColumn {
    pub field_name: String,
    pub attno: pg_sys::AttrNumber,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum OrderByColumn {
    GroupingColumn {
        field_name: String,
        attno: pg_sys::AttrNumber,
        direction: SortDirection,
    },
    AggregateColumn {
        aggregate_index: usize, // Index into the aggregate_types vec
        direction: SortDirection,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrivateData {
    pub aggregate_types: Vec<AggregateType>,
    pub indexrelid: pg_sys::Oid,
    pub heap_rti: pg_sys::Index,
    pub query: SearchQueryInput,
    pub grouping_columns: Vec<GroupingColumn>,
    pub order_by_columns: Vec<OrderByColumn>,
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

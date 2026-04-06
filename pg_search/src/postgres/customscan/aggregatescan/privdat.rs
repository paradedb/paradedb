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

use crate::api::AsCStr;
use crate::customscan::aggregatescan::build::AggregateCSClause;
use crate::postgres::customscan::aggregatescan::join_targetlist::JoinAggregateTargetList;
use crate::postgres::customscan::joinscan::build::RelNode;
use pgrx::pg_sys::AsPgCStr;
use pgrx::prelude::*;
use pgrx::PgList;

/// TopK sort+limit info pushed into the DataFusion aggregate plan.
/// Allows DataFusion to handle ORDER BY aggregate + LIMIT internally.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataFusionTopK {
    /// Index into `JoinAggregateTargetList.aggregates` for the sort target.
    pub sort_agg_idx: usize,
    pub direction: crate::api::SortDirection,
    /// Maximum number of rows to return (LIMIT + OFFSET).
    pub k: usize,
}

/// Private data serialized between planning and execution for AggregateScan.
///
/// The `Tantivy` variant is the existing single-table path. The `DataFusion`
/// variant is the new join aggregate path (and single-table fallback when
/// Tantivy bucket limits are exceeded).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PrivateData {
    /// Existing single-table Tantivy aggregation path.
    Tantivy {
        indexrelid: pg_sys::Oid,
        heap_rti: pg_sys::Index,
        aggregate_clause: Box<AggregateCSClause>,
    },

    /// New DataFusion-backed aggregation path (for JOINs).
    DataFusion {
        /// The join tree (Scan/Join/Filter nodes).
        plan: RelNode,
        /// The aggregate target list (GROUP BY columns + aggregate functions).
        targetlist: JoinAggregateTargetList,
        /// Optional TopK sort+limit pushed down from Postgres.
        topk: Option<DataFusionTopK>,
    },
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

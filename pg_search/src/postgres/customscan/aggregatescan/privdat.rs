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

/// A post-join filter clause that couldn't be pushed to individual table scans.
/// Serialized at plan time, translated to DataFusion `Expr` at execution time.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PostJoinFilter {
    /// Deparsed SQL expression (e.g., "(p.price > 100)").
    /// Used as a fallback representation — the execution path re-resolves
    /// column references via the source/alias mapping.
    pub deparsed: String,
    /// Column references: (plan_position, field_name) pairs found in the expression.
    pub columns: Vec<(usize, String)>,
}

/// TopK sort+limit info pushed into the DataFusion aggregate plan.
/// Allows DataFusion to handle ORDER BY aggregate + LIMIT internally.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataFusionTopK {
    /// Index into `JoinAggregateTargetList.aggregates` for the sort target.
    pub sort_agg_idx: usize,
    /// Sort direction: true = DESC, false = ASC.
    pub descending: bool,
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
        /// Post-join filter clauses from joinrestrictinfo that aren't equi-keys.
        post_join_filters: Vec<PostJoinFilter>,
    },
}

impl PrivateData {
    /// Helper to access Tantivy-specific fields. Panics if called on DataFusion variant.
    pub fn as_tantivy(&self) -> (&pg_sys::Oid, &pg_sys::Index, &AggregateCSClause) {
        match self {
            PrivateData::Tantivy {
                indexrelid,
                heap_rti,
                aggregate_clause,
            } => (indexrelid, heap_rti, aggregate_clause),
            PrivateData::DataFusion { .. } => {
                panic!("called as_tantivy() on DataFusion PrivateData")
            }
        }
    }

    /// Returns true if this is the Tantivy backend path.
    pub fn is_tantivy(&self) -> bool {
        matches!(self, PrivateData::Tantivy { .. })
    }

    /// Returns true if this is the DataFusion backend path.
    pub fn is_datafusion(&self) -> bool {
        matches!(self, PrivateData::DataFusion { .. })
    }
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

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
use crate::postgres::customscan::joinscan::build::{
    JoinLevelSearchPredicate, MultiTablePredicateInfo, RelNode, RelationAlias,
};
use pgrx::pg_sys::AsPgCStr;
use pgrx::prelude::*;
use pgrx::PgList;

/// Serializable boolean expression IR used for both HAVING clauses and
/// per-aggregate FILTER clauses.
///
/// HAVING uses `AggRef` and `GroupRef` (post-aggregate references).
/// FILTER uses `ColumnRef` (pre-aggregate row-level references).
/// Both share the same operator, literal, and boolean combinators.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FilterExpr {
    /// Reference to an aggregate result by index (HAVING context).
    AggRef(usize),
    /// Reference to a GROUP BY column by field name (HAVING context).
    GroupRef(String),
    /// Reference to a pre-aggregate table column (FILTER context).
    ColumnRef {
        rti: pgrx::pg_sys::Index,
        field_name: String,
    },
    LitInt(i64),
    LitFloat(f64),
    LitBool(bool),
    LitString(String),
    BinOp {
        left: Box<FilterExpr>,
        op: CompareOp,
        right: Box<FilterExpr>,
    },
    And(Vec<FilterExpr>),
    Or(Vec<FilterExpr>),
    Not(Box<FilterExpr>),
    IsNull(Box<FilterExpr>),
    IsNotNull(Box<FilterExpr>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CompareOp {
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
}

/// Identifies whether a TopK sort targets an aggregate result or a GROUP BY column.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TopKSortTarget {
    /// Sort by an aggregate result (e.g., ORDER BY COUNT(*)).
    /// The value is the index into `JoinAggregateTargetList.aggregates`.
    Aggregate(usize),
    /// Sort by a GROUP BY column (e.g., ORDER BY category).
    /// The value is the index into `JoinAggregateTargetList.group_columns`.
    GroupColumn(usize),
}

impl TopKSortTarget {
    /// Resolve the DataFusion column name for the sort target.
    ///
    /// Aggregate targets use the `agg_{idx}` alias assigned during aggregate
    /// expression building. Group column targets resolve to `{table_alias}.{field}`
    /// via the join plan's source metadata.
    pub fn resolve_sort_col_name(
        &self,
        targetlist: &JoinAggregateTargetList,
        plan: &RelNode,
    ) -> String {
        match self {
            TopKSortTarget::Aggregate(idx) => format!("agg_{}", idx),
            TopKSortTarget::GroupColumn(idx) => {
                let gc = &targetlist.group_columns[*idx];
                let source = plan.source_for_rti_in_subtree(gc.rti);
                let alias = if let Some(src) = source {
                    RelationAlias::new(src.scan_info.alias.as_deref()).execution(src.plan_position)
                } else {
                    format!("unknown_rti_{}", gc.rti)
                };
                format!("{}.{}", alias, gc.field_name)
            }
        }
    }
}

/// TopK sort+limit info pushed into the DataFusion aggregate plan.
///
/// When the sort target is a GROUP BY column or MIN/MAX aggregate, DataFusion's
/// built-in `TopKAggregation` optimizer rule can push the limit into
/// `AggregateExec`, enabling early termination (group-key ordering) or
/// PriorityMap-based pruning (MIN/MAX ordering) during aggregation.
///
/// For COUNT/SUM/AVG ordering, DataFusion's `SortExec(fetch=K)` uses a bounded
/// TopK heap — still more efficient than letting Postgres sort above us.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataFusionTopK {
    /// What the ORDER BY targets.
    pub sort_target: TopKSortTarget,
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
        /// Cross-table search predicates extracted from WHERE quals.
        /// These are @@@ predicates that reference multiple tables and cannot
        /// be pushed to individual table scans.
        #[serde(default)]
        join_level_predicates: Vec<JoinLevelSearchPredicate>,
        /// Non-@@@ cross-table predicates (like `b.id > 5`) that reference
        /// fast fields and can be evaluated by DataFusion at join time.
        #[serde(default)]
        multi_table_predicates: Vec<MultiTablePredicateInfo>,
        /// Number of raw PG Expr pointers stored in custom_private after the
        /// serialized PrivateData. These are moved to custom_exprs during
        /// plan_custom_path so setrefs transforms their Var nodes.
        #[serde(default)]
        multi_table_clause_count: usize,
        /// HAVING clause filter applied after aggregation.
        #[serde(default)]
        having_filter: Option<FilterExpr>,
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

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
use crate::postgres::customscan::aggregatescan::datafusion_exec::is_lateral_unnest;
use crate::postgres::customscan::aggregatescan::join_targetlist::{
    GroupingTransform, JoinAggregateTargetList,
};
use crate::postgres::customscan::datafusion::timestamp_to_date::TIMESTAMP_TO_DATE_UDF_NAME;
use crate::postgres::customscan::joinscan::build::{
    JoinLevelSearchPredicate, RelNode, RelationAlias,
};
use crate::query::SearchQueryInput;
use datafusion::common::Column;
use datafusion::common::tree_node::{Transformed, TreeNode, TreeNodeIterator, TreeNodeRecursion};
use pgrx::PgList;
use pgrx::pg_sys::AsPgCStr;
use pgrx::prelude::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ColumnRef {
    pub plan_position: usize,
    pub rti: pgrx::pg_sys::Index,
    pub attno: pgrx::pg_sys::AttrNumber,
    pub field_name: String,
}

/// Serializable boolean expression IR used for both HAVING clauses and
/// per-aggregate FILTER clauses.
///
/// HAVING uses `AggRef` and `GroupRef` (post-aggregate references).
/// FILTER uses `ColumnRef` (pre-aggregate row-level references).
/// Both share the same operator, literal, and boolean combinators.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum FilterExpr {
    /// Reference to an aggregate result by index (HAVING context).
    AggRef(usize),
    /// Reference to a GROUP BY column by its index in
    /// `JoinAggregateTargetList.group_columns` (HAVING context).
    GroupRef(usize),
    /// Reference to a pre-aggregate table column (FILTER context).
    /// Execution identity is `plan_position`, resolved against the
    /// `RelNode` tree at construction; rti/attno are kept for diagnostics
    /// and for fast-field projection in `populate_required_fields`.
    ColumnRef(ColumnRef),
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
    SearchTag {
        plan_position: usize,
        predicate: Box<JoinLevelSearchPredicate>,
    },
}

impl TreeNode for FilterExpr {
    fn apply_children<'n, F: FnMut(&'n Self) -> datafusion::common::Result<TreeNodeRecursion>>(
        &'n self,
        mut f: F,
    ) -> datafusion::common::Result<TreeNodeRecursion> {
        match self {
            Self::BinOp { left, right, .. } => f(left)?.visit_sibling(|| f(right)),
            Self::And(children) | Self::Or(children) => children.iter().apply_until_stop(f),
            Self::Not(inner) | Self::IsNull(inner) | Self::IsNotNull(inner) => f(inner),
            Self::AggRef(_)
            | Self::GroupRef(_)
            | Self::ColumnRef(_)
            | Self::LitInt(_)
            | Self::LitFloat(_)
            | Self::LitBool(_)
            | Self::LitString(_)
            | Self::SearchTag { .. } => Ok(TreeNodeRecursion::Continue),
        }
    }

    fn map_children<F: FnMut(Self) -> datafusion::common::Result<Transformed<Self>>>(
        self,
        mut f: F,
    ) -> datafusion::common::Result<Transformed<Self>> {
        Ok(match self {
            Self::BinOp { left, op, right } => f(*left)?
                .update_data(|left| (left, *right))
                .transform_sibling(
                    |(left, right)| Ok(f(right)?.update_data(|right| (left, right))),
                )?
                .update_data(|(left, right)| Self::BinOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                }),
            Self::And(children) => children
                .into_iter()
                .map_until_stop_and_collect(f)?
                .update_data(Self::And),
            Self::Or(children) => children
                .into_iter()
                .map_until_stop_and_collect(f)?
                .update_data(Self::Or),
            Self::Not(inner) => f(*inner)?.update_data(|inner| Self::Not(Box::new(inner))),
            Self::IsNull(inner) => f(*inner)?.update_data(|inner| Self::IsNull(Box::new(inner))),
            Self::IsNotNull(inner) => {
                f(*inner)?.update_data(|inner| Self::IsNotNull(Box::new(inner)))
            }
            leaf @ (Self::AggRef(_)
            | Self::GroupRef(_)
            | Self::ColumnRef(_)
            | Self::LitInt(_)
            | Self::LitFloat(_)
            | Self::LitBool(_)
            | Self::LitString(_)
            | Self::SearchTag { .. }) => Transformed::no(leaf),
        })
    }
}

impl FilterExpr {
    /// True if any [`FilterExpr::AggRef`] in this tree satisfies `pred`.
    pub fn any_agg_ref(&self, pred: &impl Fn(usize) -> bool) -> bool {
        self.exists(|node| Ok(matches!(node, FilterExpr::AggRef(idx) if pred(*idx))))
            .unwrap_or(false)
    }

    /// Collects every FILTER `(plan_position, predicate)` pair in this tree.
    pub fn collect_search_predicates<'a>(
        &'a self,
        out: &mut Vec<(usize, &'a JoinLevelSearchPredicate)>,
    ) {
        let _ = self.apply(|node| {
            if let FilterExpr::SearchTag {
                plan_position,
                predicate,
            } = node
            {
                out.push((*plan_position, predicate.as_ref()));
            }
            Ok(TreeNodeRecursion::Continue)
        });
    }

    /// Visit the query of every [`FilterExpr::SearchTag`] in this tree.
    ///
    /// The scan carries its own copy of each query as a tagged query, and the
    /// tag column is found by comparing the two. Whatever solves Params and
    /// Postgres expressions in the scan's copy has to solve this one too, or
    /// the comparison fails.
    ///
    /// Hand-written because [`TreeNode`] has no in-place `&mut` visit, as for
    /// `RelNode::visit_queries_mut`.
    pub fn visit_queries_mut(&mut self, f: &mut impl FnMut(&mut SearchQueryInput)) {
        match self {
            FilterExpr::SearchTag { predicate, .. } => f(&mut predicate.query),
            FilterExpr::BinOp { left, right, .. } => {
                left.visit_queries_mut(f);
                right.visit_queries_mut(f);
            }
            FilterExpr::And(children) | FilterExpr::Or(children) => {
                children.iter_mut().for_each(|c| c.visit_queries_mut(f));
            }
            FilterExpr::Not(inner) | FilterExpr::IsNull(inner) | FilterExpr::IsNotNull(inner) => {
                inner.visit_queries_mut(f);
            }
            FilterExpr::AggRef(_)
            | FilterExpr::GroupRef(_)
            | FilterExpr::ColumnRef { .. }
            | FilterExpr::LitInt(_)
            | FilterExpr::LitFloat(_)
            | FilterExpr::LitBool(_)
            | FilterExpr::LitString(_) => {}
        }
    }

    /// Read-only counterpart of [`FilterExpr::visit_queries_mut`].
    pub fn visit_queries(&self, f: &mut impl FnMut(&SearchQueryInput)) {
        let _ = self.apply(|node| {
            if let FilterExpr::SearchTag { predicate, .. } = node {
                f(&predicate.query);
            }
            Ok(TreeNodeRecursion::Continue)
        });
    }

    /// Collect all `ColumnRef` column references from an [`FilterExpr`] tree.
    /// Tag columns are synthesized by the scan, not read from fast fields, so
    /// a [`FilterExpr::SearchTag`] contributes none.
    pub fn collect_column_refs(&self) -> Vec<ColumnRef> {
        let mut refs = Vec::new();
        let _ = self.apply(|node| {
            if let FilterExpr::ColumnRef(column_ref) = node {
                refs.push(column_ref.clone());
            }
            Ok(TreeNodeRecursion::Continue)
        });
        refs
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
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
    /// Resolve the DataFusion column reference for the sort target.
    ///
    /// Aggregate and row value targets use the names the plan gives them. Group
    /// targets use either the qualified source column or the unqualified UDF
    /// output name, without parsing SQL identifiers.
    pub fn resolve_sort_column(
        &self,
        targetlist: &JoinAggregateTargetList,
        plan: &RelNode,
    ) -> Column {
        match self {
            TopKSortTarget::Aggregate(idx) => {
                Column::new_unqualified(targetlist.aggregate_name(*idx))
            }
            TopKSortTarget::GroupColumn(idx) => {
                let gc = &targetlist.group_columns[*idx];
                if gc.row_value {
                    return Column::new_unqualified(targetlist.row_value_name(*idx));
                }
                let source = plan.source_at_plan_position(gc.plan_position);
                let alias = if let Some(src) = source {
                    RelationAlias::new(src.scan_info.alias.as_deref()).execution(src.plan_position)
                } else {
                    format!("unknown_plan_position_{}", gc.plan_position)
                };

                match gc.transform {
                    GroupingTransform::Identity => {
                        // A LATERAL unnest column reaches the aggregate under its
                        // projected name, with no source qualifier to bind to.
                        if is_lateral_unnest(plan, gc.plan_position, &gc.field_name) {
                            Column::new_unqualified(format!("{alias}_{}", gc.field_name))
                        } else {
                            Column::new(Some(alias), &gc.field_name)
                        }
                    }
                    GroupingTransform::TimestampToDate => Column::new_unqualified(format!(
                        "{TIMESTAMP_TO_DATE_UDF_NAME}({alias}.{})",
                        gc.field_name
                    )),
                }
            }
        }
    }
}

/// TopK sort+limit info pushed into the DataFusion aggregate plan.
///
/// When the sort target is a GROUP BY column or MIN/MAX aggregate, DataFusion's
/// built-in `TopKAggregation` optimizer rule can push the limit into
/// `AggregateExec`, enabling early termination (group-key ordering) or
/// PriorityMap-based pruning (MIN/MAX ordering) during aggregation. That rule
/// takes exactly one sort key over exactly one group column; every other
/// shape, multi-key sorts included, runs as `SortExec` with `fetch=K`.
///
/// For COUNT/SUM/AVG ordering, DataFusion's `SortExec(fetch=K)` uses a bounded
/// TopK heap — still more efficient than letting Postgres sort above us.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataFusionTopK {
    /// What the ORDER BY targets.
    pub sorts: Vec<(TopKSortTarget, crate::api::SortDirection)>,
    /// Maximum number of rows to return after OFFSET, if a LIMIT is present.
    pub limit: Option<crate::postgres::customscan::parameterized_value::ParameterizedValue<i64>>,
    /// OFFSET count, if an OFFSET is present.
    pub offset: Option<crate::postgres::customscan::parameterized_value::ParameterizedValue<i64>>,
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
        /// Number of raw PG Expr pointers stored in custom_private after the
        /// serialized PrivateData. These are moved to custom_exprs during
        /// plan_custom_path so setrefs transforms their Var nodes.
        #[serde(default)]
        multi_table_clause_count: usize,
        /// HAVING clause filter applied after aggregation.
        #[serde(default)]
        having_filter: Option<FilterExpr>,
        /// PostgreSQL's statement-wide `PlannerGlobal.parallelModeOK` decision.
        parallel_mode_ok: bool,
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

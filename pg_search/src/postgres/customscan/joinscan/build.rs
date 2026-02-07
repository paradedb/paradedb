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

//! Data structures for JoinScan planning and serialization.
//!
//! These structures are serialized to JSON and stored in CustomScan's custom_private
//! field, then deserialized during execution.
//!
//! Note: ORDER BY score pushdown is implemented via pathkeys on CustomPath at planning
//! time. See `extract_score_pathkey()` in mod.rs.

use crate::api::OrderByInfo;
use crate::postgres::utils::RawPtr;
use crate::query::SearchQueryInput;
pub use crate::scan::ScanInfo;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents the join type for serialization.
///
/// Note: Currently only Inner join is supported, but other variants are
/// defined for future extensibility and to match PostgreSQL's JoinType enum.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub enum JoinType {
    #[default]
    Inner,
    Left,
    Full,
    Right,
    Semi,
    Anti,
}

impl fmt::Display for JoinType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            JoinType::Inner => "Inner",
            JoinType::Left => "Left",
            JoinType::Full => "Full",
            JoinType::Right => "Right",
            JoinType::Semi => "Semi",
            JoinType::Anti => "Anti",
        };
        write!(f, "{}", s)
    }
}

impl From<pg_sys::JoinType::Type> for JoinType {
    fn from(jt: pg_sys::JoinType::Type) -> Self {
        match jt {
            pg_sys::JoinType::JOIN_INNER => JoinType::Inner,
            pg_sys::JoinType::JOIN_LEFT => JoinType::Left,
            pg_sys::JoinType::JOIN_FULL => JoinType::Full,
            pg_sys::JoinType::JOIN_RIGHT => JoinType::Right,
            pg_sys::JoinType::JOIN_SEMI => JoinType::Semi,
            pg_sys::JoinType::JOIN_ANTI => JoinType::Anti,
            other => panic!("JoinScan: unsupported join type {:?}", other),
        }
    }
}

/// Represents a join key column pair with type information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinKeyPair {
    /// RTI of the outer (left) relation.
    pub outer_rti: pg_sys::Index,
    /// Attribute number from the outer relation.
    pub outer_attno: pg_sys::AttrNumber,
    /// RTI of the inner (right) relation.
    pub inner_rti: pg_sys::Index,
    /// Attribute number from the inner relation.
    pub inner_attno: pg_sys::AttrNumber,
    /// PostgreSQL type OID of the join key.
    pub type_oid: pg_sys::Oid,
    /// Type length from pg_type.typlen (-1 for varlena, -2 for cstring).
    pub typlen: i16,
    /// Whether type is pass-by-value.
    pub typbyval: bool,
}

/// A join-level search predicate - a search query that applies to a specific relation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinLevelSearchPredicate {
    /// The RTI of the relation this predicate applies to (used for column resolution).
    pub rti: pg_sys::Index,
    /// The OID of the BM25 index to use.
    pub indexrelid: pg_sys::Oid,
    /// The OID of the heap relation for visibility checks.
    pub heaprelid: pg_sys::Oid,
    /// The search query.
    pub query: SearchQueryInput,
    /// Raw pointer to the original PostgreSQL expression (for lazy deparse).
    /// Only valid within the same query execution.
    pub expr_ptr: RawPtr<pg_sys::Node>,
    /// Raw pointer to PlannerInfo (for lazy deparse context).
    pub planner_info_ptr: RawPtr<pg_sys::PlannerInfo>,
}

/// Projection information for a child join.
/// Maps an output attribute (by index in the vector) to the source column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildProjection {
    pub rti: pg_sys::Index,
    pub attno: pg_sys::AttrNumber,
    #[serde(default)]
    pub is_score: bool,
}

/// Represents the source of data for a join side.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JoinSource {
    pub scan_info: ScanInfo,
}

impl JoinSource {
    pub fn new(scan_info: ScanInfo) -> Self {
        Self { scan_info }
    }

    pub fn alias(&self) -> Option<String> {
        self.scan_info.alias.clone()
    }

    /// Returns the alias to be used for this source in the DataFusion plan.
    ///
    /// If the source has an explicit alias (from SQL), it is used.
    /// Otherwise, a synthetic alias `source_{index}` is generated based on its position.
    pub fn execution_alias(&self, index: usize) -> String {
        self.alias().unwrap_or_else(|| format!("source_{}", index))
    }

    /// Check if this source contains the given RTI.
    pub fn contains_rti(&self, rti: pg_sys::Index) -> bool {
        self.scan_info.heap_rti == Some(rti)
    }

    /// Check if this source has a search predicate.
    pub fn has_search_predicate(&self) -> bool {
        self.scan_info.has_search_predicate
    }

    /// Check if this source has a BM25 index.
    pub fn has_bm25_index(&self) -> bool {
        self.scan_info.has_bm25_index()
    }

    /// Map a base relation variable to its position in this source's output.
    /// Since we flattened the join, this is just identity if RTI matches.
    pub fn map_var(
        &self,
        varno: pg_sys::Index,
        attno: pg_sys::AttrNumber,
    ) -> Option<pg_sys::AttrNumber> {
        if self.scan_info.heap_rti == Some(varno) {
            Some(attno)
        } else {
            None
        }
    }

    /// Resolve an attribute number to its DataFusion column name.
    pub(super) fn column_name(&self, attno: pg_sys::AttrNumber) -> Option<String> {
        self.scan_info
            .fields
            .iter()
            .find(|f| f.attno == attno)
            .and_then(|f| {
                if matches!(
                    f.field,
                    crate::index::fast_fields_helper::WhichFastField::Score
                ) {
                    None
                } else {
                    Some(f.field.name())
                }
            })
    }

    /// Recursively collect all base relations in this source.
    pub fn collect_base_relations(&self, acc: &mut Vec<ScanInfo>) {
        acc.push(self.scan_info.clone());
    }

    /// Recursively find the ordering RTI of this source.
    pub fn ordering_rti(&self) -> Option<pg_sys::Index> {
        self.scan_info.heap_rti
    }
}

/// A multi-table predicate - a condition that references columns from multiple
/// tables and must be evaluated at join time (not pushed to a single table's index).
///
/// These are cross-relation conditions like `a.price > b.min_value` that reference
/// columns from both sides of the join.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiTablePredicateInfo {
    /// Human-readable description for EXPLAIN output.
    pub description: String,
    /// Index of this condition in the restrictlist (for runtime extraction).
    pub restrictinfo_index: usize,
}

/// A boolean expression tree for join-level conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinLevelExpr {
    /// Leaf: single-table predicate, check if ctid is in the Tantivy result set.
    SingleTablePredicate {
        /// Index of the source in `JoinCSClause.sources` this predicate references.
        source_idx: usize,
        /// Index into the `join_level_predicates` vector.
        predicate_idx: usize,
    },
    /// Leaf: multi-table predicate, evaluate at runtime against the joined row pair.
    MultiTablePredicate {
        /// Index into the `multi_table_predicates` vector.
        predicate_idx: usize,
    },
    /// Logical AND of child expressions.
    And(Vec<JoinLevelExpr>),
    /// Logical OR of child expressions.
    Or(Vec<JoinLevelExpr>),
    /// Logical NOT of a child expression.
    Not(Box<JoinLevelExpr>),
}

/// The clause information for a Join Custom Scan.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct JoinCSClause {
    /// Information about the sources involved in the join (N-way).
    pub sources: Vec<JoinSource>,
    /// The type of join (Currently implicitly inner for all).
    pub join_type: JoinType,
    /// The join key column pairs (for equi-joins).
    pub join_keys: Vec<JoinKeyPair>,
    /// The LIMIT value from the query, if any.
    pub limit: Option<usize>,
    /// Join-level search predicates (Tantivy queries to execute).
    pub join_level_predicates: Vec<JoinLevelSearchPredicate>,
    /// Heap conditions (PostgreSQL expressions referencing both sides).
    pub multi_table_predicates: Vec<MultiTablePredicateInfo>,
    /// The boolean expression tree that combines predicates and heap conditions.
    pub join_level_expr: Option<JoinLevelExpr>,
    /// ORDER BY clause to be applied to the DataFusion plan.
    pub order_by: Vec<OrderByInfo>,
    /// Projection of output columns for this join.
    pub output_projection: Option<Vec<ChildProjection>>,
}

impl JoinCSClause {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_source(mut self, source: JoinSource) -> Self {
        self.sources.push(source);
        self
    }

    pub fn with_join_type(mut self, join_type: JoinType) -> Self {
        self.join_type = join_type;
        self
    }

    pub fn with_limit(mut self, limit: Option<usize>) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_order_by(mut self, order_by: Vec<OrderByInfo>) -> Self {
        self.order_by = order_by;
        self
    }

    pub fn with_output_projection(mut self, projection: Vec<ChildProjection>) -> Self {
        self.output_projection = Some(projection);
        self
    }

    /// Add a join-level predicate and return its index.
    pub fn add_join_level_predicate(
        &mut self,
        rti: pg_sys::Index,
        indexrelid: pg_sys::Oid,
        heaprelid: pg_sys::Oid,
        query: SearchQueryInput,
        expr_ptr: *mut pg_sys::Node,
        planner_info_ptr: *mut pg_sys::PlannerInfo,
    ) -> usize {
        let idx = self.join_level_predicates.len();
        self.join_level_predicates.push(JoinLevelSearchPredicate {
            rti,
            indexrelid,
            heaprelid,
            query,
            expr_ptr: RawPtr::new(expr_ptr),
            planner_info_ptr: RawPtr::new(planner_info_ptr),
        });
        idx
    }

    /// Add a heap condition and return its index.
    pub fn add_multi_table_predicate(
        &mut self,
        description: String,
        restrictinfo_index: usize,
    ) -> usize {
        let idx = self.multi_table_predicates.len();
        self.multi_table_predicates.push(MultiTablePredicateInfo {
            description,
            restrictinfo_index,
        });
        idx
    }

    /// Returns true if there are heap conditions to evaluate.
    pub fn has_multi_table_predicates(&self) -> bool {
        !self.multi_table_predicates.is_empty()
    }

    /// Set the join-level expression tree.
    pub fn with_join_level_expr(mut self, expr: JoinLevelExpr) -> Self {
        self.join_level_expr = Some(expr);
        self
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_join_key(
        mut self,
        outer_rti: pg_sys::Index,
        outer_attno: pg_sys::AttrNumber,
        inner_rti: pg_sys::Index,
        inner_attno: pg_sys::AttrNumber,
        type_oid: pg_sys::Oid,
        typlen: i16,
        typbyval: bool,
    ) -> Self {
        self.join_keys.push(JoinKeyPair {
            outer_rti,
            outer_attno,
            inner_rti,
            inner_attno,
            type_oid,
            typlen,
            typbyval,
        });
        self
    }

    /// Returns the index of the ordering side (the source with a search predicate).
    /// If multiple have it, returns the first one.
    pub fn ordering_side_index(&self) -> Option<usize> {
        self.sources.iter().position(|s| s.has_search_predicate())
    }

    /// Get the ordering side source (side with search predicate).
    pub fn ordering_side(&self) -> Option<&JoinSource> {
        self.ordering_side_index().map(|i| &self.sources[i])
    }

    /// Recursively collect all base relations in this join tree.
    pub fn collect_base_relations(&self, acc: &mut Vec<ScanInfo>) {
        for source in &self.sources {
            source.collect_base_relations(acc);
        }
    }
}

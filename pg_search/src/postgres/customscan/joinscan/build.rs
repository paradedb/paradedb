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
use crate::query::SearchQueryInput;
pub use crate::scan::ScanInfo;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};

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
    /// Attribute number from the outer relation.
    pub outer_attno: pg_sys::AttrNumber,
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
    /// The OID of the BM25 index to use.
    pub indexrelid: pg_sys::Oid,
    /// The OID of the heap relation for visibility checks.
    pub heaprelid: pg_sys::Oid,
    /// The search query.
    pub query: SearchQueryInput,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinSource {
    Base(ScanInfo),
    /// A child join, including the clause, projection mapping, and an optional alias.
    Join(JoinCSClause, Vec<ChildProjection>, Option<String>),
}

impl Default for JoinSource {
    fn default() -> Self {
        JoinSource::Base(ScanInfo::default())
    }
}

impl JoinSource {
    pub fn as_base(&self) -> Option<&ScanInfo> {
        match self {
            JoinSource::Base(info) => Some(info),
            JoinSource::Join(_, _, _) => None,
        }
    }

    pub fn as_base_mut(&mut self) -> Option<&mut ScanInfo> {
        match self {
            JoinSource::Base(info) => Some(info),
            JoinSource::Join(_, _, _) => None,
        }
    }

    /// Check if this source contains the given RTI (recursively).
    pub fn contains_rti(&self, rti: pg_sys::Index) -> bool {
        match self {
            JoinSource::Base(info) => info.heap_rti == Some(rti),
            JoinSource::Join(clause, _, _) => clause.sources.iter().any(|s| s.contains_rti(rti)),
        }
    }

    /// Check if this source has a search predicate (recursively).
    pub fn has_search_predicate(&self) -> bool {
        match self {
            JoinSource::Base(info) => info.has_search_predicate,
            JoinSource::Join(clause, _, _) => {
                clause.sources.iter().any(|s| s.has_search_predicate())
            }
        }
    }

    /// Check if this source has a BM25 index (recursively).
    pub fn has_bm25_index(&self) -> bool {
        match self {
            JoinSource::Base(info) => info.has_bm25_index(),
            JoinSource::Join(clause, _, _) => clause.sources.iter().any(|s| s.has_bm25_index()),
        }
    }

    pub fn alias(&self) -> Option<String> {
        match self {
            JoinSource::Base(info) => info.alias.clone(),
            JoinSource::Join(_, _, alias) => alias.clone(),
        }
    }

    /// Map a base relation variable to its position in this source's output.
    /// Returns 1-based attribute number if found.
    pub fn map_var(
        &self,
        varno: pg_sys::Index,
        attno: pg_sys::AttrNumber,
    ) -> Option<pg_sys::AttrNumber> {
        match self {
            JoinSource::Base(info) => {
                if info.heap_rti == Some(varno) {
                    Some(attno)
                } else {
                    None
                }
            }
            JoinSource::Join(_, projection, _) => projection
                .iter()
                .position(|p| p.rti == varno && p.attno == attno)
                .map(|pos| (pos + 1) as pg_sys::AttrNumber),
        }
    }

    /// Recursively collect all base relations in this source.
    pub fn collect_base_relations(&self, acc: &mut Vec<ScanInfo>) {
        match self {
            JoinSource::Base(info) => acc.push(info.clone()),
            JoinSource::Join(clause, _, _) => {
                for source in &clause.sources {
                    source.collect_base_relations(acc);
                }
            }
        }
    }

    /// Recursively find the ordering RTI of this source.
    pub fn ordering_rti(&self) -> Option<pg_sys::Index> {
        match self {
            JoinSource::Base(info) => info.heap_rti,
            JoinSource::Join(clause, _, _) => clause.ordering_side().ordering_rti(),
        }
    }

    /// Resolve an attribute number to its DataFusion column name.
    ///
    /// For Base sources, this returns the field name (excluding Score).
    /// For Join sources, this returns "col_N" corresponding to the projection index.
    pub(super) fn column_name(&self, attno: pg_sys::AttrNumber) -> Option<String> {
        match self {
            JoinSource::Base(info) => info.fields.iter().find(|f| f.attno == attno).and_then(|f| {
                if matches!(
                    f.field,
                    crate::index::fast_fields_helper::WhichFastField::Score
                ) {
                    None
                } else {
                    Some(f.field.name())
                }
            }),
            JoinSource::Join(_, _, _) => {
                // For nested joins, we use "col_N" as assigned in build_clause_df
                Some(format!("col_{}", attno))
            }
        }
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
///
/// This preserves the full AND/OR/NOT structure of complex join-level predicates,
/// allowing correct evaluation of expressions like:
/// `(search_pred OR multi_table_pred)` or `(tbl1_search AND NOT tbl2_search)`
///
/// The tree can reference two types of leaf conditions:
/// - `SingleTablePredicate`: A condition on one table (evaluated via Tantivy ctid set)
/// - `MultiTablePredicate`: A condition spanning multiple tables (evaluated at runtime)
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
    /// Information about the sources involved in the join.
    /// For binary joins, index 0 is left/outer, index 1 is right/inner.
    pub sources: Vec<JoinSource>,
    /// The type of join.
    pub join_type: JoinType,
    /// The join key column pairs (for equi-joins).
    pub join_keys: Vec<JoinKeyPair>,
    /// The LIMIT value from the query, if any.
    pub limit: Option<usize>,
    /// Join-level search predicates (Tantivy queries to execute).
    /// Each predicate is referenced by index from `join_level_expr` via `Predicate` variant.
    pub join_level_predicates: Vec<JoinLevelSearchPredicate>,
    /// Heap conditions (PostgreSQL expressions referencing both sides).
    /// Each condition is referenced by index from `join_level_expr` via `HeapCondition` variant.
    pub multi_table_predicates: Vec<MultiTablePredicateInfo>,
    /// The boolean expression tree that combines predicates and heap conditions.
    /// When Some, this expression must be evaluated for each row-pair.
    pub join_level_expr: Option<JoinLevelExpr>,
    /// ORDER BY clause to be applied to the DataFusion plan.
    pub order_by: Vec<OrderByInfo>,
    /// Projection of output columns for this join (if it is a nested join).
    /// Maps output column position (index) to (rti, attno) in the join's context.
    /// If None, no specific projection is enforced (used for top-level).
    pub output_projection: Option<Vec<ChildProjection>>,
}

impl JoinCSClause {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_outer_side(mut self, side: JoinSource) -> Self {
        if self.sources.is_empty() {
            self.sources.push(side);
        } else {
            self.sources[0] = side;
        }
        self
    }

    pub fn with_inner_side(mut self, side: JoinSource) -> Self {
        if self.sources.len() < 2 {
            self.sources.resize_with(2, JoinSource::default);
        }
        self.sources[1] = side;
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
        indexrelid: pg_sys::Oid,
        heaprelid: pg_sys::Oid,
        query: SearchQueryInput,
    ) -> usize {
        let idx = self.join_level_predicates.len();
        self.join_level_predicates.push(JoinLevelSearchPredicate {
            indexrelid,
            heaprelid,
            query,
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

    pub fn add_join_key(
        mut self,
        outer_attno: pg_sys::AttrNumber,
        inner_attno: pg_sys::AttrNumber,
        type_oid: pg_sys::Oid,
        typlen: i16,
        typbyval: bool,
    ) -> Self {
        self.join_keys.push(JoinKeyPair {
            outer_attno,
            inner_attno,
            type_oid,
            typlen,
            typbyval,
        });
        self
    }

    /// Returns which side (outer=true, inner=false) is the ordering side (has search predicate).
    /// Prefers outer if both have predicates.
    pub fn ordering_side_is_outer(&self) -> bool {
        // If outer has predicate, use it as ordering side
        if !self.sources.is_empty() && self.sources[0].has_search_predicate() {
            return true;
        }
        // Otherwise, inner must have it
        false
    }

    /// Get the ordering side source (side with search predicate).
    pub fn ordering_side(&self) -> &JoinSource {
        if self.ordering_side_is_outer() {
            &self.sources[0]
        } else {
            &self.sources[1]
        }
    }

    /// Recursively collect all base relations in this join tree.
    pub fn collect_base_relations(&self, acc: &mut Vec<ScanInfo>) {
        for source in &self.sources {
            source.collect_base_relations(acc);
        }
    }
}

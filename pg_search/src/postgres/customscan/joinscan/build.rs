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

use crate::postgres::options::SortByField;
use crate::query::SearchQueryInput;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};

/// Information about one side of the join (outer or inner).
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct JoinSideInfo {
    /// The range table index for this side's base relation.
    pub heap_rti: Option<pg_sys::Index>,
    /// The OID of the heap table.
    pub heaprelid: Option<pg_sys::Oid>,
    /// The OID of the BM25 index (if this side has one).
    pub indexrelid: Option<pg_sys::Oid>,
    /// The search query for this side (extracted from WHERE clause predicates).
    /// None if this side has no BM25 index or no search predicate.
    pub query: Option<SearchQueryInput>,
    /// Whether this side has a search predicate (uses @@@ operator).
    pub has_search_predicate: bool,
    /// The alias used in the query (e.g., "p" for "products p"), if any.
    pub alias: Option<String>,
    /// Whether scores are needed for this side's results.
    /// True when ORDER BY paradedb.score() is present for this side.
    /// Used to optimize FastField executor (skip score computation when not needed).
    pub score_needed: bool,
    /// The sort order of the BM25 index segments, if the index was created with `sort_by`.
    ///
    /// When this is `Some`, the index segments are physically sorted by this field.
    /// This enables DataFusion-based execution to:
    /// - Declare sort ordering via `EquivalenceProperties`
    /// - Use `SortPreservingMergeExec` to merge sorted segment streams
    /// - Enable sort-merge joins when both sides are sorted on join keys
    pub sort_order: Option<SortByField>,
}

impl JoinSideInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_heap_rti(mut self, rti: pg_sys::Index) -> Self {
        self.heap_rti = Some(rti);
        self
    }

    pub fn with_heaprelid(mut self, oid: pg_sys::Oid) -> Self {
        self.heaprelid = Some(oid);
        self
    }

    pub fn with_indexrelid(mut self, oid: pg_sys::Oid) -> Self {
        self.indexrelid = Some(oid);
        self
    }

    /// Returns true if this side has a BM25 index.
    pub fn has_bm25_index(&self) -> bool {
        self.indexrelid.is_some()
    }

    pub fn with_query(mut self, query: SearchQueryInput) -> Self {
        self.query = Some(query);
        self.has_search_predicate = true;
        self
    }

    pub fn with_alias(mut self, alias: String) -> Self {
        self.alias = Some(alias);
        self
    }

    pub fn with_score_needed(mut self, needed: bool) -> Self {
        self.score_needed = needed;
        self
    }

    /// Sets the sort order from the BM25 index metadata.
    ///
    /// This is populated at planning time by reading from `SearchIndexReader::sort_order()`.
    /// When set, DataFusion-based execution can leverage the physical sort order for:
    /// - Declaring output ordering via `EquivalenceProperties`
    /// - Using `SortPreservingMergeExec` for merging sorted segment streams
    /// - Enabling sort-merge joins when beneficial
    pub fn with_sort_order(mut self, sort_order: Option<SortByField>) -> Self {
        self.sort_order = sort_order;
        self
    }

    /// Returns true if this side's index produces sorted output.
    pub fn is_sorted(&self) -> bool {
        self.sort_order.is_some()
    }
}

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

/// Preferred join algorithm hint from planner.
///
/// Allows the planner to suggest which join algorithm the executor should use
/// based on statistics available at planning time.
///
/// Note: Currently only Auto is used since JoinScan requires LIMIT.
/// PreferHash is reserved for future non-LIMIT join support.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub enum JoinAlgorithmHint {
    /// Let executor decide based on runtime conditions (default)
    #[default]
    Auto,
    /// Prefer hash join (good for larger build sides with good selectivity)
    PreferHash,
}

/// Execution hints passed from planner to executor.
///
/// These hints allow the planner to pass optimization information to the executor
/// based on statistics available at planning time. The executor can use these
/// hints to make better decisions about algorithm selection, memory allocation,
/// and batch sizing.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionHints {
    /// Preferred join algorithm based on planner's analysis
    pub algorithm: JoinAlgorithmHint,

    /// Estimated number of rows in build side (from planner statistics).
    /// Used to pre-size hash table and decide algorithm.
    pub estimated_build_rows: Option<f64>,

    /// Estimated memory needed for hash table (bytes).
    /// Helps executor decide if hash join is feasible before building.
    pub estimated_hash_memory: Option<usize>,
}

impl ExecutionHints {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_algorithm(mut self, algo: JoinAlgorithmHint) -> Self {
        self.algorithm = algo;
        self
    }

    pub fn with_estimated_build_rows(mut self, rows: f64) -> Self {
        self.estimated_build_rows = Some(rows);
        self
    }

    pub fn with_estimated_hash_memory(mut self, bytes: usize) -> Self {
        self.estimated_hash_memory = Some(bytes);
        self
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

/// Which side of the join a predicate references (serializable version).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JoinSide {
    Outer,
    Inner,
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
        /// Which side of the join this predicate references.
        side: JoinSide,
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
    /// Information about the outer (left) side of the join.
    pub outer_side: JoinSideInfo,
    /// Information about the inner (right) side of the join.
    pub inner_side: JoinSideInfo,
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
    /// Execution hints from planner to guide runtime decisions.
    pub hints: ExecutionHints,
}

impl JoinCSClause {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_outer_side(mut self, side: JoinSideInfo) -> Self {
        self.outer_side = side;
        self
    }

    pub fn with_inner_side(mut self, side: JoinSideInfo) -> Self {
        self.inner_side = side;
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

    /// Set execution hints from planner.
    pub fn with_hints(mut self, hints: ExecutionHints) -> Self {
        self.hints = hints;
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

    /// Returns true if at least one side has a BM25 index with a search predicate.
    pub fn has_driving_side(&self) -> bool {
        (self.outer_side.has_bm25_index() && self.outer_side.has_search_predicate)
            || (self.inner_side.has_bm25_index() && self.inner_side.has_search_predicate)
    }

    /// Returns true if this is a valid join for M1 (Single Feature with LIMIT).
    pub fn is_valid_for_single_feature(&self) -> bool {
        self.limit.is_some() && self.has_driving_side()
    }

    /// Returns true if this join has INNER join type (the only type we support in M1).
    pub fn is_inner_join(&self) -> bool {
        self.join_type == JoinType::Inner
    }

    /// Returns which side (outer=true, inner=false) is the driving side (has search predicate).
    /// Prefers outer if both have predicates.
    pub fn driving_side_is_outer(&self) -> bool {
        // If outer has predicate, use it as driving side
        if self.outer_side.has_search_predicate {
            return true;
        }
        // Otherwise, inner must have it
        false
    }

    /// Get the driving side info (side with search predicate).
    pub fn driving_side(&self) -> &JoinSideInfo {
        if self.driving_side_is_outer() {
            &self.outer_side
        } else {
            &self.inner_side
        }
    }

    /// Get the build side info (side without search predicate, used for hash table).
    pub fn build_side(&self) -> &JoinSideInfo {
        if self.driving_side_is_outer() {
            &self.inner_side
        } else {
            &self.outer_side
        }
    }
}

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
//! # Future Enhancements (TODO)
//!
//! - Add score_needed field to JoinSideInfo to track whether scores are needed per-side
//! - Consider adding execution hints (preferred join algorithm, memory hints)
//!
//! Note: ORDER BY score pushdown is implemented via pathkeys on CustomPath at planning
//! time. See `extract_score_pathkey()` in mod.rs.

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
    /// Whether this side has a valid BM25 index.
    pub has_bm25_index: bool,
    /// Whether this side has a search predicate (uses @@@ operator).
    pub has_search_predicate: bool,
    /// The alias used in the query (e.g., "p" for "products p"), if any.
    pub alias: Option<String>,
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
        self.has_bm25_index = true;
        self
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
}

/// Represents the join type for serialization.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub enum SerializableJoinType {
    #[default]
    Inner,
    Left,
    Full,
    Right,
    Semi,
    Anti,
}

impl From<pg_sys::JoinType::Type> for SerializableJoinType {
    fn from(jt: pg_sys::JoinType::Type) -> Self {
        match jt {
            pg_sys::JoinType::JOIN_INNER => SerializableJoinType::Inner,
            pg_sys::JoinType::JOIN_LEFT => SerializableJoinType::Left,
            pg_sys::JoinType::JOIN_FULL => SerializableJoinType::Full,
            pg_sys::JoinType::JOIN_RIGHT => SerializableJoinType::Right,
            pg_sys::JoinType::JOIN_SEMI => SerializableJoinType::Semi,
            pg_sys::JoinType::JOIN_ANTI => SerializableJoinType::Anti,
            _ => SerializableJoinType::Inner, // fallback
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

/// Which side of the join a predicate references (serializable version).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SerializableJoinSide {
    Outer,
    Inner,
}

/// A serializable boolean expression tree for join-level predicates.
///
/// This preserves the full AND/OR/NOT structure of complex join-level predicates,
/// allowing correct evaluation of expressions like:
/// `(tbl1_cond1 OR (tbl1_cond2 OR (tbl2_cond AND NOT tbl1_cond3)))`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializableJoinLevelExpr {
    /// Leaf: check if the row's ctid is in the predicate's result set.
    Predicate {
        /// Which side of the join this predicate references.
        side: SerializableJoinSide,
        /// Index into the `join_level_predicates` vector.
        predicate_idx: usize,
    },
    /// Logical AND of child expressions.
    And(Vec<SerializableJoinLevelExpr>),
    /// Logical OR of child expressions.
    Or(Vec<SerializableJoinLevelExpr>),
    /// Logical NOT of a child expression.
    Not(Box<SerializableJoinLevelExpr>),
}

/// The clause information for a Join Custom Scan.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct JoinCSClause {
    /// Information about the outer (left) side of the join.
    pub outer_side: JoinSideInfo,
    /// Information about the inner (right) side of the join.
    pub inner_side: JoinSideInfo,
    /// The type of join.
    pub join_type: SerializableJoinType,
    /// The join key column pairs (for equi-joins).
    pub join_keys: Vec<JoinKeyPair>,
    /// The LIMIT value from the query, if any.
    pub limit: Option<usize>,
    /// Whether there are other (non-equijoin) conditions that need to be evaluated.
    /// These conditions are stored in custom_exprs during planning.
    pub has_other_conditions: bool,
    /// Join-level search predicates (the actual Tantivy queries to execute).
    /// Each predicate is referenced by index from the `join_level_expr` tree.
    pub join_level_predicates: Vec<JoinLevelSearchPredicate>,
    /// The boolean expression tree that combines join-level predicates.
    /// When Some, this expression must be evaluated for each row-pair.
    pub join_level_expr: Option<SerializableJoinLevelExpr>,
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

    pub fn with_join_type(mut self, join_type: SerializableJoinType) -> Self {
        self.join_type = join_type;
        self
    }

    pub fn with_limit(mut self, limit: Option<usize>) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_has_other_conditions(mut self, has_other: bool) -> Self {
        self.has_other_conditions = has_other;
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

    /// Set the join-level expression tree.
    pub fn with_join_level_expr(mut self, expr: SerializableJoinLevelExpr) -> Self {
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

    /// Returns true if at least one side has a BM25 index with a search predicate.
    pub fn has_driving_side(&self) -> bool {
        (self.outer_side.has_bm25_index && self.outer_side.has_search_predicate)
            || (self.inner_side.has_bm25_index && self.inner_side.has_search_predicate)
    }

    /// Returns true if this is a valid join for M1 (Single Feature with LIMIT).
    pub fn is_valid_for_single_feature(&self) -> bool {
        self.limit.is_some() && self.has_driving_side()
    }

    /// Returns true if this join has INNER join type (the only type we support in M1).
    pub fn is_inner_join(&self) -> bool {
        self.join_type == SerializableJoinType::Inner
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

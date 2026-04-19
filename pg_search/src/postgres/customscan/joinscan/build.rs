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
//! See the [JoinScan README](README.md) for the full architecture overview.
//!
//! These structures are serialized to JSON and stored in CustomScan's custom_private
//! field, then deserialized during execution.
//!
//! Note: ORDER BY score pushdown is implemented via pathkeys on CustomPath at planning
//! time. See `pathkey_uses_scores_from_source()` in planning.rs.

use crate::api::OrderByInfo;
use crate::postgres::utils::ExprContextGuard;
use crate::query::SearchQueryInput;
pub use crate::scan::ScanInfo;
use anyhow::anyhow;
use pgrx::{pg_sys, PgList};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ptr::NonNull;

/// DataFusion-facing relation alias helper.
///
/// JoinScan may reference the same PostgreSQL relation multiple times in one
/// DataFusion plan, so we centralize execution-time alias generation here.
#[derive(Debug, Clone, Copy)]
pub struct RelationAlias<'a> {
    name: Option<&'a str>,
}

impl<'a> RelationAlias<'a> {
    pub fn new(name: Option<&'a str>) -> Self {
        Self { name }
    }

    /// For EXPLAIN output, don't suffix the relation to make it more readable
    pub fn display(&self, index: usize) -> String {
        self.name
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("source_{}", index))
    }

    /// For DataFusion execution, suffix the relation to make it unique
    pub fn execution(&self, index: usize) -> String {
        match self.name {
            Some(alias) => format!("{alias}_{index}"),
            None => format!("source_{}", index),
        }
    }

    /// Returns a stable context label for planner warnings.
    ///
    /// Context labels should not depend on per-plan source ordering, otherwise
    /// failed exploratory paths cannot be cleared when a successful path is found.
    pub fn warning_context(&self, heaprelid: pg_sys::Oid) -> String {
        self.name
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("relid_{}", heaprelid))
    }
}

/// DataFusion-facing synthetic CTID column name helper.
///
/// JoinScan exposes per-source CTID columns into DataFusion so rows can be
/// materialized back to PostgreSQL tuples after query execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CtidColumn {
    plan_position: usize,
}

impl CtidColumn {
    const PREFIX: &'static str = "ctid_";

    pub fn new(plan_position: usize) -> Self {
        Self { plan_position }
    }

    pub fn plan_position(self) -> usize {
        self.plan_position
    }
}

impl fmt::Display for CtidColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, self.plan_position)
    }
}

impl TryFrom<&str> for CtidColumn {
    type Error = ();

    fn try_from(col_name: &str) -> Result<Self, Self::Error> {
        let plan_position = col_name
            .strip_prefix(Self::PREFIX)
            .ok_or(())?
            .parse::<usize>()
            .map_err(|_| ())?;
        Ok(Self::new(plan_position))
    }
}

/// DataFusion/planning identity for the PostgreSQL planner root that produced a source.
///
/// We carry this through JoinScan planning so repeated RTIs from different
/// subquery roots can be disambiguated before building the DataFusion plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlannerRootId(usize);

impl From<usize> for PlannerRootId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<*mut pg_sys::PlannerInfo> for PlannerRootId {
    fn from(value: *mut pg_sys::PlannerInfo) -> Self {
        Self(value as usize)
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
    /// LeftMark join: returns all left rows with an additional boolean "mark" column
    /// indicating whether a right-side match exists. Used to decorrelate
    /// `EXISTS` / `IN` subqueries inside disjunctive predicates such as
    /// `col IS NULL OR col IN (SELECT ...)`.
    LeftMark,
    /// RightMark join: mirror of LeftMark — returns all right rows with a
    /// boolean "mark" column indicating whether a left-side match exists.
    RightMark,
    RightSemi,
    RightAnti,
    UniqueOuter,
    UniqueInner,
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
            JoinType::LeftMark => "LeftMark",
            JoinType::RightMark => "RightMark",
            JoinType::RightSemi => "RightSemi",
            JoinType::RightAnti => "RightAnti",
            JoinType::UniqueOuter => "UniqueOuter",
            JoinType::UniqueInner => "UniqueInner",
        };
        write!(f, "{}", s)
    }
}

impl TryFrom<pg_sys::JoinType::Type> for JoinType {
    type Error = anyhow::Error;

    fn try_from(jt: pg_sys::JoinType::Type) -> Result<Self, Self::Error> {
        match jt {
            pg_sys::JoinType::JOIN_INNER => Ok(JoinType::Inner),
            pg_sys::JoinType::JOIN_LEFT => Ok(JoinType::Left),
            pg_sys::JoinType::JOIN_FULL => Ok(JoinType::Full),
            pg_sys::JoinType::JOIN_RIGHT => Ok(JoinType::Right),
            pg_sys::JoinType::JOIN_SEMI => Ok(JoinType::Semi),
            pg_sys::JoinType::JOIN_ANTI => Ok(JoinType::Anti),
            #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
            pg_sys::JoinType::JOIN_RIGHT_ANTI => Ok(JoinType::RightAnti),
            #[cfg(feature = "pg18")]
            pg_sys::JoinType::JOIN_RIGHT_SEMI => Ok(JoinType::RightSemi),
            pg_sys::JoinType::JOIN_UNIQUE_OUTER => Ok(JoinType::UniqueOuter),
            pg_sys::JoinType::JOIN_UNIQUE_INNER => Ok(JoinType::UniqueInner),
            other => Err(anyhow::anyhow!("JoinScan: unknown join type {}", other)),
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

impl JoinKeyPair {
    pub fn resolve_against<'a>(
        &self,
        left: &'a RelNode,
        right: &'a RelNode,
    ) -> Option<(JoinKeySide<'a>, JoinKeySide<'a>)> {
        if let (Some(left_source), Some(right_source)) = (
            left.source_for_rti_in_subtree(self.outer_rti),
            right.source_for_rti_in_subtree(self.inner_rti),
        ) {
            Some((
                (left_source, self.outer_attno),
                (right_source, self.inner_attno),
            ))
        } else if let (Some(left_source), Some(right_source)) = (
            left.source_for_rti_in_subtree(self.inner_rti),
            right.source_for_rti_in_subtree(self.outer_rti),
        ) {
            Some((
                (left_source, self.inner_attno),
                (right_source, self.outer_attno),
            ))
        } else {
            None
        }
    }
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
    /// Human-readable representation of the original PostgreSQL expression (for EXPLAIN output).
    /// Eagerly computed during planning via `deparse_expr`.
    pub display_string: String,
}

pub use crate::postgres::customscan::expr_eval::InputVarInfo;

/// Projection information for a child join.
/// Maps an output attribute (by index in the vector) to the source column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChildProjection {
    /// Simple column reference
    Column {
        rti: pg_sys::Index,
        attno: pg_sys::AttrNumber,
    },
    /// Score function
    Score { rti: pg_sys::Index },
    /// An indexed expression handled by existing fast field machinery
    IndexedExpression {
        rti: pg_sys::Index,
        attno: pg_sys::AttrNumber,
    },
    /// Arbitrary PG expression evaluated via PgExprUdf
    Expression {
        rti: pg_sys::Index,
        pg_expr_string: String,
        input_vars: Vec<InputVarInfo>,
        result_type_oid: pg_sys::Oid,
    },
}

use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::limit_offset::LimitOffset;
use crate::postgres::customscan::range_table::{get_plain_relation_relid, get_rte};
use crate::postgres::options::SortByField;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::scan::info::{FieldInfo, RowEstimate};

/// Source information collected during planning.
///
/// This represents a relation before all required JoinScan invariants are verified.
/// Optional fields are progressively filled as planning discovers index metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinSourceCandidate {
    pub root_id: PlannerRootId,
    pub heap_rti: pg_sys::Index,
    pub heaprelid: Option<pg_sys::Oid>,
    pub indexrelid: Option<pg_sys::Oid>,
    pub query: Option<SearchQueryInput>,
    pub has_search_predicate: bool,
    pub alias: Option<String>,
    pub score_needed: bool,
    pub fields: Vec<FieldInfo>,
    pub sort_order: Option<SortByField>,
    pub estimate: Option<RowEstimate>,
    pub segment_count: Option<usize>,
    pub estimated_rows_per_worker: Option<u64>,
}

impl JoinSourceCandidate {
    pub fn new(root_id: PlannerRootId, heap_rti: pg_sys::Index) -> Self {
        Self {
            root_id,
            heap_rti,
            heaprelid: None,
            indexrelid: None,
            query: None,
            has_search_predicate: false,
            alias: None,
            score_needed: false,
            fields: Vec::new(),
            sort_order: None,
            estimate: None,
            segment_count: None,
            estimated_rows_per_worker: None,
        }
    }

    pub fn with_heaprelid(mut self, oid: pg_sys::Oid) -> Self {
        self.heaprelid = Some(oid);
        self
    }

    pub fn with_alias(mut self, alias: String) -> Self {
        self.alias = Some(alias);
        self
    }

    pub fn with_indexrelid(mut self, oid: pg_sys::Oid) -> Self {
        self.indexrelid = Some(oid);
        self
    }

    pub fn with_query(mut self, query: SearchQueryInput) -> Self {
        self.query = Some(query);
        self
    }

    pub fn with_search_predicate(mut self) -> Self {
        self.has_search_predicate = true;
        self
    }

    pub fn with_sort_order(mut self, sort_order: Option<SortByField>) -> Self {
        self.sort_order = sort_order;
        self
    }

    pub fn has_bm25_index(&self) -> bool {
        self.indexrelid.is_some()
    }

    pub fn has_search_predicate(&self) -> bool {
        self.has_search_predicate
    }

    pub fn alias(&self) -> Option<String> {
        self.alias.clone()
    }

    /// Check if this source contains the given RTI.
    pub fn contains_rti(&self, rti: pg_sys::Index) -> bool {
        self.heap_rti == rti
    }

    /// Calculate and store the estimated number of rows matching the query.
    ///
    /// This uses `MvccSatisfies::LargestSegment` to efficiently estimate the count
    /// without opening all segments.
    ///
    /// If the source does not have a BM25 index, this is a no-op.
    /// If estimation fails (e.g. IO error), this method will panic.
    pub fn estimate_rows(&mut self) {
        if !self.has_bm25_index() {
            return;
        }

        let indexrelid = self.indexrelid.expect("Index relid missing");
        let heaprelid = self.heaprelid.expect("Heap relid missing");

        let index_rel = PgSearchRelation::open(indexrelid);
        let heap_rel = PgSearchRelation::open(heaprelid);
        let mut query = self.query.clone().unwrap_or(SearchQueryInput::All);
        let row_estimate = RowEstimate::from_reltuples(heap_rel.reltuples().map(|r| r as f64));
        let has_pg_exprs = query.has_postgres_expressions();
        if has_pg_exprs {
            let reader = SearchIndexReader::empty(&index_rel, MvccSatisfies::LargestSegment)
                .expect("Failed to open index reader for estimation");
            self.segment_count = Some(reader.total_segment_count());
            self.estimate = Some(RowEstimate::Known(reader.total_docs()));
            return;
        }

        // `expr_context` only lives until the end of this function,
        // which is fine because it is only used to get estimates
        let expr_context = ExprContextGuard::new();
        let needs_tokenizer_manager = query.needs_tokenizer();
        let reader = SearchIndexReader::open_with_context(
            &index_rel,
            query,
            false,
            MvccSatisfies::LargestSegment,
            NonNull::new(expr_context.as_ptr()),
            None,
            needs_tokenizer_manager,
        )
        .expect("Failed to open index reader for estimation");

        self.segment_count = Some(reader.total_segment_count());

        let (estimate, _) = reader.estimate_docs(row_estimate);
        self.estimate = Some(RowEstimate::Known(estimate as u64));
    }
}

/// Represents the validated source of data for a join side used during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinSource {
    /// Stable zero-based position of this source in `RelNode::sources()` order.
    ///
    /// This is the DataFusion-facing identity for a join source. It is assigned
    /// once when the JoinScan plan is built and then used anywhere a source
    /// must stay distinguishable inside the plan:
    /// - synthetic ctid columns are named `ctid_<plan_position>`
    /// - deferred-visibility state is tracked per source
    /// - SearchPredicateUDF canonical segment IDs are keyed by it
    ///
    /// `indexrelid` is not sufficient here because the same underlying index can
    /// appear more than once in a single JoinScan plan (for example a self-join,
    /// or the same source copied into partitioning/non-partitioning roles in
    /// parallel execution). `plan_position` is the per-source identity that
    /// keeps those otherwise-identical sources distinct inside the plan.
    pub plan_position: usize,
    /// Identity of the PlannerInfo root this source originated from.
    pub root_id: Option<PlannerRootId>,
    pub scan_info: ScanInfo,
}

impl JoinSource {
    /// Check if this source contains the given RTI.
    pub fn contains_rti(&self, rti: pg_sys::Index) -> bool {
        self.scan_info.heap_rti == rti
    }

    /// Check if this source has a search predicate.
    pub fn has_search_predicate(&self) -> bool {
        self.scan_info.has_search_predicate
    }

    /// Returns true when this source can reference the provided attribute number.
    pub fn has_attno(&self, attno: pg_sys::AttrNumber) -> bool {
        if attno == 0 {
            return true;
        }
        if attno == pg_sys::SelfItemPointerAttributeNumber as pg_sys::AttrNumber {
            return true;
        }
        if attno < 0 {
            return false;
        }
        let heaprel = PgSearchRelation::open(self.scan_info.heaprelid);
        let tupdesc = heaprel.tuple_desc();
        (attno as usize) <= tupdesc.len()
    }

    /// Map a base relation variable to its position in this source's output.
    /// Since we flattened the join, this is just identity if RTI matches.
    pub fn map_var(
        &self,
        varno: pg_sys::Index,
        attno: pg_sys::AttrNumber,
    ) -> Option<pg_sys::AttrNumber> {
        if self.scan_info.heap_rti == varno {
            Some(attno)
        } else {
            None
        }
    }

    /// Resolve an attribute number to its DataFusion column name.
    pub fn column_name(&self, attno: pg_sys::AttrNumber) -> Option<String> {
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

    pub fn execution_alias(&self) -> String {
        RelationAlias::new(self.scan_info.alias.as_deref()).execution(self.plan_position)
    }
}

impl TryFrom<JoinSourceCandidate> for JoinSource {
    type Error = anyhow::Error;

    fn try_from(candidate: JoinSourceCandidate) -> Result<Self, Self::Error> {
        Ok(JoinSource {
            plan_position: 0,
            root_id: Some(candidate.root_id),
            scan_info: ScanInfo {
                heap_rti: candidate.heap_rti,
                heaprelid: candidate.heaprelid.ok_or_else(|| {
                    anyhow!(
                        "cannot build JoinSource for RTI {}: heaprelid is missing",
                        candidate.heap_rti
                    )
                })?,
                indexrelid: candidate.indexrelid.ok_or_else(|| {
                    anyhow!(
                        "cannot build JoinSource for RTI {}: indexrelid is missing",
                        candidate.heap_rti
                    )
                })?,
                query: candidate.query.unwrap_or(SearchQueryInput::All),
                has_search_predicate: candidate.has_search_predicate,
                alias: candidate.alias,
                score_needed: candidate.score_needed,
                fields: candidate.fields,
                sort_order: candidate.sort_order,
                estimate: candidate.estimate.ok_or_else(|| {
                    anyhow!(
                        "cannot build JoinSource for RTI {}: estimate is missing",
                        candidate.heap_rti
                    )
                })?,
                segment_count: candidate.segment_count.ok_or_else(|| {
                    anyhow!(
                        "cannot build JoinSource for RTI {}: segment_count is missing",
                        candidate.heap_rti
                    )
                })?,
                estimated_rows_per_worker: candidate.estimated_rows_per_worker,
            },
        })
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
        /// Plan position of the source (in the order yielded by `RelNode::sources()`) this predicate references.
        plan_position: usize,
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
    /// Post-LeftMark-join filter: `mark = true OR col IS NULL` (or `mark = false OR col IS NULL`
    /// for the anti/NOT-IN variant). Used to implement `col IS NULL OR col IN (SELECT ...)`.
    MarkOrNull {
        /// True for NOT IN patterns (anti-join semantics).
        is_anti: bool,
        /// Varno of the outer column tested for IS NULL.
        null_test_varno: pgrx::pg_sys::Index,
        /// Attribute number of the outer column tested for IS NULL.
        null_test_attno: pgrx::pg_sys::AttrNumber,
    },
    /// A PostgreSQL expression serialized via `nodeToString`, evaluated as a
    /// join filter.
    ///
    /// During planning the raw `pg_sys::Expr` is validated for DataFusion
    /// translatability via `PredicateTranslator::translate` and serialized with
    /// `nodeToString`. At execution time `stringToNode` rehydrates the tree,
    /// which `PredicateTranslator` then translates to a DataFusion `Expr`
    /// (Var nodes resolve via `CombinedMapper` against the join's sources).
    ///
    /// `input_vars` describes each Var dependency (RTI, attno, plus the type
    /// metadata captured at planning time so execution avoids catalog
    /// lookups). The projection pass uses `(rti, attno)` to register the
    /// required columns; the type metadata is also serialized through
    /// `JoinCSClause` and available to any future consumer. Used for
    /// Semi/Anti join filters because the MultiTablePredicate / custom_exprs
    /// pipeline would fail in setrefs: Semi/Anti prunes the inner relation
    /// from the scan tlist, leaving inner-side Vars unresolvable.
    PgExpression {
        pg_node_string: String,
        input_vars: Vec<InputVarInfo>,
    },
}

/// A node in the intermediate relational plan tree.
///
/// `RelNode` serves as the Intermediate Representation (IR) between PostgreSQL's C-based
/// planning structures and DataFusion's pure-Rust logical plan builder.
///
/// Using `RelNode` allows `JoinScan` to:
/// 1. Pre-validate query topology (e.g., separating equi-join keys from general filters)
///    prior to executing DataFusion.
/// 2. Implement DataFusion's `TreeNode` trait for plan rewrites
///    (e.g., hoisting subqueries into `SemiJoin` or `AntiJoin` nodes) via bottom-up
///    and top-down traversals.
/// 3. Lower PostgreSQL's execution plan (which frequently mixes boolean
///    predicates and hash/merge keys) into DataFusion's typed `Join` structures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelNode {
    /// A base relation scan.
    Scan(Box<JoinSource>),
    /// A join between two relational nodes.
    Join(Box<JoinNode>),
    /// A filter applied to a relational node.
    Filter(Box<FilterNode>),
}

/// A join node in the relational plan tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinNode {
    pub join_type: JoinType,
    pub left: RelNode,
    pub right: RelNode,
    /// Explicitly separated equi-join keys for DataFusion's Hash/Merge joins.
    pub equi_keys: Vec<JoinKeyPair>,
    /// Any remaining non-equi join conditions.
    pub filter: Option<JoinLevelExpr>,
    /// The `plan_id` of the PostgreSQL SubPlan that this join was extracted
    /// from, if any.  Set for Semi/Anti/LeftMark joins created by
    /// `wrap_with_semi_anti` and `wrap_with_mark_filter`; `None` for joins
    /// that come from the normal join-hook path or path reconstruction.
    pub subplan_id: Option<i32>,
}

/// A filter node in the relational plan tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterNode {
    pub input: RelNode,
    pub predicate: JoinLevelExpr,
}

type JoinKeySide<'a> = (&'a JoinSource, pg_sys::AttrNumber);

// TODO: Implement `datafusion::common::tree_node::TreeNode` for `RelNode`.
// This trait will likely be implemented in a future patch to enable functional, boilerplate-free
// tree rewrites (using `.transform_up()` and `.transform_down()`). This is specifically
// useful for hoisting PostgreSQL subqueries (like `InitPlan`s temporarily stored inside
// expressions) into relational `SemiJoin` or `AntiJoin` nodes in the IR tree.

impl RelNode {
    /// Recursively collects all unsupported join types found in the tree.
    pub fn unsupported_join_types(&self) -> Vec<JoinType> {
        let mut unsupported = Vec::new();
        self.collect_unsupported_join_types(&mut unsupported);
        unsupported.sort_by_key(|t| t.to_string());
        unsupported.dedup_by_key(|t| t.to_string());
        unsupported
    }

    /// Rewrites equi-join keys that reference columns pruned by child semi/anti
    /// joins so they instead reference output-visible equivalents.
    ///
    /// For example, given the tree `p SEMI (c SEMI d)` where the inner semi-join
    /// has key `c.id = d.company_id`, if the outer semi-join's key is
    /// `p.company_id = d.company_id` (derived by PostgreSQL's transitive closure),
    /// `d.company_id` is pruned by the inner semi-join. This method rewrites
    /// the outer key to `p.company_id = c.id` using the inner join's equivalence.
    ///
    /// Returns `true` if all keys are (or were made) valid. Returns `false` if
    /// a pruned reference cannot be resolved to an output-visible equivalent.
    pub unsafe fn rewrite_pruned_join_keys(&mut self, root: *mut pg_sys::PlannerInfo) -> bool {
        match self {
            RelNode::Scan(_) => true,
            RelNode::Join(j) => {
                if !j.left.rewrite_pruned_join_keys(root) || !j.right.rewrite_pruned_join_keys(root)
                {
                    return false;
                }

                let left_output_rtis = j.left.output_rtis();
                let right_output_rtis = j.right.output_rtis();

                let all_output_rtis: Vec<pg_sys::Index> = left_output_rtis
                    .iter()
                    .chain(right_output_rtis.iter())
                    .copied()
                    .collect();

                for jk in &mut j.equi_keys {
                    let forward_ok = left_output_rtis.contains(&jk.outer_rti)
                        && right_output_rtis.contains(&jk.inner_rti);
                    let reversed_ok = left_output_rtis.contains(&jk.inner_rti)
                        && right_output_rtis.contains(&jk.outer_rti);
                    if forward_ok || reversed_ok {
                        continue;
                    }

                    let outer_ok = left_output_rtis.contains(&jk.outer_rti)
                        || right_output_rtis.contains(&jk.outer_rti);
                    let inner_ok = left_output_rtis.contains(&jk.inner_rti)
                        || right_output_rtis.contains(&jk.inner_rti);

                    if !outer_ok
                        && !substitute_pruned_key_side(
                            root,
                            &all_output_rtis,
                            jk.outer_rti,
                            jk.outer_attno,
                            &mut jk.outer_rti,
                            &mut jk.outer_attno,
                        )
                    {
                        return false;
                    }
                    if !inner_ok
                        && !substitute_pruned_key_side(
                            root,
                            &all_output_rtis,
                            jk.inner_rti,
                            jk.inner_attno,
                            &mut jk.inner_rti,
                            &mut jk.inner_attno,
                        )
                    {
                        return false;
                    }
                }
                true
            }
            RelNode::Filter(f) => f.input.rewrite_pruned_join_keys(root),
        }
    }

    /// Returns true if the query tree contains a SEMI or ANTI join at any level.
    pub fn has_semi_or_anti(&self) -> bool {
        match self {
            RelNode::Scan(_) => false,
            RelNode::Join(j) => {
                matches!(
                    j.join_type,
                    JoinType::Semi | JoinType::Anti | JoinType::LeftMark | JoinType::RightMark
                ) || j.left.has_semi_or_anti()
                    || j.right.has_semi_or_anti()
            }
            RelNode::Filter(f) => f.input.has_semi_or_anti(),
        }
    }

    fn collect_unsupported_join_types(&self, acc: &mut Vec<JoinType>) {
        match self {
            RelNode::Scan(_) => {}
            RelNode::Join(j) => {
                if !matches!(
                    j.join_type,
                    JoinType::Inner
                        | JoinType::Semi
                        | JoinType::Anti
                        | JoinType::LeftMark
                        | JoinType::RightMark
                ) {
                    acc.push(j.join_type);
                }
                j.left.collect_unsupported_join_types(acc);
                j.right.collect_unsupported_join_types(acc);
            }
            RelNode::Filter(f) => f.input.collect_unsupported_join_types(acc),
        }
    }

    pub fn contains_rti(&self, rti: pg_sys::Index) -> bool {
        match self {
            RelNode::Scan(s) => s.scan_info.heap_rti == rti,
            RelNode::Join(j) => j.left.contains_rti(rti) || j.right.contains_rti(rti),
            RelNode::Filter(f) => f.input.contains_rti(rti),
        }
    }

    pub fn source_for_rti_in_subtree(&self, rti: pg_sys::Index) -> Option<&JoinSource> {
        self.sources().into_iter().find(|s| s.contains_rti(rti))
    }

    /// Recursively collects all base join sources from this tree.
    pub fn sources(&self) -> Vec<&JoinSource> {
        let mut result = Vec::new();
        self.collect_sources(&mut result);
        result
    }

    fn collect_sources<'a>(&'a self, acc: &mut Vec<&'a JoinSource>) {
        match self {
            RelNode::Scan(s) => acc.push(&**s),
            RelNode::Join(j) => {
                j.left.collect_sources(acc);
                j.right.collect_sources(acc);
            }
            RelNode::Filter(f) => f.input.collect_sources(acc),
        }
    }

    /// Recursively collects all mutable base join sources from this tree.
    pub fn sources_mut(&mut self) -> Vec<&mut JoinSource> {
        let mut result = Vec::new();
        self.collect_sources_mut(&mut result);
        result
    }

    fn collect_sources_mut<'a>(&'a mut self, acc: &mut Vec<&'a mut JoinSource>) {
        match self {
            RelNode::Scan(s) => acc.push(&mut **s),
            RelNode::Join(j) => {
                j.left.collect_sources_mut(acc);
                j.right.collect_sources_mut(acc);
            }
            RelNode::Filter(f) => f.input.collect_sources_mut(acc),
        }
    }

    /// Recursively collects all output RTIs (ignoring pruned sides like the right side of SemiJoin).
    pub fn output_rtis(&self) -> Vec<pg_sys::Index> {
        self.output_sources()
            .into_iter()
            .map(|s| s.scan_info.heap_rti)
            .collect()
    }

    /// Recursively collects output-visible base sources (ignoring pruned sides like the
    /// right side of SemiJoin).
    pub fn output_sources(&self) -> Vec<&JoinSource> {
        let mut result = Vec::new();
        self.collect_output_sources(&mut result);
        result
    }

    fn collect_output_sources<'a>(&'a self, acc: &mut Vec<&'a JoinSource>) {
        match self {
            RelNode::Scan(s) => acc.push(&**s),
            RelNode::Join(j) => match j.join_type {
                JoinType::Semi | JoinType::Anti | JoinType::LeftMark => {
                    j.left.collect_output_sources(acc);
                }
                JoinType::RightSemi | JoinType::RightAnti | JoinType::RightMark => {
                    j.right.collect_output_sources(acc);
                }
                _ => {
                    j.left.collect_output_sources(acc);
                    j.right.collect_output_sources(acc);
                }
            },
            RelNode::Filter(f) => f.input.collect_output_sources(acc),
        }
    }

    /// Recursively collects all equi-join keys from this tree.
    pub fn join_keys(&self) -> Vec<JoinKeyPair> {
        let mut result = Vec::new();
        self.collect_join_keys(&mut result);
        result
    }

    fn collect_join_keys(&self, acc: &mut Vec<JoinKeyPair>) {
        match self {
            RelNode::Scan(_) => {}
            RelNode::Join(j) => {
                acc.extend(j.equi_keys.iter().cloned());
                j.left.collect_join_keys(acc);
                j.right.collect_join_keys(acc);
            }
            RelNode::Filter(f) => f.input.collect_join_keys(acc),
        }
    }

    /// Recursively collect every `(rti, attno)` referenced by a join-level
    /// filter (`JoinNode.filter`). Used by `build_source_df` to keep the
    /// referenced columns out of the deferred-output promotion — the filter
    /// is evaluated before the join emits rows, so the columns must be
    /// materialized in the per-source scan.
    pub fn filter_input_vars(&self) -> Vec<(pg_sys::Index, pg_sys::AttrNumber)> {
        let mut result = Vec::new();
        self.collect_filter_input_vars(&mut result);
        result
    }

    fn collect_filter_input_vars(&self, acc: &mut Vec<(pg_sys::Index, pg_sys::AttrNumber)>) {
        match self {
            RelNode::Scan(_) => {}
            RelNode::Join(j) => {
                if let Some(JoinLevelExpr::PgExpression { input_vars, .. }) = &j.filter {
                    acc.extend(input_vars.iter().map(|v| (v.rti, v.attno)));
                }
                j.left.collect_filter_input_vars(acc);
                j.right.collect_filter_input_vars(acc);
            }
            RelNode::Filter(f) => f.input.collect_filter_input_vars(acc),
        }
    }

    /// Resolve every equi-join key in this tree structurally against the
    /// specific join node that owns it, and return `(plan_position, attno)`
    /// pairs identifying the exact sources that must project each key column.
    ///
    /// A flat lookup keyed on `(rti, attno)` can pick the wrong source when a
    /// SubPlan's inner relation shares an RTI value with an outer relation
    /// (inner queries have their own RTI numbering). By resolving each key
    /// against its owning `JoinNode`'s own `left`/`right` subtrees with the
    /// same logic used at execution time (`JoinKeyPair::resolve_against`),
    /// we bind each key to the correct `JoinSource` unambiguously.
    pub fn join_key_projections(&self) -> Vec<(usize, pg_sys::AttrNumber)> {
        let mut result = Vec::new();
        self.collect_join_key_projections(&mut result);
        result
    }

    fn collect_join_key_projections(&self, acc: &mut Vec<(usize, pg_sys::AttrNumber)>) {
        match self {
            RelNode::Scan(_) => {}
            RelNode::Join(j) => {
                for jk in &j.equi_keys {
                    if let Some(((left_src, left_att), (right_src, right_att))) =
                        jk.resolve_against(&j.left, &j.right)
                    {
                        acc.push((left_src.plan_position, left_att));
                        acc.push((right_src.plan_position, right_att));
                    }
                }
                if let Some(JoinLevelExpr::PgExpression { input_vars, .. }) = &j.filter {
                    for v in input_vars {
                        if let Some(source) = j
                            .left
                            .source_for_rti_in_subtree(v.rti)
                            .or_else(|| j.right.source_for_rti_in_subtree(v.rti))
                        {
                            acc.push((source.plan_position, v.attno));
                        }
                    }
                }
                j.left.collect_join_key_projections(acc);
                j.right.collect_join_key_projections(acc);
            }
            RelNode::Filter(f) => f.input.collect_join_key_projections(acc),
        }
    }

    /// Distribute equi-join keys to the correct join level in the tree.
    ///
    /// For 2-table joins all keys land on the single JoinNode. For 3+ table
    /// joins each key is placed at the deepest JoinNode where one RTI is in the
    /// left subtree and the other is in the right. This prevents
    /// `resolve_against` failures caused by both RTIs being in the same subtree.
    ///
    /// **Why AggregateScan needs this but JoinScan does not:**
    /// JoinScan hooks into `join_pathlist`, which PostgreSQL calls bottom-up
    /// for each join pair — so equi-keys arrive pre-distributed across join
    /// levels by the planner itself. AggregateScan hooks into
    /// `UPPERREL_GROUP_AGG` (post-join), where it must reconstruct the join
    /// tree from the parse tree. For implicit joins (`FROM a, b, c WHERE ...`)
    /// all equi-keys land in a flat WHERE clause, not distributed across join
    /// nodes, so this method is needed to place each key at the correct level.
    pub fn inject_equi_keys(&mut self, keys: Vec<JoinKeyPair>) {
        for key in keys {
            self.inject_single_equi_key(key);
        }
    }

    /// Place a single equi-key at the correct join level.
    /// Returns `true` if the key was successfully placed.
    fn inject_single_equi_key(&mut self, key: JoinKeyPair) -> bool {
        match self {
            RelNode::Join(ref mut join_node) => {
                let outer_in_left = join_node.left.contains_rti(key.outer_rti);
                let outer_in_right = join_node.right.contains_rti(key.outer_rti);
                let inner_in_left = join_node.left.contains_rti(key.inner_rti);
                let inner_in_right = join_node.right.contains_rti(key.inner_rti);

                // Key spans the two sides of this join — place it here
                if (outer_in_left && inner_in_right) || (outer_in_right && inner_in_left) {
                    let dup = join_node.equi_keys.iter().any(|k| {
                        (k.outer_rti == key.outer_rti
                            && k.outer_attno == key.outer_attno
                            && k.inner_rti == key.inner_rti
                            && k.inner_attno == key.inner_attno)
                            || (k.outer_rti == key.inner_rti
                                && k.outer_attno == key.inner_attno
                                && k.inner_rti == key.outer_rti
                                && k.inner_attno == key.outer_attno)
                    });
                    if !dup {
                        join_node.equi_keys.push(key);
                    }
                    return true;
                }

                // Both RTIs in left subtree — recurse left
                if outer_in_left && inner_in_left {
                    return join_node.left.inject_single_equi_key(key);
                }

                // Both RTIs in right subtree — recurse right
                if outer_in_right && inner_in_right {
                    return join_node.right.inject_single_equi_key(key);
                }

                false
            }
            RelNode::Filter(ref mut filter) => filter.input.inject_single_equi_key(key),
            RelNode::Scan(_) => false,
        }
    }

    /// Returns true if any `JoinNode` in the tree has an empty `equi_keys` list.
    /// Used to reject plans where an intermediate join (e.g., CROSS JOIN inside
    /// a 3-table query) would cause DataFusion to error or produce empty batches.
    pub fn has_join_without_keys(&self) -> bool {
        match self {
            RelNode::Scan(_) => false,
            RelNode::Join(j) => {
                j.equi_keys.is_empty()
                    || j.left.has_join_without_keys()
                    || j.right.has_join_without_keys()
            }
            RelNode::Filter(f) => f.input.has_join_without_keys(),
        }
    }

    /// Extract the top-level join_level_expr if present.
    pub fn join_level_expr(&self) -> Option<&JoinLevelExpr> {
        match self {
            RelNode::Filter(f) => Some(&f.predicate),
            _ => None,
        }
    }

    /// Recursively renders a human-readable representation of the join tree.
    pub fn explain(&self) -> String {
        self.explain_internal(true)
    }

    fn explain_internal(&self, is_root: bool) -> String {
        match self {
            RelNode::Scan(s) => {
                if let Some(alias) = &s.scan_info.alias {
                    alias.clone()
                } else {
                    PgSearchRelation::open(s.scan_info.heaprelid)
                        .name()
                        .to_string()
                }
            }
            RelNode::Join(j) => {
                let join_type_str = j.join_type.to_string().to_uppercase();
                let inner = format!(
                    "{} {} {}",
                    j.left.explain_internal(false),
                    join_type_str,
                    j.right.explain_internal(false)
                );

                if is_root {
                    inner
                } else {
                    format!("({})", inner)
                }
            }
            RelNode::Filter(f) => f.input.explain_internal(is_root),
        }
    }
}

/// Finds an output-visible equivalent for `(pruned_rti, pruned_attno)` by
/// searching PostgreSQL planner equivalence classes and writes it into
/// `out_rti` and `out_attno`. Returns `true` on success.
#[inline]
unsafe fn substitute_pruned_key_side(
    root: *mut pg_sys::PlannerInfo,
    output_rtis: &[pg_sys::Index],
    pruned_rti: pg_sys::Index,
    pruned_attno: pg_sys::AttrNumber,
    out_rti: &mut pg_sys::Index,
    out_attno: &mut pg_sys::AttrNumber,
) -> bool {
    if root.is_null() {
        return false;
    }

    let eq_classes = PgList::<pg_sys::EquivalenceClass>::from_pg((*root).eq_classes);
    for eqc in eq_classes.iter_ptr() {
        let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*eqc).ec_members);
        let mut contains_pruned = false;
        let mut replacement: Option<(pg_sys::Index, pg_sys::AttrNumber)> = None;

        for member in members.iter_ptr() {
            let mut node = (*member).em_expr.cast::<pg_sys::Node>();
            while !node.is_null() {
                match (*node).type_ {
                    pg_sys::NodeTag::T_RelabelType => {
                        node = (*(node as *mut pg_sys::RelabelType)).arg.cast();
                    }
                    pg_sys::NodeTag::T_PlaceHolderVar => {
                        node = (*(node as *mut pg_sys::PlaceHolderVar)).phexpr.cast();
                    }
                    _ => break,
                }
            }

            if node.is_null() || (*node).type_ != pg_sys::NodeTag::T_Var {
                continue;
            }

            let var = node as *mut pg_sys::Var;
            let rti = (*var).varno as pg_sys::Index;
            let attno = (*var).varattno;

            if rti == pruned_rti && attno == pruned_attno {
                contains_pruned = true;
                continue;
            }

            if output_rtis.contains(&rti) && replacement.is_none() {
                replacement = Some((rti, attno));
            }
        }

        if contains_pruned {
            if let Some((rti, attno)) = replacement {
                *out_rti = rti;
                *out_attno = attno;
                return true;
            }
        }
    }

    false
}

impl Default for RelNode {
    fn default() -> Self {
        RelNode::Scan(Box::new(JoinSource {
            plan_position: 0,
            root_id: None,
            scan_info: ScanInfo::default(),
        }))
    }
}

/// The clause information for a Join Custom Scan.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JoinCSClause {
    /// The root of the relational execution tree.
    pub plan: RelNode,
    /// The LIMIT and OFFSET value from the query, if any.
    pub limit_offset: LimitOffset,
    /// Join-level search predicates (Tantivy queries to execute).
    pub join_level_predicates: Vec<JoinLevelSearchPredicate>,
    /// Heap conditions (PostgreSQL expressions referencing both sides).
    pub multi_table_predicates: Vec<MultiTablePredicateInfo>,
    /// ORDER BY clause to be applied to the DataFusion plan.
    pub order_by: Vec<OrderByInfo>,
    /// Projection of output columns for this join.
    pub output_projection: Option<Vec<ChildProjection>>,
    /// Whether the join has DISTINCT specified.
    pub has_distinct: bool,
    /// Optional index of the source that MUST be partitioned, overriding cost-based selection.
    pub forced_partitioning_idx: Option<usize>,
}

impl JoinCSClause {
    pub fn new(plan: RelNode) -> Self {
        let mut clause = Self {
            plan,
            limit_offset: Default::default(),
            join_level_predicates: Vec::new(),
            multi_table_predicates: Vec::new(),
            order_by: Vec::new(),
            output_projection: None,
            has_distinct: false,
            forced_partitioning_idx: None,
        };
        for (i, source) in clause.plan.sources_mut().into_iter().enumerate() {
            source.plan_position = i;
        }
        clause
    }

    pub fn with_limit(mut self, limit: Option<u32>) -> Self {
        self.limit_offset.limit = limit;
        self
    }

    pub fn with_offset(mut self, offset: Option<u32>) -> Self {
        self.limit_offset.offset = offset;
        self
    }

    pub fn with_order_by(mut self, order_by: Vec<OrderByInfo>) -> Self {
        self.order_by = order_by;
        self
    }

    pub fn with_distinct(mut self, has_distinct: bool) -> Self {
        self.has_distinct = has_distinct;
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
        display_string: String,
    ) -> usize {
        let idx = self.join_level_predicates.len();
        self.join_level_predicates.push(JoinLevelSearchPredicate {
            rti,
            indexrelid,
            heaprelid,
            query,
            display_string,
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

    /// Set the join-level expression tree by wrapping the current plan in a FilterNode.
    pub fn with_join_level_expr(mut self, expr: JoinLevelExpr) -> Self {
        let current_plan = self.plan.clone();
        self.plan = RelNode::Filter(Box::new(FilterNode {
            input: current_plan,
            predicate: expr,
        }));
        self
    }

    pub fn with_forced_partitioning(mut self, idx: usize) -> Self {
        self.forced_partitioning_idx = Some(idx);
        self
    }

    /// Returns the source that should be partitioned for parallel execution.
    pub fn partitioning_source(&self) -> JoinSource {
        let sources = self.plan.sources();
        sources
            .get(self.partitioning_source_index())
            .cloned()
            .expect("JoinScan requires at least one source")
            .clone()
    }

    /// Returns the index of the source that should be partitioned for parallel execution.
    pub fn partitioning_source_index(&self) -> usize {
        if let Some(idx) = self.forced_partitioning_idx {
            return idx;
        }
        let sources = self.plan.sources();
        sources
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.scan_info.estimate.cmp(&b.scan_info.estimate))
            .map(|(i, _)| i)
            .expect("JoinScan requires at least one source")
    }

    /// Recursively collect all base relations in this join tree.
    pub fn collect_base_relations(&self, acc: &mut Vec<ScanInfo>) {
        for source in self.plan.sources() {
            source.collect_base_relations(acc);
        }
    }

    /// Resolve an output Var to a unique source index using output-visible sources.
    pub fn plan_position(
        &self,
        root_id: PlannerRootId,
        rti: pg_sys::Index,
        attno: pg_sys::AttrNumber,
    ) -> Option<usize> {
        let mut matches = self
            .plan
            .output_sources()
            .into_iter()
            .filter(|s| s.root_id == Some(root_id) && s.contains_rti(rti) && s.has_attno(attno))
            .map(|s| s.plan_position);

        let first = matches.next()?;
        debug_assert!(
            matches.next().is_none(),
            "plan_position: multiple output sources matched rti={rti}, attno={attno}"
        );
        Some(first)
    }

    pub fn source_for_var(
        &self,
        rti: pg_sys::Index,
        attno: pg_sys::AttrNumber,
    ) -> Option<&JoinSource> {
        let mut root_ids = self
            .plan
            .sources()
            .into_iter()
            .filter(|s| s.contains_rti(rti))
            .filter_map(|s| s.root_id);
        let root_id = root_ids.next()?;
        if !root_ids.all(|id| id == root_id) {
            return None;
        }

        let mut matches =
            self.plan.sources().into_iter().filter(|s| {
                s.root_id == Some(root_id) && s.contains_rti(rti) && s.has_attno(attno)
            });

        let first = matches.next()?;
        if matches.next().is_none() {
            Some(first)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Shared utilities used by both JoinScan and AggregateScan
// ---------------------------------------------------------------------------

/// Strip expression wrappers (`RelabelType`, `PlaceHolderVar`) to get the
/// underlying node. Used when extracting `Var` nodes from join conditions
/// that may have implicit type casts.
pub unsafe fn strip_node_wrappers(mut node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    loop {
        if node.is_null() {
            return node;
        }
        match (*node).type_ {
            pg_sys::NodeTag::T_RelabelType => {
                node = (*(node as *mut pg_sys::RelabelType)).arg.cast();
            }
            pg_sys::NodeTag::T_PlaceHolderVar => {
                node = (*(node as *mut pg_sys::PlaceHolderVar)).phexpr.cast();
            }
            _ => break,
        }
    }
    node
}

/// Try to extract a [`JoinKeyPair`] from an `OpExpr` node.
///
/// Returns `Some(JoinKeyPair)` if the expression is `var1 = var2` where both
/// variables reference different tables within `valid_rtis`. Uses
/// `op_mergejoinable` as the canonical equality check (more correct than
/// string-comparing operator names).
///
/// Shared between JoinScan (`extract_join_conditions_from_list`) and
/// AggregateScan (`extract_equi_keys_from_expr`).
pub unsafe fn try_extract_equi_key(
    op: *mut pg_sys::OpExpr,
    valid_rtis: &[pg_sys::Index],
) -> Option<JoinKeyPair> {
    if !pg_sys::op_mergejoinable((*op).opno, pg_sys::Oid::INVALID) {
        return None;
    }

    let args = PgList::<pg_sys::Node>::from_pg((*op).args);
    if args.len() != 2 {
        return None;
    }

    let left_node = strip_node_wrappers(args.get_ptr(0)?);
    let right_node = strip_node_wrappers(args.get_ptr(1)?);

    if (*left_node).type_ != pg_sys::NodeTag::T_Var || (*right_node).type_ != pg_sys::NodeTag::T_Var
    {
        return None;
    }

    let left_var = left_node as *mut pg_sys::Var;
    let right_var = right_node as *mut pg_sys::Var;

    let left_rti = (*left_var).varno as pg_sys::Index;
    let right_rti = (*right_var).varno as pg_sys::Index;

    // Must reference different tables, both within scope
    if left_rti == right_rti {
        return None;
    }
    if !valid_rtis.contains(&left_rti) || !valid_rtis.contains(&right_rti) {
        return None;
    }

    let mut typlen: i16 = 0;
    let mut typbyval: bool = false;
    pg_sys::get_typlenbyval(
        (*left_var).vartype,
        &mut typlen as *mut _,
        &mut typbyval as *mut _,
    );

    Some(JoinKeyPair {
        outer_rti: left_rti,
        outer_attno: (*left_var).varattno,
        inner_rti: right_rti,
        inner_attno: (*right_var).varattno,
        type_oid: (*left_var).vartype,
        typlen,
        typbyval,
    })
}

/// Look up base-relation metadata for a given RTI: relid, alias, and BM25 index.
///
/// Shared between JoinScan's `collect_join_sources_base_rel` and AggregateScan's
/// `collect_join_agg_sources`. Returns `None` if the RTI doesn't point to a
/// plain relation (e.g., subquery, CTE).
pub unsafe fn lookup_base_rel_info(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
) -> Option<(pg_sys::Oid, Option<String>, Option<PgSearchRelation>)> {
    let rte = get_rte(
        (*root).simple_rel_array_size as usize,
        (*root).simple_rte_array,
        rti,
    )?;

    let relid = get_plain_relation_relid(rte)?;

    let alias = if !(*rte).eref.is_null() && !(*(*rte).eref).aliasname.is_null() {
        std::ffi::CStr::from_ptr((*(*rte).eref).aliasname)
            .to_str()
            .ok()
            .map(|s| s.to_string())
    } else {
        None
    };

    let bm25_index = rel_get_bm25_index(relid).map(|(_, idx)| idx);

    Some((relid, alias, bm25_index))
}

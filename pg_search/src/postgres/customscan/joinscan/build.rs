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
use crate::postgres::utils::ExprContextGuard;
use crate::query::SearchQueryInput;
pub use crate::scan::ScanInfo;
use anyhow::anyhow;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ptr::NonNull;

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

/// Projection information for a child join.
/// Maps an output attribute (by index in the vector) to the source column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildProjection {
    pub rti: pg_sys::Index,
    pub attno: pg_sys::AttrNumber,
    #[serde(default)]
    pub is_score: bool,
}

use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::options::SortByField;
use crate::postgres::rel::PgSearchRelation;
use crate::scan::info::{FieldInfo, RowEstimate};

/// Source information collected during planning.
///
/// This represents a relation before all required JoinScan invariants are verified.
/// Optional fields are progressively filled as planning discovers index metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JoinSourceCandidate {
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
}

impl JoinSourceCandidate {
    pub fn new(heap_rti: pg_sys::Index) -> Self {
        Self {
            heap_rti,
            ..Default::default()
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

    /// Returns the alias to be used for this source in the DataFusion plan.
    pub fn execution_alias(&self, index: usize) -> String {
        self.alias().unwrap_or_else(|| format!("source_{}", index))
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

        // `expr_context` only lives until the end of this function,
        // which is fine because it is only used to get estimates
        let expr_context = ExprContextGuard::new();
        let reader = SearchIndexReader::open_with_context(
            &index_rel,
            self.query
                .clone()
                .unwrap_or(crate::query::SearchQueryInput::All),
            false,
            MvccSatisfies::LargestSegment,
            NonNull::new(expr_context.as_ptr()),
            None,
        )
        .expect("Failed to open index reader for estimation");

        self.segment_count = Some(reader.total_segment_count());

        let row_estimate = RowEstimate::from_reltuples(heap_rel.reltuples().map(|r| r as f64));

        let (estimate, _) = reader.estimate_docs(row_estimate);
        self.estimate = Some(RowEstimate::Known(estimate as u64));
    }
}

/// Represents the validated source of data for a join side used during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinSource {
    pub scan_info: ScanInfo,
}

impl JoinSource {
    /// Returns the alias to be used for this source in the DataFusion plan.
    ///
    /// If the source has an explicit alias (from SQL), it is used.
    /// Otherwise, a synthetic alias `source_{index}` is generated based on its position.
    pub fn execution_alias(&self, index: usize) -> String {
        self.scan_info
            .alias
            .clone()
            .unwrap_or_else(|| format!("source_{}", index))
    }

    /// Check if this source contains the given RTI.
    pub fn contains_rti(&self, rti: pg_sys::Index) -> bool {
        self.scan_info.heap_rti == rti
    }

    /// Check if this source has a search predicate.
    pub fn has_search_predicate(&self) -> bool {
        self.scan_info.has_search_predicate
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
}

impl TryFrom<JoinSourceCandidate> for JoinSource {
    type Error = anyhow::Error;

    fn try_from(candidate: JoinSourceCandidate) -> Result<Self, Self::Error> {
        Ok(JoinSource {
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
                query: candidate
                    .query
                    .unwrap_or(crate::query::SearchQueryInput::All),
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
        /// Index of the source (in the order yielded by `RelNode::sources()`) this predicate references.
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
}

/// A filter node in the relational plan tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterNode {
    pub input: RelNode,
    pub predicate: JoinLevelExpr,
}

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

    fn collect_unsupported_join_types(&self, acc: &mut Vec<JoinType>) {
        match self {
            RelNode::Scan(_) => {}
            RelNode::Join(j) => {
                if !matches!(j.join_type, JoinType::Inner | JoinType::Semi) {
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
        let mut result = Vec::new();
        self.collect_output_rtis(&mut result);
        result
    }

    fn collect_output_rtis(&self, acc: &mut Vec<pg_sys::Index>) {
        match self {
            RelNode::Scan(s) => acc.push(s.scan_info.heap_rti),
            RelNode::Join(j) => match j.join_type {
                JoinType::Semi | JoinType::Anti => {
                    j.left.collect_output_rtis(acc);
                }
                JoinType::RightSemi | JoinType::RightAnti => {
                    j.right.collect_output_rtis(acc);
                }
                _ => {
                    j.left.collect_output_rtis(acc);
                    j.right.collect_output_rtis(acc);
                }
            },
            RelNode::Filter(f) => f.input.collect_output_rtis(acc),
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
                acc.extend(j.equi_keys.clone());
                j.left.collect_join_keys(acc);
                j.right.collect_join_keys(acc);
            }
            RelNode::Filter(f) => f.input.collect_join_keys(acc),
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

impl Default for RelNode {
    fn default() -> Self {
        RelNode::Scan(Box::new(JoinSource {
            scan_info: ScanInfo::default(),
        }))
    }
}

/// The clause information for a Join Custom Scan.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JoinCSClause {
    /// The root of the relational execution tree.
    pub plan: RelNode,
    /// The LIMIT value from the query, if any.
    pub limit: Option<usize>,
    /// Join-level search predicates (Tantivy queries to execute).
    pub join_level_predicates: Vec<JoinLevelSearchPredicate>,
    /// Heap conditions (PostgreSQL expressions referencing both sides).
    pub multi_table_predicates: Vec<MultiTablePredicateInfo>,
    /// ORDER BY clause to be applied to the DataFusion plan.
    pub order_by: Vec<OrderByInfo>,
    /// Projection of output columns for this join.
    pub output_projection: Option<Vec<ChildProjection>>,
}

impl JoinCSClause {
    pub fn new(plan: RelNode) -> Self {
        Self {
            plan,
            limit: None,
            join_level_predicates: Vec::new(),
            multi_table_predicates: Vec::new(),
            order_by: Vec::new(),
            output_projection: None,
        }
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

    /// Returns the index of the ordering side (the source with a search predicate).
    /// If multiple have it, returns the first one.
    pub fn ordering_side_index(&self) -> Option<usize> {
        self.plan
            .sources()
            .into_iter()
            .position(|s| s.has_search_predicate())
    }

    /// Get the ordering side source (side with search predicate).
    pub fn ordering_side(&self) -> Option<JoinSource> {
        self.ordering_side_index()
            .map(|i| self.plan.sources()[i].clone())
    }

    /// Returns the source that should be partitioned for parallel execution.
    /// This is the source with the largest row estimate.
    pub fn partitioning_source(&self) -> JoinSource {
        let sources = self.plan.sources();
        sources
            .into_iter()
            .max_by(|a, b| a.scan_info.estimate.cmp(&b.scan_info.estimate))
            .cloned()
            .expect("JoinScan requires at least one source")
    }

    /// Returns the index of the source that should be partitioned for parallel execution.
    /// This is the source with the largest row estimate.
    pub fn partitioning_source_index(&self) -> usize {
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
}

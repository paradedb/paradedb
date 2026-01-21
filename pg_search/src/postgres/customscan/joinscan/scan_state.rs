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

//! Execution state for JoinScan custom scan.

use crate::api::HashMap;
use crate::postgres::customscan::joinscan::build::JoinCSClause;
use crate::postgres::customscan::joinscan::executors::JoinSideExecutor;
use crate::postgres::customscan::joinscan::privdat::OutputColumnInfo;
use crate::postgres::customscan::CustomScanState;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use pgrx::pg_sys;
use std::collections::{HashSet, VecDeque};
use std::hash::{Hash, Hasher};

/// A single key value, stored as copied bytes for pass-by-reference types.
/// This allows us to store values of any PostgreSQL type in the hash table.
#[derive(Debug, Clone)]
pub struct KeyValue {
    /// The raw bytes of the datum value (copied for varlena types).
    pub data: Vec<u8>,
}

impl PartialEq for KeyValue {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl Eq for KeyValue {}

impl Hash for KeyValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

/// Composite join key that stores actual values.
/// This avoids hash collisions by comparing the actual key data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompositeKey {
    /// Cross-join (no equi-join keys) - distinct variant that cannot collide with real keys.
    CrossJoin,
    /// Actual key values (one per join column).
    Values(Vec<KeyValue>),
}

/// Runtime key info for extracting join keys during execution.
#[derive(Debug, Clone, Default)]
pub struct JoinKeyInfo {
    /// Attribute number (1-indexed).
    pub attno: i32,
    /// Type length from pg_type.typlen (-1 for varlena, -2 for cstring).
    pub typlen: i16,
    /// Whether type is pass-by-value.
    pub typbyval: bool,
}

/// Represents an inner side row stored in the hash table.
#[derive(Debug, Clone)]
pub struct InnerRow {
    /// The ctid of the inner row.
    pub ctid: u64,
    /// The BM25 score of this row (if build side needs scores).
    pub score: f32,
}

/// Which side of the join a predicate references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinSide {
    Outer,
    Inner,
}

/// A runtime-evaluable boolean expression over join-level conditions.
///
/// This preserves the full AND/OR/NOT structure of complex join-level predicates,
/// allowing correct evaluation of expressions like:
/// `(search_pred OR heap_cond)` or `(tbl1_search AND NOT tbl2_search)`
///
/// The tree can reference two types of leaf conditions:
/// - `Predicate`: A Tantivy search result (evaluated via ctid set membership)
/// - `HeapCondition`: A PostgreSQL expression (evaluated via ExecQual at runtime)
#[derive(Debug, Clone)]
pub enum JoinLevelExpr {
    /// Leaf: check if the row's ctid is in the predicate's result set (Tantivy).
    Predicate {
        /// Which side of the join this predicate references.
        side: JoinSide,
        /// Index into the `join_level_ctid_sets` vector.
        ctid_set_idx: usize,
    },
    /// Leaf: evaluate a PostgreSQL expression (heap condition).
    /// This requires runtime evaluation via ExecQual.
    HeapCondition {
        /// Index into the `heap_condition_states` vector.
        condition_idx: usize,
    },
    /// Logical AND of child expressions.
    And(Vec<JoinLevelExpr>),
    /// Logical OR of child expressions.
    Or(Vec<JoinLevelExpr>),
    /// Logical NOT of a child expression.
    Not(Box<JoinLevelExpr>),
}

/// Context needed to evaluate a JoinLevelExpr that may contain HeapConditions.
pub struct JoinLevelEvalContext<'a> {
    /// Ctid sets from Tantivy queries.
    pub ctid_sets: &'a [HashSet<u64>],
    /// Expression states for heap conditions (parallel to heap_conditions in JoinCSClause).
    pub heap_condition_states: &'a [*mut pg_sys::ExprState],
    /// Expression context for evaluating heap conditions.
    pub econtext: *mut pg_sys::ExprContext,
}

impl JoinLevelExpr {
    /// Evaluate this expression for a given row-pair.
    ///
    /// For `Predicate` nodes: checks ctid membership in the pre-computed sets.
    /// For `HeapCondition` nodes: evaluates the PostgreSQL expression via ExecQual.
    ///
    /// # Safety
    /// The econtext in eval_ctx must have ecxt_scantuple set to a slot containing
    /// the joined tuple data before calling this function.
    pub unsafe fn evaluate(
        &self,
        outer_ctid: u64,
        inner_ctid: u64,
        eval_ctx: &JoinLevelEvalContext,
    ) -> bool {
        match self {
            JoinLevelExpr::Predicate { side, ctid_set_idx } => {
                let ctid = match side {
                    JoinSide::Outer => outer_ctid,
                    JoinSide::Inner => inner_ctid,
                };
                eval_ctx
                    .ctid_sets
                    .get(*ctid_set_idx)
                    .map(|set| set.contains(&ctid))
                    .unwrap_or(false)
            }
            JoinLevelExpr::HeapCondition { condition_idx } => {
                // Evaluate the PostgreSQL expression
                if let Some(&expr_state) = eval_ctx.heap_condition_states.get(*condition_idx) {
                    if !expr_state.is_null() && !eval_ctx.econtext.is_null() {
                        // ExecQual returns true if the expression evaluates to true
                        return pg_sys::ExecQual(expr_state, eval_ctx.econtext);
                    }
                }
                // If state is not initialized, treat as false (fail-safe)
                false
            }
            JoinLevelExpr::And(children) => children
                .iter()
                .all(|c| c.evaluate(outer_ctid, inner_ctid, eval_ctx)),
            JoinLevelExpr::Or(children) => children
                .iter()
                .any(|c| c.evaluate(outer_ctid, inner_ctid, eval_ctx)),
            JoinLevelExpr::Not(child) => !child.evaluate(outer_ctid, inner_ctid, eval_ctx),
        }
    }
}

/// The execution state for the JoinScan.
#[derive(Default)]
pub struct JoinScanState {
    /// The join clause from planning.
    pub join_clause: JoinCSClause,

    // === Driving side state (side with search predicate - we iterate through this) ===
    /// The heap relation for the driving side.
    pub driving_heaprel: Option<PgSearchRelation>,
    /// Executor for the driving side (FastField batched ctid lookups).
    pub driving_executor: Option<JoinSideExecutor>,
    /// Visibility checker for the driving side.
    pub driving_visibility_checker: Option<VisibilityChecker>,
    /// Slot for fetching driving side tuples.
    pub driving_fetch_slot: Option<*mut pg_sys::TupleTableSlot>,

    // === Build side state (side we build hash table from) ===
    /// The heap relation for the build side.
    pub build_heaprel: Option<PgSearchRelation>,
    /// Visibility checker for the build side.
    pub build_visibility_checker: Option<VisibilityChecker>,
    /// Heap scan descriptor for build side.
    pub build_scan_desc: Option<*mut pg_sys::TableScanDescData>,
    /// Slot for build side heap scan.
    pub build_scan_slot: Option<*mut pg_sys::TupleTableSlot>,
    /// Map of build side ctids to their BM25 scores (if build side has a search predicate).
    /// When this is Some, only rows with ctids in this map should be included in the hash table.
    /// The score is stored so it can be used if paradedb.score() references the build side.
    pub build_matching_ctids: Option<HashMap<u64, f32>>,

    // === Hash join state ===
    /// The hash table built from the build side.
    /// Key: composite key (supports any type), Value: list of build row ctids.
    pub hash_table: HashMap<CompositeKey, Vec<InnerRow>>,
    /// Whether the hash table has been built.
    pub hash_table_built: bool,

    // === Join key extraction info ===
    /// Key info for extracting keys from build side tuples.
    pub build_key_info: Vec<JoinKeyInfo>,
    /// Key info for extracting keys from driving side tuples.
    pub driving_key_info: Vec<JoinKeyInfo>,

    // === Driving heap scan (for join-level predicates with no side-level predicates) ===
    /// Heap scan descriptor for driving side (when no search reader).
    pub driving_scan_desc: Option<*mut pg_sys::TableScanDescData>,

    // === Probe state ===
    /// Current driving side ctid being probed.
    pub current_driving_ctid: Option<u64>,
    /// Current driving side score.
    pub current_driving_score: f32,
    /// Pending build side (ctid, score) pairs that match the current driving row.
    /// The score is used when paradedb.score() references the build side.
    pub pending_build_ctids: VecDeque<(u64, f32)>,

    // === Result state ===
    /// Result tuple slot.
    pub result_slot: Option<*mut pg_sys::TupleTableSlot>,
    /// Count of rows returned.
    pub rows_returned: usize,

    // === Side tracking ===
    /// Whether the driving side is the outer side (true) or inner side (false).
    pub driving_is_outer: bool,
    /// Whether this is a cross join (no equi-join keys).
    pub is_cross_join: bool,
    /// Whether driving side uses heap scan (vs search scan).
    /// When true, driving tuple is already in driving_fetch_slot.
    pub driving_uses_heap_scan: bool,

    // === Heap condition evaluation ===
    /// Compiled expression states for heap conditions (parallel to heap_conditions in JoinCSClause).
    /// Each HeapCondition in join_level_expr references an index into this vector.
    pub heap_condition_states: Vec<*mut pg_sys::ExprState>,
    /// Expression context for evaluating heap conditions.
    pub heap_condition_econtext: Option<*mut pg_sys::ExprContext>,

    // === Output column mapping ===
    /// Mapping of output column positions to their source (outer/inner) and original attribute numbers.
    /// Populated from PrivateData during create_custom_scan_state.
    pub output_columns: Vec<OutputColumnInfo>,

    // === Join-level predicate evaluation ===
    /// The runtime-evaluable expression tree for join-level predicates.
    /// When Some, this expression must be evaluated for each row-pair.
    pub join_level_expr: Option<JoinLevelExpr>,
    /// Ctid sets from Tantivy queries, indexed by predicate ID.
    /// Each JoinLevelExpr::Predicate references an index into this vector.
    pub join_level_ctid_sets: Vec<HashSet<u64>>,

    // === Memory tracking ===
    /// Estimated memory used by hash table (bytes).
    pub hash_table_memory: usize,
    /// Maximum allowed memory for hash table (from work_mem, in bytes).
    pub max_hash_memory: usize,
    /// Whether we exceeded memory limit and fell back to nested loop.
    pub using_nested_loop: bool,
}

impl JoinScanState {
    /// Reset the scan state for a rescan.
    pub fn reset(&mut self) {
        self.hash_table.clear();
        self.hash_table_built = false;
        self.current_driving_ctid = None;
        self.current_driving_score = 0.0;
        self.pending_build_ctids.clear();
        self.rows_returned = 0;
        // Note: join_level_expr and join_level_ctid_sets are populated once in begin_custom_scan
        // and reused across rescans, so we don't clear them here.
        // The driving_executor maintains its own state for incremental fetching.
        self.hash_table_memory = 0;
        self.using_nested_loop = false;
    }

    /// Returns (outer_slot, inner_slot) based on which side is driving.
    ///
    /// This maps the driving/build slots to outer/inner positions:
    /// - If driving_is_outer: driving_slot=outer, build_slot=inner
    /// - If driving_is_inner: driving_slot=inner, build_slot=outer
    pub fn outer_inner_slots(
        &self,
    ) -> (
        Option<*mut pg_sys::TupleTableSlot>,
        Option<*mut pg_sys::TupleTableSlot>,
    ) {
        if self.driving_is_outer {
            (self.driving_fetch_slot, self.build_scan_slot)
        } else {
            (self.build_scan_slot, self.driving_fetch_slot)
        }
    }

    /// Get the appropriate score for an output column.
    ///
    /// This determines whether to use the driving side score or the build side score
    /// based on which side the column references:
    /// - If `col_is_outer == driving_is_outer`: column references driving side → use driving_score
    /// - Otherwise: column references build side → use build_score
    pub fn score_for_column(&self, col_is_outer: bool, build_score: f32) -> f32 {
        if col_is_outer == self.driving_is_outer {
            self.current_driving_score
        } else {
            build_score
        }
    }
}

impl CustomScanState for JoinScanState {
    fn init_exec_method(&mut self, _cstate: *mut pg_sys::CustomScanState) {
        // No special initialization needed for the plain exec method
    }
}

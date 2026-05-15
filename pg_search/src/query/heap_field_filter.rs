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

use std::ptr::NonNull;

use crate::postgres::heap::HeapFetchState;
use crate::postgres::rel::PgSearchRelation;
use crate::query::PostgresPointer;

use pgrx::FromDatum;
use pgrx::{pg_guard, pg_sys, PgMemoryContexts};
use serde::{Deserialize, Serialize};
use tantivy::schema::Field;
use tantivy::{
    query::{EnableScoring, Explanation, Query, Scorer, Weight},
    DocId, DocSet, Score, SegmentReader, Term, TERMINATED,
};
/// Core heap-based field filter using PostgreSQL expression evaluation
/// This approach stores a serialized representation of the PostgreSQL expression
/// and evaluates it directly against heap tuples, supporting any PostgreSQL operator or function
#[derive(Debug, Serialize, Deserialize)]
pub struct HeapFieldFilter {
    /// PostgreSQL expression node that can be serialized and reconstructed
    expr_node: PostgresPointer,
    /// Human-readable description of the expression for EXPLAIN output
    pub heap_filter: String,

    #[serde(skip)]
    initialized_expression: Option<(*mut pg_sys::ExprState, Option<NonNull<pg_sys::PlanState>>)>,
    #[serde(skip)]
    heap_fetch_state: Option<HeapFetchState>,
}

impl Clone for HeapFieldFilter {
    fn clone(&self) -> Self {
        Self {
            expr_node: self.expr_node.clone(),
            heap_filter: self.heap_filter.clone(),
            initialized_expression: None,
            heap_fetch_state: None,
        }
    }
}

impl PartialEq for HeapFieldFilter {
    fn eq(&self, other: &HeapFieldFilter) -> bool {
        self.expr_node == other.expr_node && self.heap_filter == other.heap_filter
    }
}

// SAFETY:  we don't execute within threads, despite Tantivy expecting that to be the case
unsafe impl Send for HeapFieldFilter {}
unsafe impl Sync for HeapFieldFilter {}

impl HeapFieldFilter {
    /// Create a new HeapFieldFilter from a PostgreSQL expression node
    pub unsafe fn new(expr_node: *mut pg_sys::Node, heap_filter: String) -> Self {
        Self {
            expr_node: PostgresPointer(expr_node.cast()),
            heap_filter,
            initialized_expression: None,
            heap_fetch_state: None,
        }
    }

    /// Evaluate this filter against a heap tuple identified by ctid
    /// Uses PostgreSQL's expression evaluation system
    pub unsafe fn evaluate(
        &mut self,
        ctid: pg_sys::ItemPointer,
        heaprel: &PgSearchRelation,
        expr_context: NonNull<pg_sys::ExprContext>,
        planstate: Option<NonNull<pg_sys::PlanState>>,
    ) -> bool {
        // Get the expression node
        let expr_node = self.expr_node.0.cast::<pg_sys::Node>();
        if expr_node.is_null() {
            return true;
        }

        self.evaluate_expression_inner(ctid, heaprel, expr_node, expr_context, planstate)
    }

    /// Inner expression evaluation method that can be wrapped in panic handling
    unsafe fn evaluate_expression_inner(
        &mut self,
        ctid: pg_sys::ItemPointer,
        relation: &PgSearchRelation,
        expr_node: *mut pg_sys::Node,
        expr_context: NonNull<pg_sys::ExprContext>,
        planstate: Option<NonNull<pg_sys::PlanState>>,
    ) -> bool {
        let heap_fetch_state = self
            .heap_fetch_state
            .get_or_insert_with(|| HeapFetchState::new(relation));
        let econtext = expr_context.as_ptr();

        let mut call_again = false;
        let mut all_dead = false;
        if !heap_fetch_state.fetch_tuple(
            &mut *ctid,
            pg_sys::GetActiveSnapshot(),
            &mut call_again,
            &mut all_dead,
        ) {
            return false;
        }

        // Store the original scan tuple to restore later if we're using a provided context
        let original_scan_tuple = (*econtext).ecxt_scantuple;

        // Set the tuple slot in the expression context
        (*econtext).ecxt_scantuple = heap_fetch_state.slot();

        // Ensure all attributes in the slot are deformed (fetched from tuple storage)
        // This is necessary because the expression might reference any attribute,
        // and the slot's tts_nvalid must be >= the highest attribute number referenced
        pg_sys::slot_getallattrs(heap_fetch_state.slot());

        let eval_result = (|| {
            // Initialize the expression for execution with proper planstate for subquery support
            let expr_state = match (&self.initialized_expression, planstate) {
                // We have an existing expression state, which WAS NOT initialized without a planstate
                (Some((_existing_state, None)), Some(new_planstate)) => {
                    // Check if we need to reinitialize with a better planstate
                    self.init_expression_state(expr_node, Some(new_planstate))
                }
                // We have an existing expression state, which WAS either initialized with a planstate or
                // the newly given plan state is also None
                (Some((existing_state, _init_with_planstate)), _new_planstate) => *existing_state,
                // First initialization
                (None, planstate) => self.init_expression_state(expr_node, planstate),
            };
            if expr_state.is_null() {
                self.initialized_expression = None;
                return false;
            }

            // Evaluate the expression
            let mut is_null = false;
            let result = pg_sys::ExecEvalExpr(expr_state, econtext, &mut is_null);

            // Convert the result to a boolean
            bool::from_datum(result, is_null).unwrap_or(false)
        })();

        // Restore original scan tuple
        (*econtext).ecxt_scantuple = original_scan_tuple;

        eval_result
    }

    /// Helper function to initialize a new expression state and update the cached state
    unsafe fn init_expression_state(
        &mut self,
        expr_node: *mut pg_sys::Node,
        planstate: Option<NonNull<pg_sys::PlanState>>,
    ) -> *mut pg_sys::ExprState {
        let planstate_ptr = planstate.map_or(std::ptr::null_mut(), |ps| ps.as_ptr());
        let new_state = PgMemoryContexts::TopTransactionContext
            .switch_to(|_| pg_sys::ExecInitExpr(expr_node.cast(), planstate_ptr));
        self.initialized_expression = Some((new_state, planstate));
        new_state
    }

    /// Get the PostgreSQL expression node
    pub unsafe fn get_expression_node(&self) -> *mut pg_sys::Node {
        self.expr_node.0.cast()
    }

    /// Resolve InitPlan PARAM_EXEC nodes in the expression tree by replacing them with Const nodes.
    ///
    /// In PostgreSQL, subqueries like `(SELECT array[1, 2])` are converted to InitPlans during
    /// planning. The InitPlan result is stored in `EState->es_param_exec_vals[paramid]` after
    /// execution. The expression tree retains a `Param(PARAM_EXEC, paramid)` reference.
    ///
    /// When this expression is evaluated in a **parallel worker**, the worker's `EState` may not
    /// have the InitPlan result available, causing `ExecEvalExpr` to pass NULL to strict functions
    /// like `arrayoverlap`, resulting in a segfault (SIGSEGV at address 0x0).
    ///
    /// This method walks the expression tree and replaces each `PARAM_EXEC` whose value is already
    /// available in the EState with a `Const` node, making the expression self-contained and safe
    /// for evaluation in any execution context (leader or parallel worker).
    ///
    /// This approach mirrors PostgreSQL's own `eval_const_expressions()` pattern from the optimizer.
    pub unsafe fn resolve_initplan_params(&mut self, estate: *mut pg_sys::EState) {
        let expr_node = self.expr_node.0.cast::<pg_sys::Node>();
        if expr_node.is_null() || estate.is_null() {
            return;
        }

        // Walk the expression tree and replace PARAM_EXEC nodes in-place
        resolve_param_exec_mutator(expr_node, estate);
    }

    // The new expression-based approach handles evaluation directly
}

/// Recursively walk an expression tree and replace `Param(PARAM_EXEC)` nodes whose values
/// are already computed (i.e., their InitPlan has been executed) with `Const` nodes.
///
/// This is done by using PostgreSQL's `expression_tree_walker` pattern, but since we need
/// to mutate nodes in-place, we implement our own recursive walk for known container types.
///
/// # Safety
/// - `node` must be a valid PostgreSQL expression node
/// - `estate` must be a valid EState with `es_param_exec_vals` properly allocated
unsafe fn resolve_param_exec_mutator(node: *mut pg_sys::Node, estate: *mut pg_sys::EState) {
    if node.is_null() {
        return;
    }

    match (*node).type_ {
        // For BoolExpr (AND/OR/NOT), walk the args list and replace Param children in-place
        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = node as *mut pg_sys::BoolExpr;
            replace_params_in_list((*boolexpr).args, estate);
        }

        // For OpExpr (e.g., `field && expr`), walk the args list
        pg_sys::NodeTag::T_OpExpr => {
            let opexpr = node as *mut pg_sys::OpExpr;
            replace_params_in_list((*opexpr).args, estate);
        }

        // For FuncExpr (e.g., `arrayoverlap(a, b)`), walk the args list
        pg_sys::NodeTag::T_FuncExpr => {
            let funcexpr = node as *mut pg_sys::FuncExpr;
            replace_params_in_list((*funcexpr).args, estate);
        }

        // For ScalarArrayOpExpr (e.g., `field = ANY(array_expr)`), walk the args
        pg_sys::NodeTag::T_ScalarArrayOpExpr => {
            let saopexpr = node as *mut pg_sys::ScalarArrayOpExpr;
            replace_params_in_list((*saopexpr).args, estate);
        }

        // CoalesceExpr (e.g., `COALESCE(field, '{}')`) has an args list
        pg_sys::NodeTag::T_CoalesceExpr => {
            let coalesce = node as *mut pg_sys::CoalesceExpr;
            replace_params_in_list((*coalesce).args, estate);
        }

        // RelabelType wraps a single expression (type coercion)
        pg_sys::NodeTag::T_RelabelType => {
            let relabel = node as *mut pg_sys::RelabelType;
            let arg = (*relabel).arg as *mut pg_sys::Node;
            if !arg.is_null() && (*arg).type_ == pg_sys::NodeTag::T_Param {
                if let Some(const_node) = try_resolve_param(arg as *mut pg_sys::Param, estate) {
                    (*relabel).arg = const_node.cast();
                }
            } else {
                resolve_param_exec_mutator(arg, estate);
            }
        }

        // CoerceViaIO wraps a single expression (I/O coercion)
        pg_sys::NodeTag::T_CoerceViaIO => {
            let coerce = node as *mut pg_sys::CoerceViaIO;
            let arg = (*coerce).arg as *mut pg_sys::Node;
            if !arg.is_null() && (*arg).type_ == pg_sys::NodeTag::T_Param {
                if let Some(const_node) = try_resolve_param(arg as *mut pg_sys::Param, estate) {
                    (*coerce).arg = const_node.cast();
                }
            } else {
                resolve_param_exec_mutator(arg, estate);
            }
        }

        // NullTest wraps a single expression
        pg_sys::NodeTag::T_NullTest => {
            let nulltest = node as *mut pg_sys::NullTest;
            let arg = (*nulltest).arg as *mut pg_sys::Node;
            if !arg.is_null() && (*arg).type_ == pg_sys::NodeTag::T_Param {
                if let Some(const_node) = try_resolve_param(arg as *mut pg_sys::Param, estate) {
                    (*nulltest).arg = const_node.cast();
                }
            } else {
                resolve_param_exec_mutator(arg, estate);
            }
        }

        // Leaf nodes — nothing to recurse into
        pg_sys::NodeTag::T_Var | pg_sys::NodeTag::T_Const | pg_sys::NodeTag::T_Param => {
            // Param at the root can't be replaced (the caller doesn't own the pointer).
            // This is fine because top-level Params don't occur in HeapFieldFilter expressions.
        }

        // For other node types, delegate to PostgreSQL's expression_tree_walker
        // which knows how to iterate most expression node types
        _ => {
            pg_sys::expression_tree_walker(
                node,
                Some(resolve_param_walker_callback),
                estate.cast(),
            );
        }
    }
}

/// Callback for `expression_tree_walker` — resolves PARAM_EXEC nodes encountered during
/// a generic tree walk. This handles node types not explicitly covered above.
#[pg_guard]
unsafe extern "C-unwind" fn resolve_param_walker_callback(
    node: *mut pg_sys::Node,
    context: *mut core::ffi::c_void,
) -> bool {
    if node.is_null() {
        return false;
    }

    // We can't replace nodes during expression_tree_walker (it's read-only),
    // so we delegate to our mutating function for container nodes
    let estate = context as *mut pg_sys::EState;
    resolve_param_exec_mutator(node, estate);

    // Return false to continue walking (true would abort the walk)
    false
}

/// Walk a PostgreSQL List of expression nodes, replacing any `Param(PARAM_EXEC)` nodes
/// with `Const` nodes containing the pre-evaluated InitPlan result.
unsafe fn replace_params_in_list(list: *mut pg_sys::List, estate: *mut pg_sys::EState) {
    if list.is_null() {
        return;
    }

    let len = (*list).length as usize;
    for i in 0..len {
        let arg = pg_sys::list_nth(list, i as _) as *mut pg_sys::Node;
        if arg.is_null() {
            continue;
        }

        if (*arg).type_ == pg_sys::NodeTag::T_Param {
            if let Some(const_node) = try_resolve_param(arg as *mut pg_sys::Param, estate) {
                // Replace the list cell's value with the new Const node
                let cell_ptr = (*list).elements.add(i);
                (*cell_ptr).ptr_value = const_node.cast();
            }
        } else {
            // Recurse into non-Param children
            resolve_param_exec_mutator(arg, estate);
        }
    }
}

/// Attempt to resolve a `Param(PARAM_EXEC)` node into a `Const` node using the EState's
/// pre-computed InitPlan results.
///
/// Returns `Some(const_node)` if the param was successfully resolved, `None` otherwise
/// (e.g., if the param is not PARAM_EXEC, or the InitPlan hasn't been executed yet).
///
/// Uses `pg_sys::makeConst` to construct a properly-typed Const node, matching the
/// pattern used throughout the codebase (see `projections.rs`).
unsafe fn try_resolve_param(
    param: *mut pg_sys::Param,
    estate: *mut pg_sys::EState,
) -> Option<*mut pg_sys::Const> {
    if param.is_null() || estate.is_null() {
        return None;
    }

    // Only resolve PARAM_EXEC parameters (internal executor params from InitPlans/SubPlans)
    if (*param).paramkind != pg_sys::ParamKind::PARAM_EXEC {
        return None;
    }

    let param_id = (*param).paramid as usize;
    let param_exec_vals = (*estate).es_param_exec_vals;
    if param_exec_vals.is_null() {
        return None;
    }

    let param_data = &*param_exec_vals.add(param_id);

    // Check if the InitPlan has been executed (execPlan == NULL means result is available).
    // When execPlan is non-NULL, it means the SubPlan hasn't been executed yet.
    if !param_data.execPlan.is_null() {
        return None;
    }

    // Get type metadata for the Const node
    let mut typlen: i16 = 0;
    let mut typbyval: bool = false;
    pg_sys::get_typlenbyval((*param).paramtype, &mut typlen, &mut typbyval);

    // For pass-by-reference non-NULL values, make a copy so the Const owns its data.
    // pgrx does not expose datumCopy, so we replicate its logic here.
    let const_value = if param_data.isnull || typbyval {
        param_data.value
    } else if typlen == -1 {
        // varlena type: pg_detoast_datum returns a palloc'd copy (or the original if not toasted)
        let detoasted =
            pg_sys::pg_detoast_datum(param_data.value.cast_mut_ptr::<pg_sys::varlena>());
        pg_sys::Datum::from(detoasted)
    } else {
        // Fixed-length pass-by-reference type: palloc + memcpy
        let len = if typlen > 0 {
            typlen as usize
        } else {
            // typlen == -2 means cstring
            core::ffi::CStr::from_ptr(param_data.value.cast_mut_ptr::<core::ffi::c_char>())
                .to_bytes_with_nul()
                .len()
        };
        let dst = pg_sys::palloc(len);
        core::ptr::copy_nonoverlapping(param_data.value.cast_mut_ptr::<u8>(), dst.cast(), len);
        pg_sys::Datum::from(dst)
    };

    // Build a properly-typed Const node using the PostgreSQL API
    let const_node = pg_sys::makeConst(
        (*param).paramtype,
        (*param).paramtypmod,
        (*param).paramcollid,
        typlen as i32, // constlen: i16 → i32 (matches PostgreSQL's `int constlen`)
        const_value,
        param_data.isnull,
        typbyval,
    );

    Some(const_node)
}

/// Tantivy query that combines indexed search with heap field filtering
#[derive(Debug)]
pub struct HeapFilterQuery {
    indexed_query: Box<dyn Query>,
    field_filters: Vec<HeapFieldFilter>,
    rel_oid: pg_sys::Oid,
    expr_context: NonNull<pg_sys::ExprContext>,
    planstate: Option<NonNull<pg_sys::PlanState>>,
}

// SAFETY: PostgreSQL doesn't execute within threads despite Tantivy expecting it
unsafe impl Send for HeapFilterQuery {}
unsafe impl Sync for HeapFilterQuery {}

impl HeapFilterQuery {
    pub fn new(
        indexed_query: Box<dyn Query>,
        field_filters: Vec<HeapFieldFilter>,
        rel_oid: pg_sys::Oid,
        expr_context: NonNull<pg_sys::ExprContext>,
        planstate: Option<NonNull<pg_sys::PlanState>>,
    ) -> Self {
        Self {
            indexed_query,
            field_filters,
            rel_oid,
            expr_context,
            planstate,
        }
    }
}

impl tantivy::query::QueryClone for HeapFilterQuery {
    fn box_clone(&self) -> Box<dyn Query> {
        Box::new(Self {
            indexed_query: self.indexed_query.box_clone(),
            field_filters: self.field_filters.clone(),
            rel_oid: self.rel_oid,
            expr_context: self.expr_context,
            planstate: self.planstate,
        })
    }
}

impl Query for HeapFilterQuery {
    fn weight(&self, enable_scoring: EnableScoring) -> tantivy::Result<Box<dyn Weight>> {
        let indexed_weight = self.indexed_query.weight(enable_scoring)?;
        Ok(Box::new(HeapFilterWeight {
            indexed_weight,
            field_filters: self.field_filters.clone(),
            rel_oid: self.rel_oid,
            expr_context: self.expr_context,
            planstate: self.planstate,
        }))
    }

    fn query_terms(
        &self,
        field: Field,
        reader: &SegmentReader,
        visitor: &mut dyn for<'a> FnMut(&'a Term, bool),
    ) {
        self.indexed_query.query_terms(field, reader, visitor);
    }
}

struct HeapFilterWeight {
    indexed_weight: Box<dyn Weight>,
    field_filters: Vec<HeapFieldFilter>,
    rel_oid: pg_sys::Oid,
    expr_context: NonNull<pg_sys::ExprContext>,
    planstate: Option<NonNull<pg_sys::PlanState>>,
}

// SAFETY: PostgreSQL doesn't execute within threads despite Tantivy expecting it
unsafe impl Send for HeapFilterWeight {}
unsafe impl Sync for HeapFilterWeight {}

impl Weight for HeapFilterWeight {
    fn scorer(&self, reader: &SegmentReader, boost: Score) -> tantivy::Result<Box<dyn Scorer>> {
        let indexed_scorer = self.indexed_weight.scorer(reader, boost)?;

        // Get ctid fast field for heap access
        let fast_fields_reader = reader.fast_fields();
        let ctid_ff = crate::index::fast_fields_helper::FFType::new_ctid(fast_fields_reader);

        let scorer = HeapFilterScorer::new(
            indexed_scorer,
            self.field_filters.clone(),
            ctid_ff,
            self.rel_oid,
            self.expr_context,
            self.planstate,
        );

        Ok(Box::new(scorer))
    }

    fn explain(&self, reader: &SegmentReader, doc: DocId) -> tantivy::Result<Explanation> {
        let indexed_explanation = self.indexed_weight.explain(reader, doc)?;
        Ok(Explanation::new("HeapFilter", indexed_explanation.value()))
    }
}

struct HeapFilterScorer {
    indexed_scorer: Box<dyn Scorer>,
    field_filters: Vec<HeapFieldFilter>,
    ctid_ff: crate::index::fast_fields_helper::FFType,
    heaprel: PgSearchRelation,
    current_doc: DocId,
    expr_context: NonNull<pg_sys::ExprContext>,
    planstate: Option<NonNull<pg_sys::PlanState>>,
}

// SAFETY:  we don't execute within threads, despite Tantivy expecting that to be the case
unsafe impl Send for HeapFilterScorer {}
unsafe impl Sync for HeapFilterScorer {}

impl HeapFilterScorer {
    fn new(
        indexed_scorer: Box<dyn Scorer>,
        field_filters: Vec<HeapFieldFilter>,
        ctid_ff: crate::index::fast_fields_helper::FFType,
        rel_oid: pg_sys::Oid,
        expr_context: NonNull<pg_sys::ExprContext>,
        planstate: Option<NonNull<pg_sys::PlanState>>,
    ) -> Self {
        let mut scorer = Self {
            indexed_scorer,
            field_filters,
            ctid_ff,
            heaprel: PgSearchRelation::open(rel_oid),
            current_doc: TERMINATED,
            expr_context,
            planstate,
        };

        // Position at the first valid document
        // For initialization, we need to check the current document first, then advance if needed
        scorer.find_first_valid_document();

        scorer
    }

    fn find_first_valid_document(&mut self) {
        // For initialization, check the current document first
        self.current_doc = self.indexed_scorer.doc();

        if self.current_doc != TERMINATED && self.passes_heap_filters(self.current_doc) {
            return;
        }

        // If current document doesn't pass, advance to find the next valid one
        self.advance();
    }

    fn passes_heap_filters(&mut self, doc_id: DocId) -> bool {
        // Extract ctid from the current document
        let Some(ctid_value) = self.ctid_ff.as_u64(doc_id) else {
            panic!("Could not get ctid for doc_id: {doc_id}");
        };
        // Convert u64 ctid back to ItemPointer
        let mut item_pointer = pg_sys::ItemPointerData::default();
        crate::postgres::utils::u64_to_item_pointer(ctid_value, &mut item_pointer);

        // Evaluate all heap filters
        for filter in self.field_filters.iter_mut() {
            unsafe {
                let filter_result = filter.evaluate(
                    &mut item_pointer as *mut pg_sys::ItemPointerData,
                    &self.heaprel,
                    self.expr_context,
                    self.planstate,
                );
                if !filter_result {
                    return false;
                }
            }
        }

        true
    }
}

impl Scorer for HeapFilterScorer {
    fn score(&mut self) -> Score {
        // Return the score from the indexed query (preserving BM25 scores)
        self.indexed_scorer.score()
    }
}

impl DocSet for HeapFilterScorer {
    fn advance(&mut self) -> DocId {
        loop {
            let doc = self.indexed_scorer.advance();

            if doc == TERMINATED {
                self.current_doc = TERMINATED;
                return TERMINATED;
            }

            if self.passes_heap_filters(doc) {
                self.current_doc = doc;
                return doc;
            }
        }
    }

    fn doc(&self) -> DocId {
        self.current_doc
    }

    fn size_hint(&self) -> u32 {
        self.indexed_scorer.size_hint()
    }
}

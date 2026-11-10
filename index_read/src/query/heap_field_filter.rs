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
#![allow(clippy::missing_safety_doc, unsafe_op_in_unsafe_fn)]

use std::ptr::NonNull;

use crate::postgres::heap::HeapFetchState;
use crate::postgres::rel::PgSearchRelation;
use crate::query::PostgresPointer;
use crate::query::tid_bitmap_stream::{BitmapCell, BitmapCursor};

use pgrx::FromDatum;
use pgrx::{PgMemoryContexts, pg_sys};
use serde::{Deserialize, Serialize};
use tantivy::schema::Field;
use tantivy::{
    DocId, DocSet, Score, SeekDangerResult, SegmentReader, TERMINATED, Term,
    query::{EnableScoring, Explanation, Query, Scorer, Weight},
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
    eval_state: Option<crate::postgres::customscan::mpp::MainThread<HeapFieldFilterState>>,
}

#[derive(Debug, Default)]
struct HeapFieldFilterState {
    initialized_expression: Option<(*mut pg_sys::ExprState, Option<NonNull<pg_sys::PlanState>>)>,
    heap_fetch_state: Option<HeapFetchState>,
    scratch_context: Option<pg_sys::MemoryContext>,
}

crate::impl_safe_drop!(HeapFieldFilterState, |self| {
    if let Some(context) = self.scratch_context.take() {
        unsafe {
            if crate::postgres::utils::IsTransactionState() {
                pg_sys::MemoryContextDelete(context);
            }
        }
    }
});

impl Clone for HeapFieldFilter {
    fn clone(&self) -> Self {
        Self {
            expr_node: self.expr_node.clone(),
            heap_filter: self.heap_filter.clone(),
            eval_state: None,
        }
    }
}

impl PartialEq for HeapFieldFilter {
    fn eq(&self, other: &HeapFieldFilter) -> bool {
        self.expr_node == other.expr_node && self.heap_filter == other.heap_filter
    }
}

impl HeapFieldFilter {
    /// Create a new HeapFieldFilter from a PostgreSQL expression node
    pub unsafe fn new(expr_node: *mut pg_sys::Node, heap_filter: String) -> Self {
        Self {
            expr_node: PostgresPointer(expr_node.cast()),
            heap_filter,
            eval_state: None,
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

        crate::postgres::customscan::mpp::run_on_main_thread(|token| {
            let state = self
                .eval_state
                .get_or_insert_with(|| {
                    crate::postgres::customscan::mpp::MainThread::new(
                        HeapFieldFilterState::default(),
                    )
                })
                .get_mut(token);
            state.evaluate_expression_inner(ctid, heaprel, expr_node, expr_context, planstate)
        })
    }
}

impl HeapFieldFilterState {
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

        // Fetch the tuple and present it as a virtual slot suitable for expression
        // evaluation (see `HeapFetchState::fetch_eval_slot`).
        let Some(eval_slot) =
            heap_fetch_state.fetch_eval_slot(&mut *ctid, pg_sys::GetActiveSnapshot())
        else {
            return false;
        };

        // Store the original scan tuple to restore later if we're using a provided context
        let original_scan_tuple = (*econtext).ecxt_scantuple;
        (*econtext).ecxt_scantuple = eval_slot;

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

            let scratch_context = self.scratch_context.get_or_insert_with(|| {
                pg_sys::AllocSetContextCreateExtended(
                    (*econtext).ecxt_per_query_memory,
                    c"HeapFieldFilter scratch context".as_ptr(),
                    pg_sys::ALLOCSET_SMALL_MINSIZE as usize,
                    pg_sys::ALLOCSET_SMALL_INITSIZE as usize,
                    pg_sys::ALLOCSET_SMALL_MAXSIZE as usize,
                )
            });

            let mut scratch = PgMemoryContexts::For(*scratch_context);
            let (result, is_null) = scratch.switch_to(|_| {
                let mut is_null = false;
                let result = pg_sys::ExecEvalExpr(expr_state, econtext, &mut is_null);
                (result, is_null)
            });

            let res = bool::from_datum(result, is_null).unwrap_or(false);

            // A rejected candidate never reaches the scan's per-row reset, so the
            // expression's scratch memory is reclaimed here, or it grows per candidate.
            scratch.reset();

            res
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
}

impl HeapFieldFilter {
    /// Get the PostgreSQL expression node
    pub unsafe fn get_expression_node(&self) -> *mut pg_sys::Node {
        self.expr_node.0.cast()
    }

    /// Returns true if this filter contains any PostgreSQL parameters (like $1 or InitPlans).
    pub fn has_parameters(&self) -> bool {
        unsafe {
            let expr_node = self.expr_node.0.cast::<pg_sys::Node>();
            if expr_node.is_null() {
                return false;
            }

            #[pgrx::pg_guard]
            unsafe extern "C-unwind" fn param_walker(
                node: *mut pg_sys::Node,
                _context: *mut core::ffi::c_void,
            ) -> bool {
                if node.is_null() {
                    return false;
                }
                if (*node).type_ == pg_sys::NodeTag::T_Param {
                    return true;
                }

                pg_sys::expression_tree_walker(node, Some(param_walker), _context)
            }

            pg_sys::expression_tree_walker(expr_node, Some(param_walker), std::ptr::null_mut())
        }
    }

    /// Replaces Param nodes in the expression with Const nodes containing their evaluated values.
    /// This is required because custom parallel workers do not inherit the leader's EState.
    pub fn solve_parameters(&mut self, expr_context: *mut pg_sys::ExprContext) {
        unsafe {
            let expr_node = self.expr_node.0.cast::<pg_sys::Node>();
            if expr_node.is_null() {
                return;
            }

            #[cfg(not(any(feature = "pg16", feature = "pg17", feature = "pg18")))]
            let new_node = {
                let fnptr = param_resolver_mutator as *const ();
                let mutator: unsafe extern "C-unwind" fn() -> *mut pg_sys::Node =
                    std::mem::transmute(fnptr);
                pg_sys::expression_tree_mutator(
                    expr_node,
                    Some(mutator),
                    expr_context as *mut core::ffi::c_void,
                )
            };

            #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
            let new_node = pg_sys::expression_tree_mutator_impl(
                expr_node,
                Some(param_resolver_mutator),
                expr_context as *mut core::ffi::c_void,
            );

            self.expr_node = PostgresPointer(new_node.cast());
        }
    }
}

#[pgrx::pg_guard]
unsafe extern "C-unwind" fn param_resolver_mutator(
    node: *mut pg_sys::Node,
    context: *mut core::ffi::c_void,
) -> *mut pg_sys::Node {
    if node.is_null() {
        return std::ptr::null_mut();
    }

    if (*node).type_ == pg_sys::NodeTag::T_Param {
        let param = node.cast::<pg_sys::Param>();
        let expr_context = context.cast::<pg_sys::ExprContext>();

        let mut is_null = false;

        // Evaluate the parameter using a minimal expression state
        let expr_state = pg_sys::ExecInitExpr(node.cast(), std::ptr::null_mut());
        let result = pg_sys::ExecEvalExpr(expr_state, expr_context, &mut is_null);

        let param_type = (*param).paramtype;
        let param_typmod = (*param).paramtypmod;
        let param_collid = (*param).paramcollid;

        let mut typlen = 0;
        let mut typbyval = false;
        pg_sys::get_typlenbyval(param_type, &mut typlen, &mut typbyval);

        let const_node = pg_sys::makeConst(
            param_type,
            param_typmod,
            param_collid,
            typlen.into(),
            result,
            is_null,
            typbyval,
        );

        return const_node.cast();
    }

    #[cfg(not(any(feature = "pg16", feature = "pg17", feature = "pg18")))]
    {
        let fnptr = param_resolver_mutator as *const ();
        let mutator: unsafe extern "C-unwind" fn() -> *mut pg_sys::Node =
            std::mem::transmute(fnptr);
        pg_sys::expression_tree_mutator(node, Some(mutator), context)
    }

    #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
    {
        pg_sys::expression_tree_mutator_impl(node, Some(param_resolver_mutator), context)
    }
}
pub enum TidProbe {
    /// Not in the bitmap: the row cannot match, skip it with zero heap access.
    Reject,
    /// In the bitmap with exact, non-recheck membership.
    Candidate,
    /// Membership is lossy or flagged: recheck filters must also be evaluated.
    NeedsRecheck,
}

/// Tantivy query that combines indexed search with heap field filtering
#[derive(Debug)]
pub struct HeapFilterQuery(crate::postgres::customscan::mpp::MainThread<HeapFilterQueryInner>);

#[derive(Debug)]
struct HeapFilterQueryInner {
    indexed_query: Box<dyn Query>,
    always_filters: Vec<HeapFieldFilter>,
    recheck_filters: Vec<HeapFieldFilter>,
    bitmap_consumer_id: Option<u32>,
    bitmap_cell: Option<BitmapCell>,
    rel_oid: pg_sys::Oid,
    expr_context: NonNull<pg_sys::ExprContext>,
    planstate: Option<NonNull<pg_sys::PlanState>>,
}

impl HeapFilterQuery {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        indexed_query: Box<dyn Query>,
        always_filters: Vec<HeapFieldFilter>,
        recheck_filters: Vec<HeapFieldFilter>,
        bitmap_consumer_id: Option<u32>,
        bitmap_cell: Option<BitmapCell>,
        rel_oid: pg_sys::Oid,
        expr_context: NonNull<pg_sys::ExprContext>,
        planstate: Option<NonNull<pg_sys::PlanState>>,
    ) -> Self {
        Self(crate::postgres::customscan::mpp::MainThread::new(
            HeapFilterQueryInner {
                indexed_query,
                always_filters,
                recheck_filters,
                bitmap_consumer_id,
                bitmap_cell,
                rel_oid,
                expr_context,
                planstate,
            },
        ))
    }
}

impl tantivy::query::QueryClone for HeapFilterQuery {
    fn box_clone(&self) -> Box<dyn Query> {
        crate::postgres::customscan::mpp::run_on_main_thread(|token| {
            let inner = self.0.get(token);
            Box::new(Self::new(
                inner.indexed_query.box_clone(),
                inner.always_filters.clone(),
                inner.recheck_filters.clone(),
                inner.bitmap_consumer_id,
                inner.bitmap_cell.clone(),
                inner.rel_oid,
                inner.expr_context,
                inner.planstate,
            ))
        })
    }
}

impl Query for HeapFilterQuery {
    fn weight(&self, enable_scoring: EnableScoring) -> tantivy::Result<Box<dyn Weight>> {
        crate::postgres::customscan::mpp::run_on_main_thread(|token| {
            let inner = self.0.get(token);
            let indexed_weight = inner.indexed_query.weight(enable_scoring)?;
            Ok(Box::new(HeapFilterWeight(
                crate::postgres::customscan::mpp::MainThread::new(HeapFilterWeightInner {
                    indexed_weight,
                    always_filters: inner.always_filters.clone(),
                    recheck_filters: inner.recheck_filters.clone(),
                    bitmap_consumer_id: inner.bitmap_consumer_id,
                    bitmap_cell: inner.bitmap_cell.clone(),
                    rel_oid: inner.rel_oid,
                    expr_context: inner.expr_context,
                    planstate: inner.planstate,
                }),
            )) as Box<dyn Weight>)
        })
    }

    fn query_terms(
        &self,
        field: Field,
        reader: &SegmentReader,
        visitor: &mut dyn for<'a> FnMut(&'a Term, bool),
    ) {
        crate::postgres::customscan::mpp::run_on_main_thread(|token| {
            self.0
                .get(token)
                .indexed_query
                .query_terms(field, reader, visitor);
        });
    }
}

struct HeapFilterWeight(crate::postgres::customscan::mpp::MainThread<HeapFilterWeightInner>);

struct HeapFilterWeightInner {
    indexed_weight: Box<dyn Weight>,
    always_filters: Vec<HeapFieldFilter>,
    recheck_filters: Vec<HeapFieldFilter>,
    bitmap_consumer_id: Option<u32>,
    bitmap_cell: Option<BitmapCell>,
    rel_oid: pg_sys::Oid,
    expr_context: NonNull<pg_sys::ExprContext>,
    planstate: Option<NonNull<pg_sys::PlanState>>,
}

impl Weight for HeapFilterWeight {
    fn scorer(&self, reader: &SegmentReader, boost: Score) -> tantivy::Result<Box<dyn Scorer>> {
        crate::postgres::customscan::mpp::run_on_main_thread(|token| {
            let inner = self.0.get(token);
            let indexed_scorer = inner.indexed_weight.scorer(reader, boost)?;

            // Get ctid fast field for heap access
            let fast_fields_reader = reader.fast_fields();
            let ctid_ff = crate::index::fast_fields_helper::FFType::new_ctid(fast_fields_reader);

            // Claim this (consumer, segment) stream's cursor. No cell installed means
            // no bitmap was planned (or this is an estimation clone): evaluate filters
            // directly. An installed-but-unfilled cell is an invariant violation.
            let cursor = match (inner.bitmap_consumer_id, &inner.bitmap_cell) {
                (Some(consumer_id), Some(cell)) => {
                    let source = cell
                        .get()
                        .unwrap_or_else(|| panic!("bitmap cursor source was never initialized"));
                    Some(unsafe { source.claim(consumer_id, reader.segment_id()) })
                }
                _ => None,
            };

            let scorer = HeapFilterScorer::new(
                indexed_scorer,
                inner.always_filters.clone(),
                inner.recheck_filters.clone(),
                cursor,
                ctid_ff,
                inner.rel_oid,
                inner.expr_context,
                inner.planstate,
            );

            Ok(Box::new(scorer) as Box<dyn Scorer>)
        })
    }

    fn explain(&self, reader: &SegmentReader, doc: DocId) -> tantivy::Result<Explanation> {
        crate::postgres::customscan::mpp::run_on_main_thread(|token| {
            let indexed_explanation = self.0.get(token).indexed_weight.explain(reader, doc)?;
            Ok(Explanation::new("HeapFilter", indexed_explanation.value()))
        })
    }
}

struct HeapFilterScorer {
    inner: crate::postgres::customscan::mpp::MainThread<HeapFilterScorerInner>,
    current_doc: DocId,
    size_hint: u32,
}

struct HeapFilterScorerInner {
    indexed_scorer: Box<dyn Scorer>,
    /// Evaluated on every doc that survives the bitmap probe (or every doc when no
    /// bitmap set is attached). See `SearchQueryInput::HeapFilter`.
    always_filters: Vec<HeapFieldFilter>,
    /// Evaluated only for lossy/recheck probes, or when no bitmap cursor is attached.
    recheck_filters: Vec<HeapFieldFilter>,
    bitmap_cursor: Option<BitmapCursor>,
    ctid_ff: crate::index::fast_fields_helper::FFType,
    heaprel: PgSearchRelation,
    current_doc: DocId,
    /// `seek_danger` left `current_doc` on a document the heap filters have not accepted. The
    /// `DocSet` contract then allows only further `seek_danger` calls until one returns `Found`.
    in_danger: bool,
    expr_context: NonNull<pg_sys::ExprContext>,
    planstate: Option<NonNull<pg_sys::PlanState>>,
}

impl HeapFilterScorer {
    #[allow(clippy::too_many_arguments)]
    fn new(
        indexed_scorer: Box<dyn Scorer>,
        always_filters: Vec<HeapFieldFilter>,
        recheck_filters: Vec<HeapFieldFilter>,
        bitmap_cursor: Option<BitmapCursor>,
        ctid_ff: crate::index::fast_fields_helper::FFType,
        rel_oid: pg_sys::Oid,
        expr_context: NonNull<pg_sys::ExprContext>,
        planstate: Option<NonNull<pg_sys::PlanState>>,
    ) -> Self {
        let size_hint = indexed_scorer.size_hint();
        let mut inner = HeapFilterScorerInner {
            indexed_scorer,
            always_filters,
            recheck_filters,
            bitmap_cursor,
            ctid_ff,
            heaprel: PgSearchRelation::open(rel_oid),
            current_doc: TERMINATED,
            in_danger: false,
            expr_context,
            planstate,
        };

        // Position at the first valid document
        // For initialization, we need to check the current document first, then advance if needed
        inner.find_first_valid_document();
        let current_doc = inner.current_doc;

        Self {
            inner: crate::postgres::customscan::mpp::MainThread::new(inner),
            current_doc,
            size_hint,
        }
    }
}

impl HeapFilterScorerInner {
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

        // Probe the external index's bitmap first: a miss rejects the doc with zero
        // heap access, and exact non-recheck membership skips the recheck filters.
        // With no cursor attached, both filter lists must run: the bitmap proof
        // the recheck split relies on doesn't exist.
        let needs_recheck_filters = match &mut self.bitmap_cursor {
            Some(cursor) => match unsafe { cursor.probe(ctid_value) } {
                TidProbe::Reject => return false,
                TidProbe::Candidate => false,
                TidProbe::NeedsRecheck => true,
            },
            None => true,
        };

        let run_recheck = needs_recheck_filters && !self.recheck_filters.is_empty();
        if self.always_filters.is_empty() && !run_recheck {
            return true;
        }

        // Convert u64 ctid back to ItemPointer
        let mut item_pointer = pg_sys::ItemPointerData::default();
        crate::postgres::utils::u64_to_item_pointer(ctid_value, &mut item_pointer);

        let mut no_filters = Vec::new();
        let recheck_filters = if run_recheck {
            &mut self.recheck_filters
        } else {
            &mut no_filters
        };
        for filter in self
            .always_filters
            .iter_mut()
            .chain(recheck_filters.iter_mut())
        {
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

    fn advance(&mut self) -> DocId {
        self.in_danger = false;
        loop {
            if crate::postgres::customscan::mpp::check_for_interrupts_or_cancel() {
                self.current_doc = TERMINATED;
                return TERMINATED;
            }

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

    fn seek(&mut self, target: DocId) -> DocId {
        if !self.in_danger && self.current_doc >= target {
            return self.current_doc;
        }
        self.in_danger = false;
        let doc = self.indexed_scorer.seek(target);
        if doc == TERMINATED || self.passes_heap_filters(doc) {
            self.current_doc = doc;
            return doc;
        }
        self.advance()
    }

    fn seek_danger(&mut self, target: DocId) -> SeekDangerResult {
        if target >= TERMINATED {
            return SeekDangerResult::SeekLowerBound(TERMINATED);
        }
        if !self.in_danger && self.current_doc >= target {
            return if self.current_doc == target {
                SeekDangerResult::Found
            } else {
                SeekDangerResult::SeekLowerBound(self.current_doc)
            };
        }
        match self.indexed_scorer.seek_danger(target) {
            SeekDangerResult::Found => {
                self.current_doc = target;
                if self.passes_heap_filters(target) {
                    self.in_danger = false;
                    SeekDangerResult::Found
                } else {
                    self.in_danger = true;
                    SeekDangerResult::SeekLowerBound(target + 1)
                }
            }
            SeekDangerResult::SeekLowerBound(bound) => {
                self.current_doc = bound;
                self.in_danger = true;
                SeekDangerResult::SeekLowerBound(bound)
            }
        }
    }
}

impl Scorer for HeapFilterScorer {
    fn score(&mut self) -> Score {
        crate::postgres::customscan::mpp::run_on_main_thread(|token| {
            self.inner.get_mut(token).indexed_scorer.score()
        })
    }
}

impl DocSet for HeapFilterScorer {
    fn advance(&mut self) -> DocId {
        let doc = crate::postgres::customscan::mpp::run_on_main_thread(|token| {
            self.inner.get_mut(token).advance()
        });
        self.current_doc = doc;
        doc
    }

    // The default `seek` advances one document at a time and evaluates the heap filters on
    // each, so an intersection seeking this scorer past the documents its driver skipped would
    // fetch every one of them from the heap.
    fn seek(&mut self, target: DocId) -> DocId {
        let doc = crate::postgres::customscan::mpp::run_on_main_thread(|token| {
            self.inner.get_mut(token).seek(target)
        });
        self.current_doc = doc;
        doc
    }

    // A probe at a rejected document answers with a lower bound instead of walking the
    // indexed scorer (and the heap) to the next accepted one, which the caller may never ask
    // for.
    fn seek_danger(&mut self, target: DocId) -> SeekDangerResult {
        let (res, doc) = crate::postgres::customscan::mpp::run_on_main_thread(|token| {
            let inner = self.inner.get_mut(token);
            let res = inner.seek_danger(target);
            (res, inner.current_doc)
        });
        self.current_doc = doc;
        res
    }

    fn doc(&self) -> DocId {
        self.current_doc
    }

    fn size_hint(&self) -> u32 {
        self.size_hint
    }
}

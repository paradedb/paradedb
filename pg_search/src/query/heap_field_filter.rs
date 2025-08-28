use crate::postgres::customscan::qual_inspect::contains_exec_param;
use crate::postgres::rel::PgSearchRelation;
use crate::query::PostgresPointer;
use pgrx::FromDatum;
use pgrx::{pg_sys, PgMemoryContexts};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use tantivy::{
    query::{EnableScoring, Explanation, Query, Scorer, Weight},
    DocId, DocSet, Score, SegmentReader, TERMINATED,
};

thread_local! {
    /// Thread-local storage for the runtime expression context
    /// This allows heap filters to access the proper context for subquery evaluation
    static RUNTIME_EXPR_CONTEXT: RefCell<*mut pg_sys::ExprContext> = const { RefCell::new(std::ptr::null_mut()) };

    /// Thread-local storage for the planstate
    /// This allows expressions to be properly initialized with subplan support
    static RUNTIME_PLANSTATE: RefCell<*mut pg_sys::PlanState> = const { RefCell::new(std::ptr::null_mut()) };
}

/// Set the runtime expression context and planstate for the current thread
/// This should be called before executing queries that may use heap filters
pub fn set_runtime_context(context: *mut pg_sys::ExprContext, planstate: *mut pg_sys::PlanState) {
    RUNTIME_EXPR_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = context;
    });
    RUNTIME_PLANSTATE.with(|ps| {
        *ps.borrow_mut() = planstate;
    });
}

/// Clear the runtime context for the current thread
pub fn clear_runtime_context() {
    set_runtime_context(std::ptr::null_mut(), std::ptr::null_mut());
}

/// Get the current runtime expression context
fn get_runtime_expr_context() -> *mut pg_sys::ExprContext {
    RUNTIME_EXPR_CONTEXT.with(|ctx| *ctx.borrow())
}

/// Get the current runtime planstate
fn get_runtime_planstate() -> *mut pg_sys::PlanState {
    RUNTIME_PLANSTATE.with(|ps| *ps.borrow())
}

/// Core heap-based field filter using PostgreSQL expression evaluation
/// This approach stores a serialized representation of the PostgreSQL expression
/// and evaluates it directly against heap tuples, supporting any PostgreSQL operator or function
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HeapFieldFilter {
    /// PostgreSQL expression node that can be serialized and reconstructed
    expr_node: PostgresPointer,
    /// Human-readable description of the expression
    pub description: String,

    #[serde(skip)]
    initialized_expression: Option<*mut pg_sys::ExprState>,
    #[serde(skip)]
    initialized_with_planstate: bool,
}

// SAFETY:  we don't execute within threads, despite Tantivy expecting that to be the case
unsafe impl Send for HeapFieldFilter {}
unsafe impl Sync for HeapFieldFilter {}

impl HeapFieldFilter {
    /// Create a new HeapFieldFilter from a PostgreSQL expression node
    pub unsafe fn new(expr_node: *mut pg_sys::Node, expr_desc: String) -> Self {
        Self {
            expr_node: PostgresPointer(expr_node.cast()),
            description: expr_desc,
            initialized_expression: None,
            initialized_with_planstate: false,
        }
    }

    /// Evaluate this filter against a heap tuple identified by ctid
    /// Uses PostgreSQL's expression evaluation system
    pub unsafe fn evaluate(
        &mut self,
        ctid: pg_sys::ItemPointer,
        heaprel: &PgSearchRelation,
    ) -> bool {
        // Use the runtime expression context if available
        let runtime_context = get_runtime_expr_context();
        self.evaluate_with_context(ctid, heaprel, runtime_context)
    }

    /// Evaluate this filter with a specific expression context (supports subqueries)
    pub unsafe fn evaluate_with_context(
        &mut self,
        ctid: pg_sys::ItemPointer,
        heaprel: &PgSearchRelation,
        expr_context: *mut pg_sys::ExprContext,
    ) -> bool {
        // Get the expression node
        let expr_node = self.expr_node.0.cast::<pg_sys::Node>();
        if expr_node.is_null() {
            return true;
        }

        self.evaluate_expression_inner(ctid, heaprel, expr_node, expr_context)
    }

    /// Inner expression evaluation method that can be wrapped in panic handling
    unsafe fn evaluate_expression_inner(
        &mut self,
        ctid: pg_sys::ItemPointer,
        relation: &PgSearchRelation,
        expr_node: *mut pg_sys::Node,
        provided_expr_context: *mut pg_sys::ExprContext,
    ) -> bool {
        // Use heap_fetch to safely get the tuple
        let mut heap_tuple = pg_sys::HeapTupleData {
            t_len: 0,
            t_self: *ctid, // Set the ctid we want to fetch
            t_tableOid: relation.oid(),
            t_data: std::ptr::null_mut(),
        };
        let mut buffer = pg_sys::InvalidBuffer as pg_sys::Buffer;

        // Fetch the heap tuple using PostgreSQL's heap_fetch API
        // Function signature differs between PostgreSQL versions
        #[cfg(feature = "pg14")]
        let valid_tuple = pg_sys::heap_fetch(
            relation.as_ptr(),
            pgrx::pg_sys::GetActiveSnapshot(),
            &mut heap_tuple,
            &mut buffer,
        );

        #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
        let valid_tuple = pg_sys::heap_fetch(
            relation.as_ptr(),
            pgrx::pg_sys::GetActiveSnapshot(),
            &mut heap_tuple,
            &mut buffer,
            false, // keep_buf
        );

        if !valid_tuple {
            if buffer != pg_sys::InvalidBuffer as pg_sys::Buffer {
                pg_sys::ReleaseBuffer(buffer);
            }
            return false;
        }

        // Create a tuple table slot for expression evaluation
        let tuple_desc = relation.rd_att;
        let slot = pg_sys::MakeTupleTableSlot(tuple_desc, &pg_sys::TTSOpsHeapTuple);
        if slot.is_null() {
            if buffer != pg_sys::InvalidBuffer as pg_sys::Buffer {
                pg_sys::ReleaseBuffer(buffer);
            }
            return false;
        }

        // Store the heap tuple in the slot
        let stored_slot = pg_sys::ExecStoreHeapTuple(&mut heap_tuple, slot, false);
        if stored_slot.is_null() {
            pg_sys::ExecDropSingleTupleTableSlot(slot);
            if buffer != pg_sys::InvalidBuffer as pg_sys::Buffer {
                pg_sys::ReleaseBuffer(buffer);
            }
            return false;
        }

        // Use provided expression context if available, otherwise create a standalone one
        let (econtext, should_free_context) = if !provided_expr_context.is_null() {
            // Use the provided context (supports subqueries)
            (provided_expr_context, false)
        } else {
            // Create a standalone context (fallback for simple expressions)
            let standalone_context = pg_sys::CreateStandaloneExprContext();
            if standalone_context.is_null() {
                pg_sys::ExecDropSingleTupleTableSlot(slot);
                if buffer != pg_sys::InvalidBuffer as pg_sys::Buffer {
                    pg_sys::ReleaseBuffer(buffer);
                }
                return false;
            }
            (standalone_context, true)
        };

        // Store the original scan tuple to restore later if we're using a provided context
        let original_scan_tuple = if !provided_expr_context.is_null() {
            (*econtext).ecxt_scantuple
        } else {
            std::ptr::null_mut()
        };

        // Set the tuple slot in the expression context
        (*econtext).ecxt_scantuple = slot;

        // Initialize the expression for execution with proper planstate for subquery support
        let planstate = get_runtime_planstate();
        let has_planstate = !planstate.is_null();

        // Check if we need to reinitialize with a better planstate
        let expr_state = if let Some(existing_state) = self.initialized_expression {
            if has_planstate && !self.initialized_with_planstate {
                // We now have a planstate but were initialized without one, reinitialize
                let new_state = PgMemoryContexts::TopTransactionContext
                    .switch_to(|_| pg_sys::ExecInitExpr(expr_node.cast(), planstate));
                self.initialized_expression = Some(new_state);
                self.initialized_with_planstate = true;
                new_state
            } else {
                existing_state
            }
        } else {
            // First initialization
            let new_state = PgMemoryContexts::TopTransactionContext.switch_to(|_| {
                if has_planstate {
                    pg_sys::ExecInitExpr(expr_node.cast(), planstate)
                } else {
                    pg_sys::ExecInitExpr(expr_node.cast(), std::ptr::null_mut())
                }
            });
            self.initialized_expression = Some(new_state);
            self.initialized_with_planstate = has_planstate;
            new_state
        };
        if expr_state.is_null() {
            self.initialized_expression = None;
            // Restore original scan tuple if we're using a provided context
            if !provided_expr_context.is_null() {
                (*econtext).ecxt_scantuple = original_scan_tuple;
            }
            // Only free the context if we created it ourselves
            if should_free_context {
                pg_sys::FreeExprContext(econtext, false);
            }
            pg_sys::ExecDropSingleTupleTableSlot(slot);
            if buffer != pg_sys::InvalidBuffer as pg_sys::Buffer {
                pg_sys::ReleaseBuffer(buffer);
            }
            return false;
        }

        // Evaluate the expression
        let mut is_null = false;
        let result = pg_sys::ExecEvalExpr(expr_state, econtext, &mut is_null);

        // Convert the result to a boolean
        let eval_result = bool::from_datum(result, is_null).unwrap_or(false);

        // Cleanup resources in reverse order
        // Restore original scan tuple if we're using a provided context
        if !provided_expr_context.is_null() {
            (*econtext).ecxt_scantuple = original_scan_tuple;
        }

        // Only free the context if we created it ourselves
        if should_free_context {
            pg_sys::FreeExprContext(econtext, false);
        }

        pg_sys::ExecDropSingleTupleTableSlot(slot);
        if buffer != pg_sys::InvalidBuffer as pg_sys::Buffer {
            pg_sys::ReleaseBuffer(buffer);
        }

        eval_result
    }

    /// Get the PostgreSQL expression node
    pub unsafe fn get_expression_node(&self) -> *mut pg_sys::Node {
        self.expr_node.0.cast()
    }

    /// Check if this heap filter contains subqueries (PARAM_EXEC nodes)
    pub fn contains_subqueries(&self) -> bool {
        unsafe {
            let expr_node = self.expr_node.0.cast::<pg_sys::Node>();
            if expr_node.is_null() {
                return false;
            }
            contains_exec_param(expr_node)
        }
    }

    // The new expression-based approach handles evaluation directly
}

/// Tantivy query that combines indexed search with heap field filtering
#[derive(Debug)]
pub struct HeapFilterQuery {
    indexed_query: Box<dyn Query>,
    field_filters: Vec<HeapFieldFilter>,
    rel_oid: pg_sys::Oid,
}

impl HeapFilterQuery {
    pub fn new(
        indexed_query: Box<dyn Query>,
        field_filters: Vec<HeapFieldFilter>,
        rel_oid: pg_sys::Oid,
    ) -> Self {
        Self {
            indexed_query,
            field_filters,
            rel_oid,
        }
    }
}

impl tantivy::query::QueryClone for HeapFilterQuery {
    fn box_clone(&self) -> Box<dyn Query> {
        Box::new(Self {
            indexed_query: self.indexed_query.box_clone(),
            field_filters: self.field_filters.clone(),
            rel_oid: self.rel_oid,
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
        }))
    }
}

struct HeapFilterWeight {
    indexed_weight: Box<dyn Weight>,
    field_filters: Vec<HeapFieldFilter>,
    rel_oid: pg_sys::Oid,
}

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
    ) -> Self {
        let mut scorer = Self {
            indexed_scorer,
            field_filters,
            ctid_ff,
            heaprel: PgSearchRelation::open(rel_oid),
            current_doc: TERMINATED,
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
        let ctid_value = self.ctid_ff.as_u64(doc_id);
        if ctid_value.is_none() {
            panic!("Could not get ctid for doc_id: {doc_id}");
        }
        let ctid_value = ctid_value.unwrap();
        // Convert u64 ctid back to ItemPointer
        let mut item_pointer = pg_sys::ItemPointerData::default();
        crate::postgres::utils::u64_to_item_pointer(ctid_value, &mut item_pointer);

        // Evaluate all heap filters
        for filter in self.field_filters.iter_mut() {
            unsafe {
                let filter_result = filter.evaluate(
                    &mut item_pointer as *mut pg_sys::ItemPointerData,
                    &self.heaprel,
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

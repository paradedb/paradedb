use pgrx::{pg_sys, PgRelation, PgTupleDesc};
use pgrx::heap_tuple::PgHeapTuple;
use crate::api::SearchQueryInput;

/// Create a HeapExpr from a PostgreSQL expression node
/// This approach stores the original PostgreSQL expression and evaluates it
/// directly against heap tuples, supporting any PostgreSQL operator or function
pub unsafe fn try_create_heap_expr_from_node(
    expr_node: *mut pg_sys::Node,
    search_query_input: Box<SearchQueryInput>,
) -> Option<crate::postgres::customscan::pdbscan::qual_inspect::Qual> {
    // Create a description of the expression for debugging
    let expr_description = format!("PostgreSQL expression at {:p}", expr_node);
    
    Some(crate::postgres::customscan::pdbscan::qual_inspect::Qual::HeapExpr {
        expr_node,
        expr_description,
        search_query_input,
    })
}

/// Evaluate a PostgreSQL expression against a heap tuple
/// Returns Some(bool) for boolean results, None for NULL
pub unsafe fn evaluate_postgres_expression_against_tuple(
    expr_node: *mut pg_sys::Node,
    ctid: pg_sys::ItemPointer,
    relation_oid: pg_sys::Oid,
    expr_description: &str,
) -> Option<bool> {

    // Open the relation
    let heaprel = PgRelation::open(relation_oid);
    let ipd = *ctid;

    // Create HeapTupleData structure
    let mut htup = pg_sys::HeapTupleData {
        t_self: ipd,
        ..Default::default()
    };
    let mut buffer: pg_sys::Buffer = pg_sys::InvalidBuffer as i32;

    // Fetch the heap tuple
    #[cfg(feature = "pg14")]
    let fetch_success = pg_sys::heap_fetch(
        heaprel.as_ptr(),
        pg_sys::GetActiveSnapshot(),
        &mut htup,
        &mut buffer,
    );

    #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
    let fetch_success = pg_sys::heap_fetch(
        heaprel.as_ptr(),
        pg_sys::GetActiveSnapshot(),
        &mut htup,
        &mut buffer,
        false,
    );

    if !fetch_success {
        if buffer != (pg_sys::InvalidBuffer as i32) {
            pg_sys::ReleaseBuffer(buffer);
        }
        return None;
    }

    // Create an expression state for evaluation
    let expr_state = pg_sys::ExecInitExpr(expr_node, std::ptr::null_mut());
    if expr_state.is_null() {
        if buffer != (pg_sys::InvalidBuffer as i32) {
            pg_sys::ReleaseBuffer(buffer);
        }
        return None;
    }

    // Create an expression context
    let expr_context = pg_sys::CreateStandaloneExprContext();
    if expr_context.is_null() {
        if buffer != (pg_sys::InvalidBuffer as i32) {
            pg_sys::ReleaseBuffer(buffer);
        }
        return None;
    }

    // Set up the tuple table slot for the current tuple
    let tuple_desc = heaprel.rd_att;
    let slot = pg_sys::MakeTupleTableSlot(tuple_desc, &pg_sys::TTSOpsHeapTuple);
    if slot.is_null() {
        pg_sys::FreeExprContext(expr_context, false);
        if buffer != (pg_sys::InvalidBuffer as i32) {
            pg_sys::ReleaseBuffer(buffer);
        }
        return None;
    }

    // Store the tuple in the slot
    pg_sys::ExecStoreHeapTuple(&mut htup, slot, false);

    // Set the slot in the expression context
    (*expr_context).ecxt_scantuple = slot;

    // Evaluate the expression
    let mut is_null = false;
    let result_datum = pg_sys::ExecEvalExpr(expr_state, expr_context, &mut is_null);

    // Clean up
    pg_sys::ExecDropSingleTupleTableSlot(slot);
    pg_sys::FreeExprContext(expr_context, false);
    if buffer != (pg_sys::InvalidBuffer as i32) {
        pg_sys::ReleaseBuffer(buffer);
    }

    if is_null {
        None
    } else {
        // Convert datum to boolean
        let result = pgrx::FromDatum::from_datum(result_datum, false)
            .unwrap_or(false);
        Some(result)
    }
} 

use crate::customscan::port::tuptable_h::ExecClearTuple;
use pgrx::pg_sys::{
    uint16, AttrNumber, Datum, ExprContext, ExprState, MemoryContextSwitchTo, ProjectionInfo,
    TupleTableSlot, TTS_FLAG_EMPTY,
};

/// ```c
/// static inline TupleTableSlot *
/// ExecProject(ProjectionInfo *projInfo)
/// {
/// 	ExprContext *econtext = projInfo->pi_exprContext;
/// 	ExprState  *state = &projInfo->pi_state;
/// 	TupleTableSlot *slot = state->resultslot;
/// 	bool		isnull;
///
/// 	/*
/// 	 * Clear any former contents of the result slot.  This makes it safe for
/// 	 * us to use the slot's Datum/isnull arrays as workspace.
/// 	 */
/// 	ExecClearTuple(slot);
///
/// 	/* Run the expression, discarding scalar result from the last column. */
/// 	(void) ExecEvalExprSwitchContext(state, econtext, &isnull);
///
/// 	/*
/// 	 * Successfully formed a result row.  Mark the result slot as containing a
/// 	 * valid virtual tuple (inlined version of ExecStoreVirtualTuple()).
/// 	 */
/// 	slot->tts_flags &= ~TTS_FLAG_EMPTY;
/// 	slot->tts_nvalid = slot->tts_tupleDescriptor->natts;
///
/// 	return slot;
/// }
/// ```
pub unsafe fn ExecProject(projInfo: *mut ProjectionInfo) -> *mut TupleTableSlot {
    let econtext = (*projInfo).pi_exprContext;
    let state = &mut (*projInfo).pi_state;
    let slot = state.resultslot;
    let mut isnull = false;

    ExecClearTuple(slot);

    ExecEvalExprSwitchContext(state, econtext, &mut isnull);

    (*slot).tts_flags &= !TTS_FLAG_EMPTY as uint16;
    (*slot).tts_nvalid = (*(*slot).tts_tupleDescriptor).natts as AttrNumber;

    slot
}

/// ```c
/// static inline Datum
/// ExecEvalExprSwitchContext(ExprState *state,
/// 						  ExprContext *econtext,
/// 						  bool *isNull)
/// {
/// 	Datum		retDatum;
/// 	MemoryContext oldContext;
///
/// 	oldContext = MemoryContextSwitchTo(econtext->ecxt_per_tuple_memory);
/// 	retDatum = state->evalfunc(state, econtext, isNull);
/// 	MemoryContextSwitchTo(oldContext);
/// 	return retDatum;
/// }
/// ```
pub unsafe fn ExecEvalExprSwitchContext(
    state: *mut ExprState,
    econtext: *mut ExprContext,
    isNull: *mut bool,
) -> Datum {
    let oldContext = MemoryContextSwitchTo((*econtext).ecxt_per_tuple_memory);
    let retDatum = (*state).evalfunc.as_ref().unwrap()(state, econtext, isNull);
    MemoryContextSwitchTo(oldContext);
    retDatum
}

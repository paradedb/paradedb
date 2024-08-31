use pgrx::pg_sys::TupleTableSlot;

/// ```c
/// static inline TupleTableSlot *
/// ExecClearTuple(TupleTableSlot *slot)
/// {
/// 	slot->tts_ops->clear(slot);
///
/// 	return slot;
/// }
/// ```
pub unsafe fn ExecClearTuple(slot: *mut TupleTableSlot) -> *mut TupleTableSlot {
    (*(*slot).tts_ops).clear.as_ref().unwrap()(slot);
    slot
}

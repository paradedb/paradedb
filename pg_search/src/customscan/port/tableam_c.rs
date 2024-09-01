use pgrx::pg_sys;
use std::ffi::c_char;

/// ```c
/// const TupleTableSlotOps *
/// table_slot_callbacks(Relation relation)
/// {
/// 	const TupleTableSlotOps *tts_cb;
///
/// 	if (relation->rd_tableam)
/// 		tts_cb = relation->rd_tableam->slot_callbacks(relation);
/// 	else if (relation->rd_rel->relkind == RELKIND_FOREIGN_TABLE)
/// 	{
/// 		/*
/// 		 * Historically FDWs expect to store heap tuples in slots. Continue
/// 		 * handing them one, to make it less painful to adapt FDWs to new
/// 		 * versions. The cost of a heap slot over a virtual slot is pretty
/// 		 * small.
/// 		 */
/// 		tts_cb = &TTSOpsHeapTuple;
/// 	}
/// 	else
/// 	{
/// 		/*
/// 		 * These need to be supported, as some parts of the code (like COPY)
/// 		 * need to create slots for such relations too. It seems better to
/// 		 * centralize the knowledge that a heap slot is the right thing in
/// 		 * that case here.
/// 		 */
/// 		Assert(relation->rd_rel->relkind == RELKIND_VIEW ||
/// 			   relation->rd_rel->relkind == RELKIND_PARTITIONED_TABLE);
/// 		tts_cb = &TTSOpsVirtual;
/// 	}
///
/// 	return tts_cb;
/// }
/// ```
pub fn table_slot_callbacks(relation: pg_sys::Relation) -> *const pg_sys::TupleTableSlotOps {
    static mut TTS_CB: *const pg_sys::TupleTableSlotOps = std::ptr::null_mut();

    unsafe {
        if !(*relation).rd_tableam.is_null() {
            TTS_CB = (*(*relation).rd_tableam).slot_callbacks.as_ref().unwrap()(relation);
        } else if (*(*relation).rd_rel).relkind == pg_sys::RELKIND_FOREIGN_TABLE as c_char {
            TTS_CB = &pg_sys::TTSOpsHeapTuple;
        } else {
            debug_assert!(
                (*(*relation).rd_rel).relkind == pg_sys::RELKIND_VIEW as c_char
                    || (*(*relation).rd_rel).relkind == pg_sys::RELKIND_PARTITIONED_TABLE as c_char
            );
            TTS_CB = &pg_sys::TTSOpsVirtual;
        }

        TTS_CB
    }
}

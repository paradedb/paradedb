use pgrx::*;
use std::mem::size_of;

static INVALID_SUBTRANSACTION_ID: pg_sys::SubTransactionId = 0;
pub static SIZEOF_HEAP_TUPLE_HEADER: usize = size_of::<pg_sys::HeapTupleHeaderData>();

unsafe fn xlog_is_needed() -> bool {
    pg_sys::wal_level >= pg_sys::WalLevel_WAL_LEVEL_REPLICA as i32
}

unsafe fn relation_is_permanent(rel: pg_sys::Relation) -> bool {
    (*(*rel).rd_rel).relpersistence == pg_sys::RELPERSISTENCE_PERMANENT as i8
}

/// # Safety
/// This function is unsafe because it calls pg_sys functions
pub unsafe fn relation_needs_wal(rel: pg_sys::Relation) -> bool {
    relation_is_permanent(rel)
        && (xlog_is_needed()
            || ((*rel).rd_createSubid == INVALID_SUBTRANSACTION_ID
                && (*rel).rd_firstRelfilelocatorSubid == INVALID_SUBTRANSACTION_ID))
}

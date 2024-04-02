use async_std::task;
use pgrx::*;
use shared::postgres::transaction::{Transaction, TransactionError};
use std::panic::AssertUnwindSafe;
use thiserror::Error;

use crate::datafusion::writer::{Writer, TRANSACTION_CALLBACK_CACHE_ID};
use crate::hooks::handler::{HandlerError, IsColumn};

pub fn insert(
    rtable: *mut pg_sys::List,
    _query_desc: PgBox<pg_sys::QueryDesc>,
) -> Result<(), InsertHookError> {
    let rte: *mut pg_sys::RangeTblEntry;

    #[cfg(feature = "pg12")]
    {
        let current_cell = unsafe { (*rtable).head };
        rte = unsafe { (*current_cell).data.ptr_value as *mut pg_sys::RangeTblEntry };
    }
    #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
    {
        let elements = unsafe { (*rtable).elements };
        rte = unsafe { (*elements.offset(0)).ptr_value as *mut pg_sys::RangeTblEntry };
    }

    let relation = unsafe { pg_sys::RelationIdGetRelation((*rte).relid) };

    if relation.is_null() {
        return Ok(());
    }

    if unsafe { !relation.is_column()? } {
        unsafe { pg_sys::RelationClose(relation) };
        return Ok(());
    }

    unsafe { pg_sys::RelationClose(relation) };

    Ok(())
}

#[derive(Error, Debug)]
pub enum InsertHookError {
    #[error(transparent)]
    HandlerError(#[from] HandlerError),

    #[error(transparent)]
    TransactionError(#[from] TransactionError),
}

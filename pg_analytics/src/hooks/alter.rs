use pgrx::*;

use crate::errors::{NotSupported, ParadeError};
use crate::hooks::handler::DeltaHandler;

pub unsafe fn alter(alter_stmt: *mut pg_sys::AlterTableStmt) -> Result<(), ParadeError> {
    let rangevar = (*alter_stmt).relation;
    let rangevar_oid = pg_sys::RangeVarGetRelidExtended(
        rangevar,
        pg_sys::ShareUpdateExclusiveLock as i32,
        0,
        None,
        std::ptr::null_mut(),
    );
    let relation = pg_sys::RelationIdGetRelation(rangevar_oid);

    if relation.is_null() {
        return Ok(());
    }

    if !DeltaHandler::relation_is_delta(relation)? {
        pg_sys::RelationClose(relation);
        return Ok(());
    }

    Err(NotSupported::AlterTable.into())
}

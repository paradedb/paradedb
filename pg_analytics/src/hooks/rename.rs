use async_std::task;
use pgrx::*;
use std::ffi::CStr;

use crate::datafusion::context::DatafusionContext;
use crate::errors::ParadeError;
use crate::hooks::handler::DeltaHandler;

pub unsafe fn rename(rename_stmt: *mut pg_sys::RenameStmt) -> Result<(), ParadeError> {
    let new_name = CStr::from_ptr((*rename_stmt).newname).to_str()?;
    let rangevar = (*rename_stmt).relation;
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

    let pg_relation = PgRelation::from_pg(relation);
    let table_name = pg_relation.name();
    let schema_name = pg_relation.namespace();

    pg_sys::RelationClose(relation);

    DatafusionContext::with_schema_provider(schema_name, |provider| {
        task::block_on(provider.rename(table_name, new_name))
    })
}

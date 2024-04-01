use pgrx::*;
use thiserror::Error;

use crate::datafusion::catalog::CatalogError;
use crate::datafusion::session::Session;
use crate::datafusion::table::{DataFusionTableError, DatafusionTable};
use crate::hooks::handler::{HandlerError, IsColumn};

pub unsafe fn drop(drop_stmt: *mut pg_sys::DropStmt) -> Result<(), DropHookError> {
    // Ignore if not DROP TABLE
    if (*drop_stmt).removeType != pg_sys::ObjectType_OBJECT_TABLE {
        return Ok(());
    }

    // Remove all dropped relations from schema provider
    let rels = (*drop_stmt).objects;
    let num_rels = (*rels).length;

    #[cfg(feature = "pg12")]
    let mut current_cell = (*rels).head;
    #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
    let elements = (*rels).elements;

    // DROP can be called on multiple relations at once, so we need to iterate over all of them
    for i in 0..num_rels {
        let range_list: *mut pg_sys::List;

        #[cfg(feature = "pg12")]
        {
            range_list = (*current_cell).data.ptr_value as *mut pg_sys::List;
            current_cell = (*current_cell).next;
        }
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        {
            range_list = (*elements.offset(i as isize)).ptr_value as *mut pg_sys::List;
        }

        let rangevar = pg_sys::makeRangeVarFromNameList(range_list);

        // Determine the flags for RangeVarGetRelidExtended
        let flags = if (*drop_stmt).missing_ok {
            pg_sys::RVROption_RVR_MISSING_OK
        } else {
            0
        };

        let rangevar_oid = pg_sys::RangeVarGetRelidExtended(
            rangevar,
            pg_sys::ShareUpdateExclusiveLock as i32,
            flags,
            None,
            std::ptr::null_mut(),
        );

        let relation = pg_sys::RelationIdGetRelation(rangevar_oid);

        if relation.is_null() {
            continue;
        }

        if !relation.is_column()? {
            pg_sys::RelationClose(relation);
            continue;
        }

        let pg_relation = PgRelation::from_pg(relation);
        let schema_name = pg_relation.namespace();
        let table_path = pg_relation.table_path()?;

        Session::with_tables(schema_name, |mut tables| {
            Box::pin(async move { Ok(tables.deregister(&table_path)?) })
        })?;

        pg_sys::RelationClose(relation);
    }

    Ok(())
}

#[derive(Error, Debug)]
pub enum DropHookError {
    #[error(transparent)]
    CatalogError(#[from] CatalogError),

    #[error(transparent)]
    DataFusionTableError(#[from] DataFusionTableError),

    #[error(transparent)]
    HandlerError(#[from] HandlerError),
}

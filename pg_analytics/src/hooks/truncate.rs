use async_std::task;
use pgrx::*;

use crate::datafusion::context::DatafusionContext;
use crate::errors::ParadeError;
use crate::hooks::handler::IsColumn;

pub unsafe fn truncate(truncate_stmt: *mut pg_sys::TruncateStmt) -> Result<(), ParadeError> {
    let rels = (*truncate_stmt).relations;
    let num_rels = (*rels).length;

    #[cfg(feature = "pg12")]
    let mut current_cell = (*rels).head;
    #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
    let elements = (*rels).elements;

    // TRUNCATE can be called on multiple relations at once, so we need to iterate over all of them
    for i in 0..num_rels {
        let rangevar: *mut pg_sys::RangeVar;

        #[cfg(feature = "pg12")]
        {
            rangevar = (*current_cell).data.ptr_value as *mut pg_sys::RangeVar;
            current_cell = (*current_cell).next;
        }
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        {
            rangevar = (*elements.offset(i as isize)).ptr_value as *mut pg_sys::RangeVar;
        }

        let rangevar_oid = pg_sys::RangeVarGetRelidExtended(
            rangevar,
            pg_sys::ShareUpdateExclusiveLock as i32,
            0,
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
        let table_name = pg_relation.name();
        let schema_name = pg_relation.namespace();

        pg_sys::RelationClose(relation);

        DatafusionContext::with_schema_provider(schema_name, |provider| {
            task::block_on(provider.delete(table_name, None))
        })?;
    }

    Ok(())
}

use pgrx::*;

use crate::datafusion::session::Session;
use crate::datafusion::table::DatafusionTable;
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
        let schema_name = pg_relation.namespace();
        let table_path = pg_relation.table_path()?;

        pg_sys::RelationClose(relation);

        Session::with_tables(schema_name, |tables| async move {
            let mut lock = tables.lock().await;
            let (mut delta_table, _) = lock.delete(&table_path, None).await?;

            delta_table.update().await?;
            lock.register(&table_path, delta_table)
        })?;
    }

    Ok(())
}

use deltalake::datafusion::catalog::schema::SchemaProvider;
use pgrx::*;

use crate::datafusion::context::DatafusionContext;
use crate::errors::ParadeError;
use crate::hooks::handler::DeltaHandler;
use crate::hooks::utils::relation_from_rangevar;

pub unsafe fn drop(drop_stmt: *mut pg_sys::DropStmt) -> Result<(), ParadeError> {
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
        let relation = relation_from_rangevar(rangevar);

        if relation.is_null() {
            continue;
        }

        if !DeltaHandler::relation_is_delta(relation)? {
            pg_sys::RelationClose(relation);
            continue;
        }

        let pg_relation = PgRelation::from_pg(relation);
        let table_name = pg_relation.name();
        let schema_name = pg_relation.namespace();

        DatafusionContext::with_schema_provider(schema_name, |provider| {
            let _ = provider.deregister_table(table_name);
            Ok(())
        })?;

        pg_sys::RelationClose(relation);
    }

    Ok(())
}

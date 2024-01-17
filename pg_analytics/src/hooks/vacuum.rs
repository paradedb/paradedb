use async_std::task;
use pgrx::*;
use std::ffi::CStr;

use crate::datafusion::context::DatafusionContext;
use crate::errors::ParadeError;
use crate::hooks::handler::DeltaHandler;
use deltalake::datafusion::catalog::CatalogProvider;

#[derive(Debug)]
struct VacuumOptions {
    full: bool,
    freeze: bool,
}

impl VacuumOptions {
    fn new() -> Self {
        Self {
            full: false,
            freeze: false,
        }
    }

    unsafe fn init(&mut self, options: *mut pg_sys::List) -> Result<(), ParadeError> {
        if !options.is_null() {
            let num_options = (*options).length;

            #[cfg(feature = "pg12")]
            let mut current_cell = (*options).head;
            #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
            let elements = (*options).elements;

            for i in 0..num_options {
                let option: *mut pg_sys::DefElem;
                #[cfg(feature = "pg12")]
                {
                    option = (*current_cell).data.ptr_value as *mut pg_sys::DefElem;
                    current_cell = (*current_cell).next;
                }
                #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
                {
                    option = (*elements.offset(i as isize)).ptr_value as *mut pg_sys::DefElem;
                }

                let option_name = CStr::from_ptr((*option).defname).to_str()?.to_uppercase();

                match option_name.as_str() {
                    "FULL" => self.full = true,
                    "FREEZE" => self.freeze = true,
                    _ => {}
                }
            }
        }

        Ok(())
    }
}

pub unsafe fn vacuum_analytics(vacuum_stmt: *mut pg_sys::VacuumStmt) -> Result<(), ParadeError> {
    // Read VACUUM options
    let mut vacuum_options = VacuumOptions::new();
    vacuum_options.init((*vacuum_stmt).options)?;

    // VacuumStmt can also be used for other statements, so we need to check if it's actually VACUUM
    if !(*vacuum_stmt).is_vacuumcmd {
        return Ok(());
    }

    // FREEZE doesn't actually vacuum
    if vacuum_options.freeze {
        return Ok(());
    }

    // Rels is null if VACUUM was called, not null if VACUUM <table> was called
    let rels = (*vacuum_stmt).rels;
    let vacuum_all = (*vacuum_stmt).rels.is_null();

    // Perform vacuum
    match vacuum_all {
        true => {
            let schema_names =
                DatafusionContext::with_catalog(|catalog| Ok(catalog.schema_names()))?;

            for schema_name in schema_names {
                DatafusionContext::with_schema_provider(&schema_name, |provider| {
                    task::block_on(provider.vacuum_all(vacuum_options.full))
                })?;
            }

            Ok(())
        }
        false => {
            let num_rels = (*rels).length;

            #[cfg(feature = "pg12")]
            let mut current_cell = (*rels).head;
            #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
            let elements = (*rels).elements;

            for i in 0..num_rels {
                let vacuum_rel: *mut pg_sys::VacuumRelation;

                #[cfg(feature = "pg12")]
                {
                    vacuum_rel = (*current_cell).data.ptr_value as *mut pg_sys::VacuumRelation;
                    current_cell = (*current_cell).next;
                }
                #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
                {
                    vacuum_rel =
                        (*elements.offset(i as isize)).ptr_value as *mut pg_sys::VacuumRelation;
                }

                // If the relation is null or not analytics, skip it
                let oid = match (*(*vacuum_rel).relation).schemaname.is_null() {
                    true => pg_sys::RelnameGetRelid((*(*vacuum_rel).relation).relname),
                    false => pg_sys::get_relname_relid(
                        (*(*vacuum_rel).relation).relname,
                        pg_sys::get_namespace_oid((*(*vacuum_rel).relation).schemaname, true),
                    ),
                };

                let relation = pg_sys::RelationIdGetRelation(oid);

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
                    task::block_on(provider.vacuum(table_name, vacuum_options.full))
                })?;

                pg_sys::RelationClose(relation);
            }

            Ok(())
        }
    }
}

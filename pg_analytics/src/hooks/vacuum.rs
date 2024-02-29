use deltalake::datafusion::catalog::CatalogProvider;
use pgrx::*;
use std::ffi::{CStr, CString};
use std::fs::remove_dir_all;

use crate::datafusion::directory::ParadeDirectory;
use crate::datafusion::session::Session;
use crate::datafusion::table::DatafusionTable;
use crate::errors::ParadeError;
use crate::hooks::handler::IsColumn;

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

pub unsafe fn vacuum(vacuum_stmt: *mut pg_sys::VacuumStmt) -> Result<(), ParadeError> {
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
            let schema_names = Session::with_catalog(|catalog| Ok(catalog.schema_names()))?;

            for schema_name in schema_names {
                let schema_oid = unsafe {
                    pg_sys::get_namespace_oid(CString::new(schema_name.clone())?.as_ptr(), true)
                };
                let schema_path =
                    ParadeDirectory::schema_path(Session::catalog_oid()?, schema_oid)?;
                let directory = std::fs::read_dir(schema_path.clone())?;

                // Vacuum all tables in the schema directory and delete directories for dropped tables
                for file in directory {
                    let table_oid = file?.file_name().into_string()?;

                    if let Ok(oid) = table_oid.parse::<u32>() {
                        let pg_oid = pg_sys::Oid::from(oid);
                        let relation = unsafe { pg_sys::RelationIdGetRelation(pg_oid) };

                        // If the relation is null, delete the directory
                        if relation.is_null() {
                            let path = schema_path.join(&table_oid);
                            remove_dir_all(path.clone())?;
                        // Otherwise, vacuum the table
                        } else {
                            let pg_relation = unsafe { PgRelation::from_pg(relation) };
                            let table_path = pg_relation.table_path()?;

                            unsafe { pg_sys::RelationClose(relation) }

                            Session::with_tables(&schema_name, |tables| async move {
                                let mut lock = tables.lock().await;
                                let mut delta_table =
                                    lock.vacuum(&table_path, vacuum_options.full).await?;

                                delta_table.update().await?;
                                lock.register(&table_path, delta_table)
                            })?;
                        }
                    }
                }
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

                // If the relation is null or not deltalake Table Access Method, skip it
                let rangevar = (*vacuum_rel).relation;
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

                Session::with_tables(schema_name, |tables| async move {
                    let mut lock = tables.lock().await;
                    let mut delta_table = lock.vacuum(&table_path, vacuum_options.full).await?;

                    delta_table.update().await?;
                    lock.register(&table_path, delta_table)
                })?;

                pg_sys::RelationClose(relation);
            }

            Ok(())
        }
    }
}

use anyhow::Result;
use pgrx::*;
use std::ffi::CStr;
use supabase_wrappers::prelude::options_to_hashmap;

use super::query::{get_current_query, get_query_relations};
use crate::duckdb::connection;
use crate::duckdb::parquet::create_parquet_view;
use crate::fdw::handler::FdwHandler;

pub fn executor_start(
    query_desc: PgBox<pg_sys::QueryDesc>,
    eflags: i32,
    prev_hook: fn(query_desc: PgBox<pg_sys::QueryDesc>, eflags: i32) -> HookResult<()>,
) -> Result<()> {
    let ps = query_desc.plannedstmt;
    let query = get_current_query(ps, unsafe { CStr::from_ptr(query_desc.sourceText) })?;

    if query.to_lowercase().starts_with("explain") {
        let explain = ps as *mut pg_sys::ExplainStmt;
    }

    let query_relations = get_query_relations(ps);
    for pg_relation in query_relations {
        if pg_relation.is_foreign_table() {
            let foreign_table = unsafe { pg_sys::GetForeignTable(pg_relation.oid()) };
            let fdw_handler = FdwHandler::from(foreign_table);
            let table_name = pg_relation.name();
            let schema_name = pg_relation.namespace();

            if fdw_handler != FdwHandler::Other
                && !connection::view_exists(table_name, schema_name)?
            {
                let table_options = unsafe { options_to_hashmap((*foreign_table).options)? };
                match fdw_handler {
                    FdwHandler::Parquet => {
                        create_parquet_view(table_name, schema_name, table_options)?;
                    }
                    _ => {
                        todo!()
                    }
                }
            }
        }
    }

    // if unsafe { (*ps).type_ } == pg_sys::NodeTag::T_ExplainStmt {
    //     info!("explain");
    //     info!("executor start {:?}", query_relations.len());
    // }

    Ok(())
}

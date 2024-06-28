use anyhow::{anyhow, Result};
use pgrx::*;
use std::ffi::CStr;

use crate::duckdb::connection;
use crate::fdw::handler::FdwHandler;

extension_sql!(
    r#"
    CREATE EVENT TRIGGER auto_create_schema_trigger
    ON ddl_command_end
    WHEN TAG IN ('CREATE FOREIGN TABLE')
    EXECUTE FUNCTION auto_create_schema_hook();
    "#,
    name = "auto_create_schema_trigger",
    requires = [auto_create_schema_hook]
);

#[pg_extern(sql = "
    CREATE FUNCTION auto_create_schema_hook() 
    RETURNS event_trigger 
    LANGUAGE c 
    AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
")]
fn auto_create_schema_hook(fcinfo: pg_sys::FunctionCallInfo) {
    unsafe {
        auto_create_schema_impl(fcinfo).unwrap_or_else(|e| {
            panic!("{}", e);
        });
    }
}

type SchemaRow = (String, String);

#[inline]
unsafe fn auto_create_schema_impl(fcinfo: pg_sys::FunctionCallInfo) -> Result<()> {
    // Parse fcinfo
    if !is_a((*fcinfo).context, pg_sys::NodeTag::T_EventTriggerData) {
        return Ok(());
    }

    let event_trigger_data = (*fcinfo).context as *mut pg_sys::EventTriggerData;

    if !is_a(
        (*event_trigger_data).parsetree,
        pg_sys::NodeTag::T_CreateForeignTableStmt,
    ) {
        return Ok(());
    }

    let create_foreign_stmt =
        (*event_trigger_data).parsetree as *mut pg_sys::CreateForeignTableStmt;
    let create_stmt = (*create_foreign_stmt).base;

    // Get the PgRelation that triggered the event
    let relation = create_stmt.relation;
    let schema_name = CStr::from_ptr((*relation).schemaname).to_str()?;
    let table_name = CStr::from_ptr((*relation).relname).to_str()?;
    let pg_relation =
        PgRelation::open_with_name(format!("\"{}\".\"{}\"", schema_name, table_name).as_str())
            .map_err(|err| anyhow!(err))?;

    // If the foreign table was not created by this extension, exit
    let foreign_table = unsafe { pg_sys::GetForeignTable(pg_relation.oid()) };
    if FdwHandler::from(foreign_table) == FdwHandler::Other {
        return Ok(());
    }

    // If the table already has columns, exit
    if pg_relation.tuple_desc().len() != 0 {
        return Ok(());
    }

    // Execute dummy query to force FDW to initialize a DuckDB view
    let query = format!("SELECT * FROM \"{schema_name}\".\"{table_name}\" LIMIT 0");
    Spi::run(query.as_str())?;

    let conn = unsafe { &*connection::get_global_connection().get() };
    let query = format!("DESCRIBE \"{schema_name}\".\"{table_name}\"");
    let mut stmt = conn.prepare(&query)?;

    let schema_rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .map(|row| row.unwrap())
        .collect::<Vec<SchemaRow>>();

    info!("Schema rows: {:?}", schema_rows);

    Ok(())
}

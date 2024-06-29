use anyhow::Result;
use pgrx::*;
use std::ffi::CStr;
use supabase_wrappers::prelude::options_to_hashmap;

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
    let oid = pg_sys::RelnameGetRelid((*relation).relname);

    // If the foreign table was not created by this extension, exit
    let foreign_table = unsafe { pg_sys::GetForeignTable(oid) };
    if FdwHandler::from(foreign_table) == FdwHandler::Other {
        return Ok(());
    }

    let relation = pg_sys::relation_open(oid, pg_sys::AccessShareLock as i32);

    // If the table already has columns, exit
    if (*(*relation).rd_att).natts != 0 {
        return Ok(());
    }

    pg_sys::RelationClose(relation);

    // Execute dummy query to force FDW to initialize a DuckDB view
    connection::execute(
        format!("CREATE SCHEMA IF NOT EXISTS {schema_name}").as_str(),
        [],
    )?;

    let table_options = unsafe { options_to_hashmap((*foreign_table).options)? };
    match FdwHandler::from(foreign_table) {
        FdwHandler::Csv => {
            connection::create_csv_view(table_name, schema_name, table_options)?;
        }
        FdwHandler::Delta => {
            connection::create_delta_view(table_name, schema_name, table_options)?;
        }
        FdwHandler::Iceberg => {
            connection::create_iceberg_view(table_name, schema_name, table_options)?;
        }
        FdwHandler::Parquet => {
            connection::create_parquet_view(table_name, schema_name, table_options)?;
        }
        _ => {
            todo!()
        }
    }

    let conn = unsafe { &*connection::get_global_connection().get() };
    let query = format!("DESCRIBE {schema_name}.{table_name}");
    let mut stmt = conn.prepare(&query)?;

    let schema_rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .map(|row| row.unwrap())
        .collect::<Vec<(String, String)>>();

    if schema_rows.is_empty() {
        return Ok(());
    }

    let alter_table_statement = construct_alter_table_statement(table_name, schema_rows);
    Spi::run(alter_table_statement.as_str())?;

    Ok(())
}

#[inline]
fn duckdb_type_to_pg(column_name: &str, duckdb_type: &str) -> &'static str {
    match duckdb_type.to_uppercase().as_str() {
        "BOOLEAN" => "BOOLEAN",
        "TINYINT" => "SMALLINT",
        "SMALLINT" => "SMALLINT",
        "INTEGER" => "INTEGER",
        "BIGINT" => "BIGINT",
        "UTINYINT" => "SMALLINT",
        "USMALLINT" => "INTEGER",
        "UINTEGER" => "BIGINT",
        "UBIGINT" => "BIGINT",
        "FLOAT" => "REAL",
        "DOUBLE" => "DOUBLE PRECISION",
        "TIMESTAMP" => "TIMESTAMP",
        "DATE" => "DATE",
        "TIME" => "TIME",
        "INTERVAL" => "INTERVAL",
        "HUGEINT" => "BIGINT",
        "UHUGEINT" => "BIGINT",
        "VARCHAR" => "VARCHAR",
        "BLOB" => "BYTEA",
        "DECIMAL" => "DECIMAL",
        "TIMESTAMP_S" => "TIMESTAMP",
        "TIMESTAMP_MS" => "TIMESTAMP",
        "TIMESTAMP_NS" => "TIMESTAMP",
        "ENUM" => "TEXT",
        "LIST" => "TEXT",
        "STRUCT" => "JSONB",
        "MAP" => "JSONB",
        "ARRAY" => "JSONB",
        "UUID" => "UUID",
        "UNION" => "JSONB",
        "BIT" => "BIT",
        "TIME_TZ" => "TIMETZ",
        "TIMESTAMP_TZ" => "TIMESTAMPTZ",
        other => {
            warning!("Field '{}' has DuckDB type {}, which has no clear Postgres mapping. Falling back to default type TEXT, which can be changed with ALTER TABLE", column_name, other);
            "TEXT"
        }
    }
}

#[inline]
fn construct_alter_table_statement(table_name: &str, columns: Vec<(String, String)>) -> String {
    let column_definitions: Vec<String> = columns
        .iter()
        .map(|(column_name, duckdb_type)| {
            let pg_type = duckdb_type_to_pg(column_name, duckdb_type);
            format!("ADD COLUMN {} {}", column_name, pg_type)
        })
        .collect();

    format!(
        "ALTER TABLE {} {}",
        table_name,
        column_definitions.join(", ")
    )
}

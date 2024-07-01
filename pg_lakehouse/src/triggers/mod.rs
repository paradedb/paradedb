use anyhow::{bail, Result};
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

// Foreign tables should not be create  with these names
// because they conflict with built-in DuckDB tables
// https://duckdb.org/docs/guides/meta/duckdb_environment#meta-table-functions
const DUCKDB_RESERVED_NAMES: [&str; 16] = [
    "duckdb_columns",
    "duckdb_constraints",
    "duckdb_databases",
    "duckdb_dependencies",
    "duckdb_extensions",
    "duckdb_functions",
    "duckdb_indexes",
    "duckdb_keywords",
    "duckdb_optimizers",
    "duckdb_schemas",
    "duckdb_sequences",
    "duckdb_settings",
    "duckdb_tables",
    "duckdb_types",
    "duckdb_views",
    "duckdb_temporary_files",
];

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

    // Get relation name, oid, etc. that triggered the event
    let relation = create_stmt.relation;
    let schema_name = CStr::from_ptr((*relation).schemaname).to_str()?;
    let table_name = CStr::from_ptr((*relation).relname).to_str()?;
    let oid = pg_sys::RangeVarGetRelidExtended(
        relation,
        pg_sys::AccessShareLock as i32,
        0,
        None,
        std::ptr::null_mut(),
    );

    // If the foreign table was not created by this extension, exit
    let foreign_table = unsafe { pg_sys::GetForeignTable(oid) };
    if FdwHandler::from(foreign_table) == FdwHandler::Other {
        return Ok(());
    }

    // Don't allow DuckDB reserved names
    if DUCKDB_RESERVED_NAMES.contains(&table_name) {
        bail!(
            "Table name '{}' is not allowed because it is reserved by DuckDB",
            table_name
        );
    }

    // If the table already has columns, no need for auto schema creation
    let relation = pg_sys::relation_open(oid, pg_sys::AccessShareLock as i32);
    if (*(*relation).rd_att).natts != 0 {
        return Ok(());
    }

    pg_sys::RelationClose(relation);

    // Initialize DuckDB view
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

    // Get DuckDB schema
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

    // Alter Postgres table to match DuckDB schema
    let alter_table_statement = construct_alter_table_statement(table_name, schema_rows);
    Spi::run(alter_table_statement.as_str())?;

    Ok(())
}

#[inline]
fn duckdb_type_to_pg(column_name: &str, duckdb_type: &str) -> String {
    let mut postgres_type = duckdb_type
        .replace("TINYINT", "SMALLINT")
        .replace("UTINYINT", "SMALLINT")
        .replace("USMALLINT", "INTEGER")
        .replace("UINTEGER", "BIGINT")
        .replace("UBIGINT", "NUMERIC")
        .replace("HUGEINT", "NUMERIC")
        .replace("UHUGEINT", "NUMERIC")
        .replace("BLOB", "BYTEA")
        .replace("DOUBLE", "DOUBLE PRECISION")
        .replace("TIMESTAMP_S", "TIMESTAMP")
        .replace("TIMESTAMP_MS", "TIMESTAMP")
        .replace("TIMESTAMP_NS", "TIMESTAMP")
        .replace("ARRAY", "JSONB");

    if postgres_type.starts_with("STRUCT") {
        postgres_type = "JSONB".to_string();
    }

    if postgres_type.starts_with("MAP") {
        postgres_type = "JSONB".to_string();
    }

    postgres_type
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

// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use anyhow::{bail, Result};
use pgrx::*;
use std::ffi::CStr;
use supabase_wrappers::prelude::{options_to_hashmap, user_mapping_options};

use super::base::register_duckdb_view;
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

// Foreign tables should not be created with these names
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

    // Drop stale view
    connection::execute(
        format!("DROP VIEW IF EXISTS {schema_name}.{table_name}").as_str(),
        [],
    )?;

    // Register DuckDB view
    let foreign_server = unsafe { pg_sys::GetForeignServer((*foreign_table).serverid) };
    let user_mapping_options = unsafe { user_mapping_options(foreign_server) };
    let table_options = unsafe { options_to_hashmap((*foreign_table).options)? };
    let handler = FdwHandler::from(foreign_table);
    register_duckdb_view(
        table_name,
        schema_name,
        table_options.clone(),
        user_mapping_options,
        handler,
    )?;

    // If the table already has columns, no need for auto schema creation
    let relation = pg_sys::relation_open(oid, pg_sys::AccessShareLock as i32);
    if (*(*relation).rd_att).natts != 0 {
        pg_sys::RelationClose(relation);
        return Ok(());
    }

    pg_sys::RelationClose(relation);

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
    let preserve_casing = table_options
        .get("preserve_casing")
        .map_or(false, |s| s.eq_ignore_ascii_case("true"));
    let alter_table_statement =
        construct_alter_table_statement(schema_name, table_name, schema_rows, preserve_casing);
    Spi::run(alter_table_statement.as_str())?;

    Ok(())
}

#[inline]
fn duckdb_type_to_pg(column_name: &str, duckdb_type: &str) -> Result<String> {
    if duckdb_type == "INVALID" {
        bail!("Column '{}' has an invalid DuckDB type", column_name);
    }

    if duckdb_type.starts_with("MAP") {
        bail!(
            "Column '{}' has type MAP, which is not supported",
            column_name
        );
    }

    if duckdb_type.starts_with("ENUM") {
        bail!(
            "Column '{}' has type ENUM, which is not supported",
            column_name
        );
    }

    if duckdb_type.starts_with("UNION") {
        bail!(
            "Column '{}' has type UNION, which is not supported",
            column_name
        );
    }

    if duckdb_type.starts_with("BIT") {
        bail!(
            "Column '{}' has type BIT, which is not supported",
            column_name
        );
    }

    let mut postgres_type = duckdb_type
        .replace("TINYINT", "SMALLINT")
        .replace("UTINYINT", "SMALLINT")
        .replace("USMALLINT", "INTEGER")
        .replace("UINTEGER", "BIGINT")
        .replace("UBIGINT", "NUMERIC")
        .replace("HUGEINT", "NUMERIC")
        .replace("UHUGEINT", "NUMERIC")
        .replace("BLOB", "TEXT")
        .replace("DOUBLE", "DOUBLE PRECISION")
        .replace("TIMESTAMP_S", "TIMESTAMP")
        .replace("TIMESTAMP_MS", "TIMESTAMP")
        .replace("TIMESTAMP_NS", "TIMESTAMP")
        .replace("TIME WITH TIME ZONE", "TIME");

    if postgres_type.starts_with("STRUCT") {
        postgres_type = "JSONB".to_string();
    }

    Ok(postgres_type)
}

#[inline]
fn construct_alter_table_statement(
    schema_name: &str,
    table_name: &str,
    columns: Vec<(String, String)>,
    preserve_casing: bool,
) -> String {
    let column_definitions: Vec<String> = columns
        .iter()
        .map(|(column_name, duckdb_type)| {
            let pg_type =
                duckdb_type_to_pg(column_name, duckdb_type).expect("failed to convert DuckDB type");

            let column_name = if preserve_casing {
                spi::quote_identifier(column_name)
            } else {
                column_name.to_string()
            };
            format!("ADD COLUMN {} {}", column_name, pg_type)
        })
        .collect();

    format!(
        "ALTER TABLE {}.{} {}",
        spi::quote_identifier(schema_name),
        spi::quote_identifier(table_name),
        column_definitions.join(", ")
    )
}

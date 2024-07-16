use anyhow::Result;
use pgrx::*;

use crate::duckdb::connection;

type DuckdbSettingsRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

#[pg_extern]
pub fn duckdb_execute(query: &str) {
    if !is_icu_loaded() {
        connection::execute("INSTALL icu", []).unwrap_or_else(|err| panic!("error installing ICU extension: {err:?}"));
        connection::execute("LOAD icu", []).unwrap_or_else(|err| panic!("error loading ICU extension: {err:?}"));
    }
    connection::execute(query, []).unwrap_or_else(|err| panic!("error executing query: {err:?}"));
}

fn is_icu_loaded() -> bool {
    let conn = unsafe { &*connection::get_global_connection().get() };
    let result = conn.query_row("SELECT name FROM pragma_database_list() WHERE name = 'icu'", [], |row| {
        row.get::<_, Option<String>>(0)
    });

    match result {
        Ok(Some(_)) => true,
        _ => false,
    }
}

#[allow(clippy::type_complexity)]
#[pg_extern]
pub fn duckdb_settings() -> iter::TableIterator<
    'static,
    (
        name!(name, Option<String>),
        name!(value, Option<String>),
        name!(description, Option<String>),
        name!(input_type, Option<String>),
        name!(scope, Option<String>),
    ),
> {
    let rows = duckdb_settings_impl().unwrap_or_else(|e| {
        panic!("{}", e);
    });
    iter::TableIterator::new(rows)
}

#[inline]
fn duckdb_settings_impl() -> Result<Vec<DuckdbSettingsRow>> {
    let conn = unsafe { &*connection::get_global_connection().get() };
    let mut stmt = conn.prepare("SELECT * FROM duckdb_settings()")?;

    Ok(stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })?
        .map(|row| row.unwrap())
        .collect::<Vec<DuckdbSettingsRow>>())
}
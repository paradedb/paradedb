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
    connection::execute(query, []).unwrap_or_else(|err| panic!("error executing query: {err:?}"));
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

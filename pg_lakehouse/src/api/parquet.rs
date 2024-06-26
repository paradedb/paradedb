use anyhow::Result;
use pgrx::*;

use crate::duckdb::connection;
use crate::duckdb::utils;

type ParquetSchemaRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<String>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<String>,
);

#[allow(clippy::type_complexity)]
#[pg_extern]
pub fn parquet_schema(
    files: &str,
) -> iter::TableIterator<(
    name!(file_name, Option<String>),
    name!(name, Option<String>),
    name!(type, Option<String>),
    name!(type_length, Option<String>),
    name!(repetition_type, Option<String>),
    name!(num_children, Option<i64>),
    name!(converted_type, Option<String>),
    name!(scale, Option<i64>),
    name!(precision, Option<i64>),
    name!(field_id, Option<i64>),
    name!(logical_type, Option<String>),
)> {
    let rows = parquet_schema_impl(files).unwrap_or_else(|e| {
        panic!("{}", e);
    });
    iter::TableIterator::new(rows)
}

#[inline]
fn parquet_schema_impl(files: &str) -> Result<Vec<ParquetSchemaRow>> {
    let schema_str = utils::format_csv(files);
    let conn = unsafe { &*connection::get_global_connection().get() };
    let query = format!("SELECT * FROM parquet_schema({})", schema_str);
    let mut stmt = conn.prepare(&query)?;

    Ok(stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<i64>>(7)?,
                row.get::<_, Option<i64>>(8)?,
                row.get::<_, Option<i64>>(9)?,
                row.get::<_, Option<String>>(10)?,
            ))
        })?
        .map(|row| row.unwrap())
        .collect::<Vec<ParquetSchemaRow>>())
}

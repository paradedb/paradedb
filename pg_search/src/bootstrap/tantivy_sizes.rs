use crate::postgres::utils::get_search_index;
use pgrx::prelude::*;
use pgrx::Spi;

#[pg_extern(sql = "
CREATE OR REPLACE FUNCTION paradedb.pg_total_relation_size_with_tantivy(
    index_name text,
    table_name text
) RETURNS bigint
LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
")]
fn pg_total_relation_size_with_tantivy(index_name: &str, table_name: &str) -> i64 {
    let standard_size =  {
        Spi::get_one::<i64>(&format!("SELECT pg_total_relation_size('{}')", table_name))
    }.unwrap_or(Some(0));

    let tantivy_size = match get_tantivy_index_size(index_name) {
        Ok(size) => size,
        Err(e) => {
            log!("Error getting Tantivy index size: {}", e);
            0
        }
    };
    standard_size.unwrap_or(0) + tantivy_size
}

#[pg_extern(sql = "
CREATE OR REPLACE FUNCTION paradedb.pg_indexes_size_with_tantivy(
    index_name text,
    table_name text
) RETURNS bigint
LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
")]
fn pg_indexes_size_with_tantivy(index_name: &str, table_name: &str) -> i64 {
    let standard_index_size =  {
        Spi::get_one::<i64>(&format!("SELECT pg_indexes_size('{}')", table_name))
    }.unwrap_or(Some(0));

    let tantivy_size = match get_tantivy_index_size(index_name) {
        Ok(size) => size,
        Err(e) => {
            eprintln!("Error getting Tantivy index size: {}", e);
            0
        }
    };
    standard_index_size.unwrap_or(0) + tantivy_size
}

fn get_tantivy_index_size(index_name: &str) -> Result<i64, String> {
    let tantivy_index_name = format!("{}_bm25_index", index_name);
    let search_index = get_search_index(&tantivy_index_name);
    let index_size = search_index.index_size();
    Ok(index_size as i64)
}

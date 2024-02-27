use async_std::task;
use pgrx::*;
use shared::postgres::transaction::Transaction;
use std::panic::AssertUnwindSafe;
use std::path::Path;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::table::DatafusionTable;
use crate::errors::ParadeError;

const TRANSACTION_CALLBACK_CACHE_ID: &str = "parade_parquet_table";

pub fn insert(
    rtable: *mut pg_sys::List,
    _query_desc: PgBox<pg_sys::QueryDesc>,
) -> Result<(), ParadeError> {
    let rte: *mut pg_sys::RangeTblEntry;

    #[cfg(feature = "pg12")]
    {
        let current_cell = unsafe { (*rtable).head };
        rte = unsafe { (*current_cell).data.ptr_value as *mut pg_sys::RangeTblEntry };
    }
    #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
    {
        let elements = unsafe { (*rtable).elements };
        rte = unsafe { (*elements.offset(0)).ptr_value as *mut pg_sys::RangeTblEntry };
    }

    let relation = unsafe { pg_sys::RelationIdGetRelation((*rte).relid) };
    let pg_relation = unsafe { PgRelation::from_pg_owned(relation) };
    let table_name = pg_relation.name().to_string();
    let schema_name = pg_relation.namespace().to_string();
    let table_path = pg_relation.table_path()?;

    Transaction::call_once_on_precommit(
        TRANSACTION_CALLBACK_CACHE_ID,
        AssertUnwindSafe(move || {
            task::block_on(insert_callback(table_name, schema_name, &table_path))
                .expect("Insert callback failed");
        }),
    )?;

    Ok(())
}

#[inline]
async fn insert_callback(
    table_name: String,
    schema_name: String,
    table_path: &Path,
) -> Result<(), ParadeError> {
    let mut delta_table = DatafusionContext::with_writers(&schema_name, |mut writers| {
        task::block_on(writers.flush_and_commit(&table_name, &schema_name, table_path))
    })?;

    delta_table.update().await?;

    DatafusionContext::with_tables(&schema_name, |mut tables| {
        tables.register(table_path, delta_table)
    })
}

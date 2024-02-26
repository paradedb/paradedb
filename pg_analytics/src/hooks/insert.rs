use async_std::task;
use deltalake::datafusion::arrow::datatypes::Schema as ArrowSchema;
use pgrx::*;
use shared::postgres::transaction::Transaction;
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::sync::Arc;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::directory::ParadeDirectory;
use crate::datafusion::schema::ParadeSchemaProvider;
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
    let schema_name = pg_relation.namespace();
    let table_oid = pg_relation.oid();
    let schema_oid = pg_relation.namespace_oid();
    let table_path =
        ParadeDirectory::table_path(DatafusionContext::catalog_oid()?, schema_oid, table_oid)?;

    let writer =
        DatafusionContext::with_schema_provider(schema_name, |provider| provider.writers())?;

    Transaction::call_once_on_precommit(
        TRANSACTION_CALLBACK_CACHE_ID,
        AssertUnwindSafe(move || {
            let mut writer_lock = writer.lock();
            task::block_on(writer_lock.flush_and_commit(&table_name, table_path)).unwrap();
        }),
    )?;

    Ok(())
}

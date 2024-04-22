use std::path::PathBuf;
use std::sync::Arc;

use crate::datafusion::catalog::CatalogError;
use crate::datafusion::session::Session;
use crate::datafusion::table::DatafusionTable;
use crate::datafusion::writer::Writer;
use crate::storage::metadata::PgMetadata;
use async_std::task::block_on;
use deltalake::arrow::record_batch::RecordBatch;
use pgrx::*;

async unsafe fn delete_impl(
    pg_relation: PgRelation,
    schema_name: String,
    table_path: PathBuf,
) -> Result<(), CatalogError> {
    // Clear all pending write commits for this table since it's being truncated
    Writer::clear_actions(&table_path).await?;

    Session::with_tables(&schema_name, |mut tables| {
        Box::pin(async move {
            let _ = tables.logical_delete(&table_path, None).await?;

            let arrow_schema = Arc::new(pg_relation.arrow_schema_with_reserved_fields()?);
            let batch = RecordBatch::new_empty(arrow_schema);

            tables.alter_schema(&table_path, batch).await?;

            Ok(())
        })
    })
}

#[pg_guard]
pub unsafe extern "C" fn deltalake_tuple_delete(
    rel: pg_sys::Relation,
    _tid: pg_sys::ItemPointer,
    _cid: pg_sys::CommandId,
    _snapshot: pg_sys::Snapshot,
    _crosscheck: pg_sys::Snapshot,
    _wait: bool,
    _tmfd: *mut pg_sys::TM_FailureData,
    _changingPart: bool,
) -> pg_sys::TM_Result {
    // DELETE AM METHOD
    let pg_relation = unsafe { PgRelation::from_pg(rel) };
    let schema_name = pg_relation.namespace().to_string();
    let table_path = pg_relation.table_path().expect("ERROR");

    // Removes all blocks from the relation
    pg_sys::RelationTruncate(rel, 0);

    // Reset the relation's metadata
    rel.init_metadata((*rel).rd_smgr).expect("ERROR");
    pg_sys::RelationClose(rel);

    block_on(async {
        delete_impl(pg_relation, schema_name, table_path)
            .await
            .expect("ERROR")
    });

    0
}

use std::path::PathBuf;
use std::sync::Arc;

use crate::datafusion::catalog::CatalogError;
use crate::datafusion::session::Session;
use crate::datafusion::table::{DatafusionTable, RESERVED_TID_FIELD};
use crate::datafusion::writer::Writer;
use crate::storage::tid::RowNumber;
use async_std::task::block_on;
use deltalake::arrow::record_batch::RecordBatch;
use deltalake::datafusion::logical_expr::{col, Expr};
use deltalake::datafusion::scalar::ScalarValue;
use pgrx::*;

async unsafe fn delete_impl<I>(
    cid: pg_sys::CommandId,
    pg_relation: PgRelation,
    schema_name: String,
    table_path: PathBuf,
    row_numbers: I,
) -> Result<(), CatalogError>
where
    I: IntoIterator<Item = RowNumber>,
{
    Writer::clear_actions(&table_path).await?;

    let rows_exprs: Vec<_> = row_numbers
        .into_iter()
        .map(|rn| Expr::Literal(ScalarValue::from(rn.0)))
        .collect();

    Session::with_tables(&schema_name, |mut tables| {
        Box::pin(async move {
            let _ = tables
                .logical_delete(
                    cid,
                    &table_path,
                    Some(col(RESERVED_TID_FIELD).in_list(rows_exprs, false)),
                )
                .await?;

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
    tid: pg_sys::ItemPointer,
    cid: pg_sys::CommandId,
    _snapshot: pg_sys::Snapshot,
    _crosscheck: pg_sys::Snapshot,
    _wait: bool,
    _tmfd: *mut pg_sys::TM_FailureData,
    _changingPart: bool,
) -> pg_sys::TM_Result {
    let pg_relation = unsafe { PgRelation::from_pg(rel) };
    let schema_name = pg_relation.namespace().to_string();
    let table_path = pg_relation.table_path().expect("ERROR");

    let row_number = RowNumber::try_from(*tid).expect("ERROR");

    block_on(async {
        delete_impl(cid, pg_relation, schema_name, table_path, [row_number; 1])
            .await
            .expect("ERROR")
    });
    0
}

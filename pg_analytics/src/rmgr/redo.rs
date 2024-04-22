use deltalake::datafusion::common::{DataFusionError, ScalarValue};
use deltalake::datafusion::logical_expr::{col, Expr};
use pgrx::*;
use shared::postgres::wal::{page_get_lsn, xlog_rec_get_data};
use std::ffi::CStr;
use thiserror::Error;

use crate::datafusion::catalog::CatalogError;
use crate::datafusion::directory::ParadeDirectory;
use crate::datafusion::session::Session;
use crate::datafusion::table::{PgTableProvider, RESERVED_TID_FIELD, RESERVED_XMIN_FIELD};
use crate::storage::metadata::{PgMetadata, RelationMetadata};
use crate::storage::tid::{RowNumber, TidError};

use super::XLogInsertRecord;

pub async unsafe fn redo_insert(record: *mut pg_sys::XLogReaderState) -> Result<(), RmgrRedoError> {
    let mut arg_len = 0;
    let block_data = pg_sys::XLogRecGetBlockData(record, 0, &mut arg_len);
    let heap_tuple = block_data as *mut pg_sys::HeapTupleData;
    let metadata = xlog_rec_get_data(record) as *mut XLogInsertRecord;
    let RowNumber(row_number) = (*heap_tuple).t_self.try_into()?;
    let xmin = (*metadata).xmin();
    let table_oid = (*heap_tuple).t_tableOid;
    let schema_oid = (*metadata).schema_oid();
    let tablespace_oid = (*metadata).tablespace_oid();

    // TODO: Check if xmin is aborted

    // Check to see if the tuple has already been written
    let full_dataframe = Session::with_tables(&schema_name.clone(), |mut tables| {
        Box::pin(async move {
            let table_path = ParadeDirectory::table_path_from_oid(tablespace_oid, schema_oid, table_oid)?;
            let delta_table = tables.get_ref(&table_path).await?;
            let provider =
                PgTableProvider::new(delta_table.clone(), &schema_name, &table_name).await?;

            Ok(provider.dataframe())
        })
    })?;

    ereport!(
        PgLogLevel::LOG,
        PgSqlErrorCode::ERRCODE_SUCCESSFUL_COMPLETION,
        "HERE"
    );

    let filtered_dataframe = full_dataframe.filter(
        col(RESERVED_TID_FIELD).eq(Expr::Literal(ScalarValue::from(row_number))
            .and(col(RESERVED_XMIN_FIELD).eq(Expr::Literal(ScalarValue::from(xmin))))),
    )?;

    let needs_redo = filtered_dataframe.collect().await?.is_empty();

    ereport!(
        PgLogLevel::LOG,
        PgSqlErrorCode::ERRCODE_SUCCESSFUL_COMPLETION,
        format!("needs redo {}", needs_redo)
    );

    Ok(())
}

#[derive(Error, Debug)]
pub enum RmgrRedoError {
    #[error(transparent)]
    CatalogError(#[from] CatalogError),

    #[error(transparent)]
    DataFusionError(#[from] DataFusionError),

    #[error(transparent)]
    TidError(#[from] TidError),
}

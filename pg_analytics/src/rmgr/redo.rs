use pgrx::*;
use shared::postgres::wal::page_get_lsn;
use thiserror::Error;

use crate::storage::metadata::{PgMetadata, RelationMetadata};
use crate::storage::tid::{RowNumber, TidError};

pub unsafe fn redo_insert(record: *mut pg_sys::XLogReaderState) -> Result<(), RmgrRedoError> {
    let mut arg_len = 0;
    let block_data = pg_sys::XLogRecGetBlockData(record, 0, &mut arg_len);
    let heap_tuple = block_data as *mut pg_sys::HeapTupleData;
    let RowNumber(row_number) = (*heap_tuple).t_self.try_into()?;

    ereport!(
        PgLogLevel::LOG,
        PgSqlErrorCode::ERRCODE_SUCCESSFUL_COMPLETION,
        "GETTING BUFFER"
    );

    let mut buffer: pg_sys::Buffer = pg_sys::InvalidBuffer as i32;
    let should_redo = pg_sys::XLogReadBufferForRedo(record, 0, &mut buffer);

    ereport!(
        PgLogLevel::LOG,
        PgSqlErrorCode::ERRCODE_SUCCESSFUL_COMPLETION,
        format!(
            "redo? {:?} {} {}",
            should_redo,
            pg_sys::XLogRedoAction_BLK_DONE,
            pg_sys::XLogRedoAction_BLK_NEEDS_REDO
        )
    );

    if buffer != pg_sys::InvalidBuffer as i32 {
        let page = pg_sys::BufferGetPage(buffer);
        let highest_lsn = page_get_lsn(page);
        let wal_lsn = (*record).EndRecPtr;

        ereport!(
            PgLogLevel::LOG,
            PgSqlErrorCode::ERRCODE_SUCCESSFUL_COMPLETION,
            format!("page {:?} wal {:?}", highest_lsn, wal_lsn)
        );

        pg_sys::UnlockReleaseBuffer(buffer);
    }

    Ok(())
}

#[derive(Error, Debug)]
pub enum RmgrRedoError {
    #[error(transparent)]
    TidError(#[from] TidError),
}

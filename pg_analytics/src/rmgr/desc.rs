use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use shared::postgres::wal::xlog_rec_get_data;
use thiserror::Error;

use super::XLogInsertRecord;
use crate::storage::tid::{RowNumber, TidError};

pub unsafe fn desc_insert(
    buf: *mut pg_sys::StringInfoData,
    record: *mut pg_sys::XLogReaderState,
) -> Result<(), RmgrDescError> {
    let mut arg_len = 0;
    let block_data = pg_sys::XLogRecGetBlockData(record, 0, &mut arg_len);
    let heap_tuple = block_data as *mut pg_sys::HeapTupleData;
    let metadata = xlog_rec_get_data(record) as *mut XLogInsertRecord;
    let RowNumber(row_number) = (*heap_tuple).t_self.try_into()?;

    pg_sys::appendStringInfo(
        buf,
        format!(
            "flags: 0x{:02X} row number: {}",
            (*metadata).flags(),
            row_number
        )
        .as_pg_cstr(),
    );

    Ok(())
}

#[derive(Error, Debug)]
pub enum RmgrDescError {
    #[error(transparent)]
    TidError(#[from] TidError),
}

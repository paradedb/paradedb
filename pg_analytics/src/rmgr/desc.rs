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
    let metadata = xlog_rec_get_data(record) as *mut XLogInsertRecord;

    pg_sys::appendStringInfo(
        buf,
        format!("flags: 0x{:02X}", (*metadata).flags()).as_pg_cstr(),
    );

    Ok(())
}

#[derive(Error, Debug)]
pub enum RmgrDescError {
    #[error(transparent)]
    TidError(#[from] TidError),
}

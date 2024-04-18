use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use shared::postgres::wal::{xlog_rec_get_data, xlog_rec_get_info};

use super::xlog::XLOG_INSERT;

pub unsafe extern "C" fn rm_desc(
    buf: *mut pg_sys::StringInfoData,
    record: *mut pg_sys::XLogReaderState,
) {
    let decoded_record = xlog_rec_get_data(record);
    let info_mask = pg_sys::XLR_INFO_MASK as u8;
    let info = xlog_rec_get_info(record) & !info_mask;

    if info == XLOG_INSERT {
        pg_sys::appendStringInfoString(buf, "off: insert, flags: insert".as_pg_cstr());
    } else {
        pg_sys::appendStringInfoString(buf, "off: unknown, flags: unknown".as_pg_cstr());
    }
}

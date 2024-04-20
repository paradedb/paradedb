mod redo;
pub mod xlog;

use once_cell::sync::Lazy;
use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use shared::postgres::wal::{xlog_rec_get_data, xlog_rec_get_info};

use crate::rmgr::redo::*;
use crate::rmgr::xlog::*;

pub static CUSTOM_RMGR_ID: u8 = 128;

pub static mut CUSTOM_RMGR: Lazy<pg_sys::RmgrData> = Lazy::new(|| pg_sys::RmgrData {
    rm_name: "pg_analytics".as_pg_cstr(),
    rm_redo: Some(rm_redo),
    rm_desc: Some(rm_desc),
    rm_identify: Some(rm_identify),
    rm_startup: Some(rm_startup),
    rm_cleanup: Some(rm_cleanup),
    rm_mask: Some(rm_mask),
    rm_decode: Some(rm_decode),
});

unsafe extern "C" fn rm_startup() {}
unsafe extern "C" fn rm_cleanup() {}

pub unsafe extern "C" fn rm_desc(
    buf: *mut pg_sys::StringInfoData,
    record: *mut pg_sys::XLogReaderState,
) {
    let metadata = xlog_rec_get_data(record) as *mut XLogInsertRecord;
    let info_mask = pg_sys::XLR_INFO_MASK as u8;
    let info = xlog_rec_get_info(record) & !info_mask;

    if info == XLOG_INSERT {
        pg_sys::appendStringInfo(
            buf,
            format!(
                "flags: 0x{:02X} row number: {}",
                (*metadata).flags(),
                (*metadata).row_number()
            )
            .as_pg_cstr(),
        );
    }
}

unsafe extern "C" fn rm_redo(record: *mut pg_sys::XLogReaderState) {
    let info_mask = pg_sys::XLR_INFO_MASK as u8;
    let info = xlog_rec_get_info(record) & !info_mask;

    if info == XLOG_INSERT {
        redo_insert(record).unwrap_or_else(|err| {
            panic!("{:?}", err);
        });
    }
}

unsafe extern "C" fn rm_mask(page_data: *mut i8, block_number: u32) {}

unsafe extern "C" fn rm_decode(
    context: *mut pg_sys::LogicalDecodingContext,
    buffer: *mut pg_sys::XLogRecordBuffer,
) {
}

unsafe extern "C" fn rm_identify(info: u8) -> *const i8 {
    let info_mask = pg_sys::XLR_INFO_MASK as u8;
    if (info & !info_mask) == XLOG_INSERT {
        "INSERT".as_pg_cstr()
    } else {
        "UNKNOWN".as_pg_cstr()
    }
}

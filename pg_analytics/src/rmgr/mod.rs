mod desc;
pub mod xlog;

use once_cell::sync::Lazy;
use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use shared::postgres::wal::xlog_rec_get_info;
use std::ffi::c_char;

use crate::rmgr::desc::*;
use crate::rmgr::xlog::*;

// See https://wiki.postgresql.org/wiki/CustomWALResourceManagers
// for available rmgr IDs
pub static CUSTOM_RMGR_ID: u8 = 137;

pub static mut CUSTOM_RMGR: Lazy<pg_sys::RmgrData> = Lazy::new(|| pg_sys::RmgrData {
    rm_name: "pg_analytics".as_pg_cstr(),
    rm_redo: Some(rm_redo),
    rm_desc: Some(rm_desc),
    rm_identify: Some(rm_identify),
    rm_startup: Some(rm_startup),
    rm_cleanup: Some(rm_cleanup),
    rm_mask: Some(rm_mask),
    #[cfg(any(feature = "pg15", feature = "pg16"))]
    rm_decode: Some(rm_decode),
});

unsafe extern "C" fn rm_startup() {}
unsafe extern "C" fn rm_cleanup() {}

pub unsafe extern "C" fn rm_desc(
    buf: *mut pg_sys::StringInfoData,
    record: *mut pg_sys::XLogReaderState,
) {
    let info_mask = pg_sys::XLR_INFO_MASK as u8;
    let masked_info = xlog_rec_get_info(record) & !info_mask;

    match masked_info {
        XLOG_INSERT => desc_insert(buf, record).unwrap_or_else(|err| {
            panic!("{:?}", err);
        }),
        XLOG_TRUNCATE => desc_truncate(buf, record).unwrap_or_else(|err| {
            panic!("{:?}", err);
        }),
        _ => {}
    };
}

unsafe extern "C" fn rm_redo(_record: *mut pg_sys::XLogReaderState) {
    // Tech Debt: rm_redo is not implemented
}

unsafe extern "C" fn rm_mask(_page_data: *mut c_char, _block_number: u32) {
    // Tech Debt: rm_mask is not implemented
}

unsafe extern "C" fn rm_identify(info: u8) -> *const i8 {
    let info_mask = pg_sys::XLR_INFO_MASK as u8;
    let masked_info = info & !info_mask;

    match masked_info {
        XLOG_INSERT => XLogEntry::Insert.to_str().as_pg_cstr(),
        XLOG_TRUNCATE => XLogEntry::Truncate.to_str().as_pg_cstr(),
        _ => XLogEntry::Unknown.to_str().as_pg_cstr(),
    }
}

#[cfg(any(feature = "pg15", feature = "pg16"))]
unsafe extern "C" fn rm_decode(
    _context: *mut pg_sys::LogicalDecodingContext,
    _buffer: *mut pg_sys::XLogRecordBuffer,
) {
    // rm_decode, used for logical replication, is an enterprise feature
}

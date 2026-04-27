// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use pgrx::{pg_guard, pg_sys};
use std::ffi::CStr;

// see https://wiki.postgresql.org/wiki/CustomWALResourceManagers
pub const RMGR_ID: pg_sys::RmgrId = 137;
const RMGR_NAME: &CStr = c"pg_search";

const XLOG_PG_SEARCH_INIT_INDEX: u8 = 0x10;
const XLOG_PG_SEARCH_INIT_INDEX_NAME: &CStr = c"INIT_INDEX";

#[pg_guard]
unsafe extern "C-unwind" fn rm_redo(record: *mut pg_sys::XLogReaderState) {
    let info = (*(*record).record).header.xl_info & !(pg_sys::XLR_INFO_MASK as u8);
    if info == XLOG_PG_SEARCH_INIT_INDEX && pg_sys::StandbyMode {
        pgrx::error!(
            "replicas are not supported on community and require paradedb enterprise, \
             which guarantees physical replication safety on standbys."
        );
    }
}

#[pg_guard]
unsafe extern "C-unwind" fn rm_desc(
    _buf: pg_sys::StringInfo,
    _record: *mut pg_sys::XLogReaderState,
) {
}

#[pg_guard]
unsafe extern "C-unwind" fn rm_identify(info: u8) -> *const ::core::ffi::c_char {
    match info & !(pg_sys::XLR_INFO_MASK as u8) {
        XLOG_PG_SEARCH_INIT_INDEX => XLOG_PG_SEARCH_INIT_INDEX_NAME.as_ptr(),
        _ => std::ptr::null(),
    }
}

#[pg_guard]
unsafe extern "C-unwind" fn rm_startup() {}

#[pg_guard]
unsafe extern "C-unwind" fn rm_cleanup() {}

#[pg_guard]
unsafe extern "C-unwind" fn rm_mask(
    _pagedata: *mut ::core::ffi::c_char,
    _blkno: pg_sys::BlockNumber,
) {
}

#[pg_guard]
unsafe extern "C-unwind" fn rm_decode(
    _ctx: *mut pg_sys::LogicalDecodingContext,
    _buf: *mut pg_sys::XLogRecordBuffer,
) {
}

pub fn emit_init_record() {
    // The data pointer signature differs by Postgres version (`*mut c_char` on pg15-17,
    // `*const c_void` on pg18); `as _` lets each build infer the right type.
    let mut payload: u8 = 0;
    unsafe {
        pg_sys::XLogBeginInsert();
        pg_sys::XLogRegisterData(
            &mut payload as *mut u8 as _,
            std::mem::size_of::<u8>() as u32,
        );
        pg_sys::XLogInsert(RMGR_ID, XLOG_PG_SEARCH_INIT_INDEX);
    }
}

pub fn register() {
    let rmgr = pg_sys::RmgrData {
        rm_name: RMGR_NAME.as_ptr(),
        rm_redo: Some(rm_redo),
        rm_desc: Some(rm_desc),
        rm_identify: Some(rm_identify),
        rm_startup: Some(rm_startup),
        rm_cleanup: Some(rm_cleanup),
        rm_mask: Some(rm_mask),
        rm_decode: Some(rm_decode),
    };

    unsafe {
        pg_sys::RegisterCustomRmgr(RMGR_ID, &rmgr);
    }
}

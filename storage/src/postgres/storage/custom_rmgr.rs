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
#![allow(unsafe_op_in_unsafe_fn)]

use crate::postgres::promotion;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::metadata::MetaPage;
use pgrx::{pg_guard, pg_sys};
use std::ffi::CStr;

// see https://wiki.postgresql.org/wiki/CustomWALResourceManagers
pub const RMGR_ID: pg_sys::RmgrId = 137;
const RMGR_NAME: &CStr = c"pg_search";

const XLOG_PG_SEARCH_INIT_INDEX: u8 = 0x10;
const XLOG_PG_SEARCH_INIT_INDEX_NAME: &CStr = c"INIT_INDEX";

/// The record's payload: where the index's files are, which is what replay can reach without
/// a catalog.
#[repr(C)]
#[derive(Clone, Copy)]
struct InitIndexRecord {
    tablespace: u32,
    database: u32,
    relfilenumber: u32,
}

unsafe fn record_info(record: *mut pg_sys::XLogReaderState) -> u8 {
    (*(*record).record).header.xl_info & !(pg_sys::XLR_INFO_MASK as u8)
}

/// Primaries older than 0.26.7 wrote a single zero byte instead.
unsafe fn payload(record: *mut pg_sys::XLogReaderState) -> Option<InitIndexRecord> {
    if ((*(*record).record).main_data_len as usize) < std::mem::size_of::<InitIndexRecord>() {
        return None;
    }
    Some(std::ptr::read_unaligned(
        (*(*record).record).main_data.cast::<InitIndexRecord>(),
    ))
}

/// Sets the standby taint in block 0 of the named index. A block that does not exist is left
/// to Postgres's invalid-page tracking, as for any rmgr: a later DROP forgets it.
unsafe fn taint_replicated_index(record: *mut pg_sys::XLogReaderState, index: InitIndexRecord) {
    #[cfg(feature = "pg15")]
    let locator = pg_sys::RelFileNode {
        spcNode: pg_sys::Oid::from(index.tablespace),
        dbNode: pg_sys::Oid::from(index.database),
        relNode: pg_sys::Oid::from(index.relfilenumber),
    };
    #[cfg(not(feature = "pg15"))]
    let locator = pg_sys::RelFileLocator {
        spcOid: pg_sys::Oid::from(index.tablespace),
        dbOid: pg_sys::Oid::from(index.database),
        relNumber: pg_sys::Oid::from(index.relfilenumber),
    };
    let buffer = pg_sys::XLogReadBufferExtended(
        locator,
        pg_sys::ForkNumber::MAIN_FORKNUM,
        0,
        pg_sys::ReadBufferMode::RBM_NORMAL,
        pg_sys::InvalidBuffer as pg_sys::Buffer,
    );
    if buffer == pg_sys::InvalidBuffer as pg_sys::Buffer {
        return;
    }
    pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_EXCLUSIVE as _);
    let page = pg_sys::BufferGetPage(buffer);
    MetaPage::taint_page(page);
    pg_sys::PageSetLSN(page, (*record).EndRecPtr);
    pg_sys::MarkBufferDirty(buffer);
    pg_sys::UnlockReleaseBuffer(buffer);
    pgrx::log!(
        "a ParadeDB index (database {}, file {}) was built or vacuumed on the primary; ParadeDB \
         Community does not serve ParadeDB indexes on a standby, and after promotion this one \
         refuses queries until REINDEX",
        index.database,
        index.relfilenumber
    );
}

#[pg_guard]
unsafe extern "C-unwind" fn rm_redo(record: *mut pg_sys::XLogReaderState) {
    // The index pages reached the standby as full-page images before this record. An ERROR in
    // the startup process is FATAL and stops replay for the whole cluster, so nothing here errors.
    if record_info(record) == XLOG_PG_SEARCH_INIT_INDEX && promotion::taints_during_redo() {
        match payload(record) {
            Some(index) => taint_replicated_index(record, index),
            None => pgrx::log!(
                "a ParadeDB index was created on the primary, which runs a pg_search older than \
                 0.26.7; ParadeDB Community does not serve ParadeDB indexes on a standby"
            ),
        }
    }
}

#[pg_guard]
unsafe extern "C-unwind" fn rm_desc(buf: pg_sys::StringInfo, record: *mut pg_sys::XLogReaderState) {
    if record_info(record) != XLOG_PG_SEARCH_INIT_INDEX {
        return;
    }
    if let Some(index) = payload(record) {
        let text = format!(
            "tablespace {} database {} relfilenumber {}\0",
            index.tablespace, index.database, index.relfilenumber
        );
        pg_sys::appendStringInfoString(buf, text.as_ptr().cast());
    }
}

#[pg_guard]
unsafe extern "C-unwind" fn rm_identify(info: u8) -> *const ::core::ffi::c_char {
    match info & !(pg_sys::XLR_INFO_MASK as u8) {
        XLOG_PG_SEARCH_INIT_INDEX => XLOG_PG_SEARCH_INIT_INDEX_NAME.as_ptr(),
        _ => std::ptr::null(),
    }
}

#[pg_guard]
unsafe extern "C-unwind" fn rm_startup() {
    promotion::recovery_starting();
}

#[pg_guard]
unsafe extern "C-unwind" fn rm_cleanup() {
    promotion::recovery_ending();
}

#[pg_guard]
unsafe extern "C-unwind" fn rm_mask(
    _pagedata: *mut ::core::ffi::c_char,
    _blkno: pg_sys::BlockNumber,
) {
}

/// Writes the `INIT_INDEX` record naming `index`. A standby replaying it taints the index's
/// metapage; see `postgres::promotion`.
pub fn emit_init_record(index: &PgSearchRelation) {
    // XLogRegisterData's signature varies across Postgres versions
    // (data is `*mut c_char` on pg15-17 vs `*const c_void` on pg18, and len is `c_int`
    // on pg15 vs `uint32` later). `as _` lets each build infer the right types.
    let mut payload = InitIndexRecord {
        tablespace: index.tablespace_oid().to_u32(),
        database: index.database_oid().to_u32(),
        relfilenumber: index.relfilenumber(),
    };
    unsafe {
        pg_sys::XLogBeginInsert();
        pg_sys::XLogRegisterData(
            &mut payload as *mut InitIndexRecord as _,
            std::mem::size_of::<InitIndexRecord>() as _,
        );
        pg_sys::XLogInsert(RMGR_ID, XLOG_PG_SEARCH_INIT_INDEX);
    }
}

pub fn register() {
    // `mut` because RegisterCustomRmgr takes `*mut RmgrData` on pg15 (vs `*const` later).
    // The function only reads the struct, but the C signature was tightened post-15.
    let mut rmgr = pg_sys::RmgrData {
        rm_name: RMGR_NAME.as_ptr(),
        rm_redo: Some(rm_redo),
        rm_desc: Some(rm_desc),
        rm_identify: Some(rm_identify),
        rm_startup: Some(rm_startup),
        rm_cleanup: Some(rm_cleanup),
        rm_mask: Some(rm_mask),
        // Postgres decode.c line 77: every record's xid needs to be processed by reorderbuffer
        // If this is set to none, Postgres will invoke reorderbuffer for us
        rm_decode: None,
    };

    unsafe {
        pg_sys::RegisterCustomRmgr(RMGR_ID, &mut rmgr as *mut _);
    }
}

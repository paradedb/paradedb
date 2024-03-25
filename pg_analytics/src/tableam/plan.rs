#[cfg(any(feature = "pg12", feature = "pg13"))]
use core::ffi::c_int;
use pgrx::*;
use std::ptr::addr_of_mut;

use super::create::{TableMetadata, FIRST_BLOCK_NUMBER};

#[pg_guard]
pub extern "C" fn deltalake_relation_nontransactional_truncate(_rel: pg_sys::Relation) {
    todo!()
}

#[pg_guard]
pub extern "C" fn deltalake_relation_size(
    rel: pg_sys::Relation,
    fork_number: pg_sys::ForkNumber,
) -> pg_sys::uint64 {
    unsafe {
        if (*rel).rd_smgr.is_null() {
            #[cfg(feature = "pg16")]
            pg_sys::smgrsetowner(
                addr_of_mut!((*rel).rd_smgr),
                pg_sys::smgropen((*rel).rd_locator, (*rel).rd_backend),
            );
            #[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
            pg_sys::smgrsetowner(
                addr_of_mut!((*rel).rd_smgr),
                pg_sys::smgropen((*rel).rd_node, (*rel).rd_backend),
            );
        }

        let mut nblocks: pg_sys::uint64 = 0;

        match fork_number {
            pg_sys::ForkNumber_InvalidForkNumber => {
                for i in 0..pg_sys::ForkNumber_INIT_FORKNUM {
                    nblocks += pg_sys::smgrnblocks((*rel).rd_smgr, i) as pg_sys::uint64;
                }
            }
            fork_number => {
                nblocks = pg_sys::smgrnblocks((*rel).rd_smgr, fork_number) as pg_sys::uint64;
            }
        };

        nblocks * pg_sys::BLCKSZ as pg_sys::uint64
    }
}

#[pg_guard]
pub extern "C" fn deltalake_relation_needs_toast_table(_rel: pg_sys::Relation) -> bool {
    false
}

#[pg_guard]
#[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
pub extern "C" fn deltalake_relation_toast_am(_rel: pg_sys::Relation) -> pg_sys::Oid {
    pg_sys::Oid::INVALID
}

#[pg_guard]
#[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
pub extern "C" fn deltalake_relation_fetch_toast_slice(
    _toastrel: pg_sys::Relation,
    _valueid: pg_sys::Oid,
    _attrsize: pg_sys::int32,
    _sliceoffset: pg_sys::int32,
    _slicelength: pg_sys::int32,
    _result: *mut pg_sys::varlena,
) {
}

#[pg_guard]
pub extern "C" fn deltalake_relation_estimate_size(
    rel: pg_sys::Relation,
    attr_widths: *mut pg_sys::int32,
    pages: *mut pg_sys::BlockNumber,
    tuples: *mut f64,
    allvisfrac: *mut f64,
) {
    unsafe {
        if (*rel).rd_smgr.is_null() {
            #[cfg(feature = "pg16")]
            pg_sys::smgrsetowner(
                addr_of_mut!((*rel).rd_smgr),
                pg_sys::smgropen((*rel).rd_locator, (*rel).rd_backend),
            );
            #[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
            pg_sys::smgrsetowner(
                addr_of_mut!((*rel).rd_smgr),
                pg_sys::smgropen((*rel).rd_node, (*rel).rd_backend),
            );
        }

        // Set tuple count
        let buffer = pg_sys::ReadBufferExtended(
            rel,
            pg_sys::ForkNumber_MAIN_FORKNUM,
            FIRST_BLOCK_NUMBER,
            pg_sys::ReadBufferMode_RBM_NORMAL,
            std::ptr::null_mut(),
        );

        pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_EXCLUSIVE as i32);
        let page = pg_sys::BufferGetPage(buffer);
        let metadata = pg_sys::PageGetSpecialPointer(page) as *mut TableMetadata;
        *tuples = (*metadata).max_row_number as f64;

        pg_sys::MarkBufferDirty(buffer);
        pg_sys::UnlockReleaseBuffer(buffer);

        // Set page count and visibility
        *pages = pg_sys::smgrnblocks((*rel).rd_smgr, pg_sys::ForkNumber_MAIN_FORKNUM);
        *allvisfrac = 1.0;

        pg_sys::get_rel_data_width(rel, attr_widths);
    }
}

#[pg_guard]
#[cfg(any(feature = "pg12", feature = "pg13"))]
pub extern "C" fn deltalake_compute_xid_horizon_for_tuples(
    _rel: pg_sys::Relation,
    _items: *mut pg_sys::ItemPointerData,
    _nitems: c_int,
) -> pg_sys::TransactionId {
    0
}

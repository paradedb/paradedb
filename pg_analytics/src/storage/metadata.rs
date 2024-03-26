use pgrx::*;
use std::ffi::c_void;
use std::mem::size_of;

use super::tid::{FIRST_BLOCK_NUMBER, FIRST_ROW_NUMBER};

pub struct RelationMetadata {
    next_row_number: i64,
}

pub trait PgMetadata {
    fn read_next_row_number(self) -> i64;
    fn write_next_row_number(self, next_row_number: i64);
    fn init_metadata(self, smgr: pg_sys::SMgrRelation);
}

impl PgMetadata for pg_sys::Relation {
    fn read_next_row_number(self) -> i64 {
        unsafe {
            let buffer = pg_sys::ReadBuffer(self, FIRST_BLOCK_NUMBER);

            pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_SHARE as i32);
            let page = pg_sys::BufferGetPage(buffer);
            let metadata = pg_sys::PageGetSpecialPointer(page) as *mut RelationMetadata;
            let next_row_number = (*metadata).next_row_number;
            pg_sys::UnlockReleaseBuffer(buffer);

            next_row_number
        }
    }

    fn write_next_row_number(self, next_row_number: i64) {
        unsafe {
            let buffer = pg_sys::ReadBuffer(self, FIRST_BLOCK_NUMBER);
            let state = pg_sys::GenericXLogStart(self);

            pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_EXCLUSIVE as i32);
            let page = pg_sys::GenericXLogRegisterBuffer(
                state,
                buffer,
                pg_sys::GENERIC_XLOG_FULL_IMAGE as i32,
            );

            let metadata = pg_sys::PageGetSpecialPointer(page) as *mut RelationMetadata;
            (*metadata).next_row_number = next_row_number;

            pg_sys::GenericXLogFinish(state);
            pg_sys::UnlockReleaseBuffer(buffer);
        }
    }

    fn init_metadata(self, smgr: pg_sys::SMgrRelation) {
        unsafe {
            let mut buffer = pg_sys::ReadBufferExtended(
                self,
                pg_sys::ForkNumber_MAIN_FORKNUM,
                pg_sys::InvalidBlockNumber,
                pg_sys::ReadBufferMode_RBM_NORMAL,
                std::ptr::null_mut(),
            );

            if pg_sys::BufferGetBlockNumber(buffer) != FIRST_BLOCK_NUMBER {
                buffer = pg_sys::ReadBuffer(self, FIRST_BLOCK_NUMBER);
            }

            let state = pg_sys::GenericXLogStart(self);

            pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_EXCLUSIVE as i32);
            let page = pg_sys::GenericXLogRegisterBuffer(
                state,
                buffer,
                pg_sys::GENERIC_XLOG_FULL_IMAGE as i32,
            );

            pg_sys::PageInit(page, pg_sys::BLCKSZ as usize, size_of::<RelationMetadata>());
            pg_sys::smgrextend(
                smgr,
                pg_sys::ForkNumber_MAIN_FORKNUM,
                FIRST_BLOCK_NUMBER,
                page as *const c_void,
                true,
            );

            let metadata = pg_sys::PageGetSpecialPointer(page) as *mut RelationMetadata;
            (*metadata).next_row_number = FIRST_ROW_NUMBER;

            pg_sys::GenericXLogFinish(state);
            pg_sys::UnlockReleaseBuffer(buffer);
        }
    }
}

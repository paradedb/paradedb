/*
    Uses Postgres block storage to store metadata about a table.
    Specifically, it stores the next row number to be used in the table.
    read_next_row_number reads the next row number from the metadata, and
    write_next_row_number writes the next row number to the metadata.
    init_metadata initializes the metadata for a table.
*/

use pgrx::*;
use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::addr_of_mut;
use thiserror::Error;

use super::tid::{FIRST_ROW_NUMBER, LSN_BLOCKNO, METADATA_BLOCKNO};

pub struct RelationMetadata {
    next_row_number: i64,
}

pub trait PgMetadata {
    fn read_next_row_number(self) -> Result<i64, MetadataError>;
    fn write_next_row_number(self, next_row_number: i64) -> Result<(), MetadataError>;
    fn get_lsn_buffer(self) -> Result<i32, MetadataError>;
    fn get_metadata_buffer(self) -> Result<i32, MetadataError>;
    fn init_metadata(self, smgr: pg_sys::SMgrRelation) -> Result<(), MetadataError>;
}

impl PgMetadata for pg_sys::Relation {
    fn read_next_row_number(self) -> Result<i64, MetadataError> {
        unsafe {
            let buffer = Self::get_metadata_buffer(self)?;
            pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_SHARE as i32);

            let page = pg_sys::BufferGetPage(buffer);
            let metadata = pg_sys::PageGetSpecialPointer(page) as *mut RelationMetadata;
            let next_row_number = (*metadata).next_row_number;
            pg_sys::UnlockReleaseBuffer(buffer);

            Ok(next_row_number)
        }
    }

    fn write_next_row_number(self, next_row_number: i64) -> Result<(), MetadataError> {
        unsafe {
            let buffer = Self::get_metadata_buffer(self)?;
            pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_EXCLUSIVE as i32);

            let state = pg_sys::GenericXLogStart(self);
            let page = pg_sys::GenericXLogRegisterBuffer(
                state,
                buffer,
                pg_sys::GENERIC_XLOG_FULL_IMAGE as i32,
            );

            let metadata = pg_sys::PageGetSpecialPointer(page) as *mut RelationMetadata;
            (*metadata).next_row_number = next_row_number;

            pg_sys::GenericXLogFinish(state);
            pg_sys::UnlockReleaseBuffer(buffer);

            Ok(())
        }
    }

    fn get_metadata_buffer(self) -> Result<i32, MetadataError> {
        unsafe {
            if (*self).rd_smgr.is_null() {
                #[cfg(feature = "pg16")]
                pg_sys::smgrsetowner(
                    addr_of_mut!((*self).rd_smgr),
                    pg_sys::smgropen((*self).rd_locator, (*self).rd_backend),
                );
                #[cfg(any(
                    feature = "pg12",
                    feature = "pg13",
                    feature = "pg14",
                    feature = "pg15"
                ))]
                pg_sys::smgrsetowner(
                    addr_of_mut!((*self).rd_smgr),
                    pg_sys::smgropen((*self).rd_node, (*self).rd_backend),
                );
            }

            let nblocks = pg_sys::smgrnblocks((*self).rd_smgr, pg_sys::ForkNumber_MAIN_FORKNUM);

            if nblocks == 0 {
                return Err(MetadataError::MetadataNotFound);
            }

            Ok(pg_sys::ReadBuffer(self, METADATA_BLOCKNO))
        }
    }

    fn get_lsn_buffer(self) -> Result<i32, MetadataError> {
        unsafe {
            if (*self).rd_smgr.is_null() {
                #[cfg(feature = "pg16")]
                pg_sys::smgrsetowner(
                    addr_of_mut!((*self).rd_smgr),
                    pg_sys::smgropen((*self).rd_locator, (*self).rd_backend),
                );
                #[cfg(any(
                    feature = "pg12",
                    feature = "pg13",
                    feature = "pg14",
                    feature = "pg15"
                ))]
                pg_sys::smgrsetowner(
                    addr_of_mut!((*self).rd_smgr),
                    pg_sys::smgropen((*self).rd_node, (*self).rd_backend),
                );
            }

            let nblocks = pg_sys::smgrnblocks((*self).rd_smgr, pg_sys::ForkNumber_MAIN_FORKNUM);

            if nblocks < 2 {
                return Err(MetadataError::LsnNotFound);
            }

            Ok(pg_sys::ReadBuffer(self, LSN_BLOCKNO))
        }
    }

    fn init_metadata(self, smgr: pg_sys::SMgrRelation) -> Result<(), MetadataError> {
        unsafe {
            let nblocks = pg_sys::smgrnblocks(smgr, pg_sys::ForkNumber_MAIN_FORKNUM);

            if nblocks > 0 {
                return Err(MetadataError::MetadataAlreadyExists(nblocks));
            }

            let mut block: pg_sys::PGAlignedBlock = Default::default();
            let page = block.data.as_mut_ptr();

            pg_sys::PageInit(page, pg_sys::BLCKSZ as usize, size_of::<RelationMetadata>());

            let metadata = pg_sys::PageGetSpecialPointer(page) as *mut RelationMetadata;
            (*metadata).next_row_number = FIRST_ROW_NUMBER;

            for blockno in vec![METADATA_BLOCKNO, LSN_BLOCKNO] {
                #[cfg(feature = "pg16")]
                pg_sys::log_newpage(
                    addr_of_mut!((*smgr).smgr_rlocator.locator),
                    pg_sys::ForkNumber_MAIN_FORKNUM,
                    blockno,
                    page,
                    true,
                );
                #[cfg(any(
                    feature = "pg12",
                    feature = "pg13",
                    feature = "pg14",
                    feature = "pg15"
                ))]
                pg_sys::log_newpage(
                    addr_of_mut!((*smgr).smgr_rnode.node),
                    pg_sys::ForkNumber_MAIN_FORKNUM,
                    blockno,
                    page,
                    true,
                );

                pg_sys::PageSetChecksumInplace(page, METADATA_BLOCKNO);

                #[cfg(feature = "pg16")]
                pg_sys::smgrextend(
                    smgr,
                    pg_sys::ForkNumber_MAIN_FORKNUM,
                    blockno,
                    page as *const c_void,
                    true,
                );

                #[cfg(any(
                    feature = "pg12",
                    feature = "pg13",
                    feature = "pg14",
                    feature = "pg15"
                ))]
                pg_sys::smgrextend(
                    smgr,
                    pg_sys::ForkNumber_MAIN_FORKNUM,
                    blockno,
                    page as *mut std::ffi::c_char,
                    true,
                );
            }

            pg_sys::smgrimmedsync(smgr, pg_sys::ForkNumber_MAIN_FORKNUM);

            Ok(())
        }
    }
}

#[derive(Error, Debug)]
pub enum MetadataError {
    #[error("Unexpected error: {0} blocks already exist when creating table metadata")]
    MetadataAlreadyExists(u32),

    #[error("Unexpected error: Table metadata not found")]
    MetadataNotFound,

    #[error("Unexpected error: LSN block not found")]
    LsnNotFound,
}

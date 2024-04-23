use pgrx::*;
use std::ffi::c_char;

static INVALID_SUBTRANSACTION_ID: pg_sys::SubTransactionId = 0;
pub const XLOG_INSERT: u8 = 0x00;
// pub const XLOG_DELETE: u8 = 0x10;
// pub const XLOG_UPDATE: u8 = 0x20;
pub const XLOG_TRUNCATE: u8 = 0x30;

pub enum XLogEntry {
    Insert,
    // Update,
    // Delete,
    Truncate,
    Unknown,
}

impl XLogEntry {
    pub fn to_str(&self) -> &'static str {
        match self {
            XLogEntry::Insert => "INSERT",
            // XLogEntry::Update => "UPDATE",
            // XLogEntry::Delete => "DELETE",
            XLogEntry::Truncate => "TRUNCATE",
            XLogEntry::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone)]
pub struct XLogInsertRecord {
    // For now, flags is unused
    flags: u8,
}

impl XLogInsertRecord {
    pub fn new(flags: u8) -> Self {
        Self { flags }
    }

    pub fn flags(&self) -> u8 {
        self.flags
    }
}

#[derive(Debug, Clone)]
pub struct XLogTruncateRecord {
    relid: pg_sys::Oid,
}

impl XLogTruncateRecord {
    pub fn new(relid: pg_sys::Oid) -> Self {
        Self { relid }
    }

    pub fn relid(&self) -> pg_sys::Oid {
        self.relid
    }
}

/// Rust implementations of Postgres functions in src/include/utils/rel.h
/// related to Write-Ahead Logging (WAL).
///
/// These can be contributed to pgrx.
unsafe fn xlog_is_needed() -> bool {
    pg_sys::wal_level >= pg_sys::WalLevel_WAL_LEVEL_REPLICA as i32
}

unsafe fn relation_is_permanent(rel: pg_sys::Relation) -> bool {
    (*(*rel).rd_rel).relpersistence == pg_sys::RELPERSISTENCE_PERMANENT as i8
}

/// # Safety
/// This function is unsafe because it calls pg_sys functions
pub unsafe fn relation_needs_wal(rel: pg_sys::Relation) -> bool {
    // #define RelationNeedsWAL(relation)							        \
    // (RelationIsPermanent(relation) && (XLogIsNeeded() ||				    \
    //   (relation->rd_createSubid == InvalidSubTransactionId &&			\
    //    relation->rd_firstRelfilelocatorSubid == InvalidSubTransactionId)))
    #[cfg(feature = "pg12")]
    {
        relation_is_permanent(rel)
    }

    #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15"))]
    {
        relation_is_permanent(rel)
            && (xlog_is_needed()
                || ((*rel).rd_createSubid == INVALID_SUBTRANSACTION_ID
                    && (*rel).rd_firstRelfilenodeSubid == INVALID_SUBTRANSACTION_ID))
    }

    #[cfg(feature = "pg16")]
    {
        relation_is_permanent(rel)
            && (xlog_is_needed()
                || ((*rel).rd_createSubid == INVALID_SUBTRANSACTION_ID
                    && (*rel).rd_firstRelfilelocatorSubid == INVALID_SUBTRANSACTION_ID))
    }
}

/// # Safety
/// This function is unsafe because it calls pg_sys functions
pub unsafe fn xlog_rec_get_info(record: *mut pg_sys::XLogReaderState) -> u8 {
    #[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
    {
        (*(*record).decoded_record).xl_info
    }

    #[cfg(feature = "pg16")]
    {
        (*(*record).record).header.xl_info
    }
}

/// # Safety
/// This function is unsafe because it calls pg_sys functions
pub unsafe fn xlog_rec_get_data(record: *mut pg_sys::XLogReaderState) -> *mut c_char {
    #[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
    {
        (*record).main_data
    }

    #[cfg(feature = "pg16")]
    {
        (*(*record).record).main_data
    }
}

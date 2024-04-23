use pgrx::*;

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

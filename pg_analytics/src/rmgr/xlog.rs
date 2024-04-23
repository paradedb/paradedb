use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use std::ffi::CStr;

pub const XLOG_INSERT: u8 = 0x00;
pub const _XLOG_DELETE: u8 = 0x10;
pub const _XLOG_UPDATE: u8 = 0x20;
pub const XLOG_TRUNCATE: u8 = 0x30;

pub enum XLogEntry {
    Insert,
    Update,
    Delete,
    Truncate,
    Unknown,
}

impl XLogEntry {
    pub fn to_str(&self) -> &'static str {
        match self {
            XLogEntry::Insert => "INSERT",
            XLogEntry::Update => "UPDATE",
            XLogEntry::Delete => "DELETE",
            XLogEntry::Truncate => "TRUNCATE",
            XLogEntry::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone)]
pub struct XLogInsertRecord {
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

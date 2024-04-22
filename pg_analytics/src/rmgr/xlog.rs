use pgrx::*;
use pgrx::pg_sys::AsPgCStr;
use std::ffi::CStr;

pub static XLOG_INSERT: u8 = 0x00;

#[derive(Debug, Clone)]
pub struct XLogInsertRecord {
    flags: u8,
    schema_oid: pg_sys::Oid,
    tablespace_oid: pg_sys::Oid,
    xmin: u32
}

impl XLogInsertRecord {
    pub fn new(flags: u8, schema_oid: pg_sys::Oid, tablespace_oid: pg_sys::Oid, xmin: u32) -> Self {
        Self {
            flags,
            schema_oid,
            tablespace_oid,
            xmin
        }
    }

    pub fn flags(&self) -> u8 {
        self.flags
    }

    pub fn schema_oid(&self) -> pg_sys::Oid {
        self.schema_oid
    }

    pub fn tablespace_oid(&self) -> pg_sys::Oid {
        self.tablespace_oid
    }

    pub fn xmin(&self) -> u32 {
        self.xmin
    }
}

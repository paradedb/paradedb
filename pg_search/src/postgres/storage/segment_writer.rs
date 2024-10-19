use pgrx::*;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct SegmentWriter {
    relation_oid: u32,
    last_blockno: Option<pg_sys::BlockNumber>,
    offset_blockno: Option<pg_sys::OffsetNumber>,
    path: PathBuf
}

pub struct NextSegmentAddress {
    pub blockno: pg_sys::BlockNumber,
    pub offsetno: pg_sys::OffsetNumber,
}

impl SegmentWriter {
    pub unsafe fn new(relation_oid: u32, path: &Path) -> Self {
        Self {
            relation_oid,
            blockno: None,
            offsetno: None,
            path: path.to_path_buf()
        }
    }
}

impl Write for SegmentWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        unsafe {
            pgrx::info!("Writing data for {}", self.path);

            Ok(data.len())
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        pgrx::info!("Flushing");
        Ok(())
    }
}

use crate::index::channel::NeedWal;
use crate::postgres::storage::block::{DirectoryEntry, DIRECTORY_START};
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use pgrx::*;
use std::io::{Result, Write};
use std::path::{Path, PathBuf};
use tantivy::directory::{AntiCallToken, TerminatingWrite};

#[derive(Clone, Debug)]
pub struct SegmentComponentWriter {
    relation_oid: pg_sys::Oid,
    path: PathBuf,
    need_wal: NeedWal,
    header_blockno: pg_sys::BlockNumber,
    total_bytes: usize,
}

impl SegmentComponentWriter {
    pub unsafe fn new(relation_oid: pg_sys::Oid, path: &Path, need_wal: NeedWal) -> Self {
        let segment_component = LinkedBytesList::create(relation_oid, need_wal);

        Self {
            relation_oid,
            path: path.to_path_buf(),
            need_wal,
            header_blockno: segment_component.header_blockno,
            total_bytes: 0,
        }
    }
}

impl Write for SegmentComponentWriter {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        let mut segment_component =
            LinkedBytesList::open(self.relation_oid, self.header_blockno, self.need_wal);
        unsafe { segment_component.write(data).expect("write should succeed") };
        self.total_bytes += data.len();
        Ok(data.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl TerminatingWrite for SegmentComponentWriter {
    fn terminate_ref(&mut self, _: AntiCallToken) -> Result<()> {
        unsafe {
            let mut directory = LinkedItemList::<DirectoryEntry>::open(
                self.relation_oid,
                DIRECTORY_START,
                self.need_wal,
            );
            let entry = DirectoryEntry {
                path: self.path.clone(),
                total_bytes: self.total_bytes,
                start: self.header_blockno,
                xmin: pg_sys::GetCurrentTransactionId(),
                xmax: pg_sys::InvalidTransactionId,
            };
            directory
                .add_items(vec![entry])
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }

        Ok(())
    }
}

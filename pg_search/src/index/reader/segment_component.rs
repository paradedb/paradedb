use anyhow::Result;
use pgrx::*;
use std::io::{Error, ErrorKind};
use std::ops::Range;
use std::slice::from_raw_parts;
use tantivy::directory::FileHandle;
use tantivy::directory::OwnedBytes;
use tantivy::HasLen;

use crate::postgres::storage::block::{bm25_max_free_space, DirectoryEntry, LinkedList};
use crate::postgres::storage::utils::BM25BufferCache;
use crate::postgres::storage::LinkedBytesList;

#[derive(Debug)]
pub struct SegmentComponentReader {
    block_list: LinkedBytesList,
    entry: DirectoryEntry,
    relation_oid: pg_sys::Oid,
}

impl SegmentComponentReader {
    pub unsafe fn new(relation_oid: pg_sys::Oid, entry: DirectoryEntry) -> Self {
        let block_list = LinkedBytesList::open(relation_oid, entry.start);

        Self {
            block_list,
            entry,
            relation_oid,
        }
    }
}

impl FileHandle for SegmentComponentReader {
    fn read_bytes(&self, range: Range<usize>) -> Result<OwnedBytes, Error> {
        unsafe {
            const ITEM_SIZE: usize = bm25_max_free_space();
            let start = range.start;
            let end = range.end.min(self.len());
            if start > end {
                return Err(Error::new(ErrorKind::InvalidInput, "Invalid range"));
            }
            let start_block = start / ITEM_SIZE;

            if start_block == self.block_list.npages() as usize {
                // short circuit direct access if the caller is specifically asking for the last page
                let cache = BM25BufferCache::open(self.relation_oid);
                let buffer = cache.get_buffer(
                    self.block_list.get_last_blockno(),
                    Some(pg_sys::BUFFER_LOCK_SHARE),
                );
                let page = pg_sys::BufferGetPage(buffer);
                let slice_start = start % ITEM_SIZE;
                let slice_end = end % ITEM_SIZE;
                let slice_len = slice_end - slice_start;
                let header_size = std::mem::offset_of!(pg_sys::PageHeaderData, pd_linp);
                let slice =
                    from_raw_parts((page as *mut u8).add(slice_start + header_size), slice_len);
                let data = OwnedBytes::new(slice.to_vec());
                pg_sys::UnlockReleaseBuffer(buffer);
                return Ok(data);
            }

            let data = self.block_list.get_bytes_range(range.clone())?;
            Ok(OwnedBytes::new(data))
        }
    }
}

impl HasLen for SegmentComponentReader {
    fn len(&self) -> usize {
        self.entry.total_bytes
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;

    use crate::index::blocking::BlockingDirectory;
    use crate::index::writer::segment_component::SegmentComponentWriter;
    use std::io::Write;
    use std::path::Path;
    use tantivy::directory::TerminatingWrite;

    #[pg_test]
    unsafe fn test_segment_component_read_bytes() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();

        let bytes: Vec<u8> = (1..=255).cycle().take(100_000).collect();
        let segment = format!("{}.term", uuid::Uuid::new_v4());
        let path = Path::new(segment.as_str());

        let mut writer = unsafe { SegmentComponentWriter::new(relation_oid, path) };
        writer.write_all(&bytes).unwrap();
        writer.terminate().unwrap();

        let directory = BlockingDirectory::new(relation_oid);
        let (entry, _, _) = unsafe {
            directory
                .directory_lookup(&path)
                .expect("open directory entry should succeed")
        };
        let reader = SegmentComponentReader::new(relation_oid, entry.clone());

        assert_eq!(reader.len(), 100_000);
        assert_eq!(
            reader.read_bytes(99_998..100_000).unwrap().as_ref(),
            &bytes[99_998..100_000]
        );
        assert_eq!(
            reader.read_bytes(99_999..100_001).unwrap().as_ref(),
            &bytes[99_999..100_000]
        );
        assert_eq!(
            reader.read_bytes(0..100_000).unwrap().as_ref(),
            &bytes[0..100_000]
        );
    }
}

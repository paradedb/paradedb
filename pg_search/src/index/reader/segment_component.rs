use anyhow::Result;
use pgrx::*;
use std::io::{Error, ErrorKind};
use std::ops::Range;
use std::slice::from_raw_parts;
use tantivy::directory::FileHandle;
use tantivy::directory::OwnedBytes;
use tantivy::HasLen;

use crate::postgres::storage::block::{
    bm25_max_free_space, BlockNumberList, MetaPageData, SegmentComponentOpaque, METADATA_BLOCKNO,
};
use crate::postgres::storage::linked_list::{LinkedBytesList, LinkedItemList};
use crate::postgres::storage::utils::BM25BufferCache;

#[derive(Clone, Debug)]
pub struct SegmentComponentReader {
    opaque: SegmentComponentOpaque,
    relation_oid: pg_sys::Oid,
    blocks: Vec<pg_sys::BlockNumber>,
}

impl SegmentComponentReader {
    pub unsafe fn new(relation_oid: pg_sys::Oid, opaque: SegmentComponentOpaque) -> Self {
        let block_list = LinkedBytesList::open(relation_oid, opaque.start);
        let blocks: BlockNumberList = block_list.read_all().into();

        Self {
            opaque,
            relation_oid,
            blocks: blocks.0,
        }
    }
}

impl FileHandle for SegmentComponentReader {
    fn read_bytes(&self, range: Range<usize>) -> Result<OwnedBytes, Error> {
        unsafe {
            const ITEM_SIZE: usize = unsafe { bm25_max_free_space() };
            let cache = BM25BufferCache::open(self.relation_oid);
            let start = range.start;
            let end = range.end.min(self.len());
            if start >= end {
                return Err(Error::new(ErrorKind::InvalidInput, "Invalid range"));
            }
            let start_block = start / ITEM_SIZE;
            let end_block = end / ITEM_SIZE;
            let mut data: Vec<u8> = vec![];

            for (i, blockno) in self
                .blocks
                .iter()
                .enumerate()
                .take(end_block + 1)
                .skip(start_block)
            {
                let buffer = cache.get_buffer(*blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
                let page = pg_sys::BufferGetPage(buffer);
                let slice_start = if i == start_block {
                    start % ITEM_SIZE
                } else {
                    0
                };
                let slice_end = if i == end_block {
                    end % ITEM_SIZE
                } else {
                    ITEM_SIZE
                };
                let slice_len = slice_end - slice_start;
                let header_size = std::mem::offset_of!(pg_sys::PageHeaderData, pd_linp);
                let slice =
                    from_raw_parts((page as *mut u8).add(slice_start + header_size), slice_len);
                data.extend_from_slice(slice);

                pg_sys::UnlockReleaseBuffer(buffer);
            }

            Ok(OwnedBytes::new(data))
        }
    }
}

impl HasLen for SegmentComponentReader {
    fn len(&self) -> usize {
        self.opaque.total_bytes
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

    fn setup_index() -> pg_sys::Oid {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let index_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();

        index_oid
    }

    #[pg_test]
    unsafe fn test_segment_component_read_bytes() {
        let bytes: Vec<u8> = (1..=255).cycle().take(100_000).collect();
        let index_oid = setup_index();
        let segment = format!("{}.term", uuid::Uuid::new_v4());
        let path = Path::new(segment.as_str());

        let mut writer = unsafe { SegmentComponentWriter::new(index_oid, path) };
        writer.write_all(&bytes).unwrap();
        writer.terminate().unwrap();

        let directory = BlockingDirectory::new(index_oid);
        let (opaque, _, _) = unsafe {
            directory
                .lookup_segment_component(&path)
                .expect("open segment component opaque should succeed")
        };
        let reader = SegmentComponentReader::new(index_oid, opaque.clone());

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

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::FileEntry;
use crate::postgres::storage::linked_bytes::RangeData;
use crate::postgres::storage::LinkedBytesList;
use anyhow::Result;
use std::io::Error;
use std::ops::Range;
use tantivy::directory::FileHandle;
use tantivy::directory::OwnedBytes;
use tantivy::HasLen;

#[derive(Debug)]
pub struct SegmentComponentReader {
    block_list: LinkedBytesList,
    entry: FileEntry,
}

impl SegmentComponentReader {
    pub unsafe fn new(indexrel: &PgSearchRelation, entry: FileEntry) -> Self {
        let block_list = LinkedBytesList::open(indexrel, entry.starting_block);

        Self { block_list, entry }
    }

    fn read_bytes_raw(&self, range: Range<usize>) -> Result<RangeData, Error> {
        unsafe {
            let end = range.end.min(self.len());
            let range = range.start..end;

            // read one or more pages
            Ok(self.block_list.get_bytes_range(range))
        }
    }
}

impl FileHandle for SegmentComponentReader {
    fn read_bytes(&self, range: Range<usize>) -> Result<OwnedBytes, Error> {
        let range_data = self.read_bytes_raw(range)?;
        let bytes =
            unsafe { std::slice::from_raw_parts(range_data.as_ptr(), range_data.len()).to_vec() };
        Ok(OwnedBytes::new(bytes))
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

    use crate::index::writer::segment_component::SegmentComponentWriter;
    use crate::postgres::rel::PgSearchRelation;
    use pgrx::*;
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
        let indexrel = PgSearchRelation::open(relation_oid);

        let bytes: Vec<u8> = (1..=255).cycle().take(100_000).collect();
        let segment = format!("{}.term", uuid::Uuid::new_v4());
        let path = Path::new(segment.as_str());

        let mut writer = unsafe { SegmentComponentWriter::new(&indexrel, path) };
        writer.write_all(&bytes).unwrap();
        let file_entry = writer.file_entry();
        writer.terminate().unwrap();

        let reader = SegmentComponentReader::new(&indexrel, file_entry);

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

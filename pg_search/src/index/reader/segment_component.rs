// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use crate::index::mvcc::SegmentPins;
use crate::index::reader::io_stats;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::FileEntry;

use crate::postgres::storage::LinkedBytesList;
use anyhow::Result;
use std::io::Error;
use std::ops::Range;
use tantivy::HasLen;
use tantivy::directory::FileHandle;
use tantivy::directory::OwnedBytes;

#[derive(Debug)]
pub struct SegmentComponentReader {
    block_list: LinkedBytesList,
    entry: FileEntry,
    component: Option<tantivy::index::SegmentComponent>,
    segment_pins: Option<SegmentPins>,
}

impl SegmentComponentReader {
    pub unsafe fn new(
        indexrel: &PgSearchRelation,
        entry: FileEntry,
        component: Option<tantivy::index::SegmentComponent>,
        segment_pins: Option<SegmentPins>,
    ) -> Self {
        let block_list = LinkedBytesList::open(indexrel, entry.starting_block);

        Self {
            block_list,
            entry,
            component,
            segment_pins,
        }
    }

    /// Enable endpoint lookups for a published, immutable component file.
    pub fn with_finalized_length(mut self) -> Self {
        self.block_list = self.block_list.with_length(self.entry.total_bytes);
        self
    }

    fn read_bytes_raw(&self, range: Range<usize>) -> Result<OwnedBytes, Error> {
        unsafe {
            let end = range.end.min(self.len());
            let range = range.start..end;

            // read one or more pages
            Ok(self
                .block_list
                .get_bytes_range(range, self.segment_pins.as_ref()))
        }
    }
}

impl FileHandle for SegmentComponentReader {
    fn read_bytes(&self, range: Range<usize>) -> Result<OwnedBytes, Error> {
        match &self.component {
            Some(component) => io_stats::record(component, || self.read_bytes_raw(range)),
            None => self.read_bytes_raw(range),
        }
    }

    fn read_byte(&self, offset: usize) -> Result<u8, Error> {
        let read = || Ok(unsafe { self.block_list.get_byte(offset, self.segment_pins.as_ref()) });
        match &self.component {
            Some(component) => io_stats::record(component, read),
            None => read(),
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

    use crate::index::writer::segment_component::SegmentComponentWriter;
    use crate::postgres::rel::PgSearchRelation;
    use pgrx::*;
    use std::io::Write;
    use std::path::Path;
    use tantivy::directory::TerminatingWrite;

    #[pg_test]
    unsafe fn test_segment_component_read_bytes() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING paradedb (id, data)").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();
        let indexrel = PgSearchRelation::open(relation_oid);

        let page_size = crate::postgres::storage::block::bm25_max_free_space();
        for len in [
            0,
            1,
            page_size - 1,
            page_size,
            page_size + 1,
            2 * page_size,
            100_000,
        ] {
            let bytes: Vec<u8> = (1..=255).cycle().take(len).collect();
            let segment = format!("{}.term", uuid::Uuid::new_v4());
            let path = Path::new(segment.as_str());
            let mut writer = SegmentComponentWriter::new(&indexrel, path);
            writer.write_all(&bytes).unwrap();
            let file_entry = writer.file_entry();
            writer.terminate().unwrap();

            for finalized in [false, true] {
                let mut reader = SegmentComponentReader::new(&indexrel, file_entry, None, None);
                if finalized {
                    reader = reader.with_finalized_length();
                }
                assert_eq!(reader.len(), len);
                let tail = len.saturating_sub(24);
                assert_eq!(
                    reader.read_bytes(tail..len + 1).unwrap().as_ref(),
                    &bytes[tail..]
                );
                for offset in (0..len).step_by(page_size.saturating_sub(1)).rev() {
                    assert_eq!(reader.read_byte(offset).unwrap(), bytes[offset]);
                }
                assert_eq!(reader.read_bytes(0..len).unwrap().as_ref(), bytes);
            }
        }
    }
}

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

use crate::index::mvcc::PinCushion;
use crate::index::reader::io_stats;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{FileEntry, VECTOR_VEC_EXT};

use crate::postgres::storage::LinkedBytesList;
use anyhow::Result;
use parking_lot::Mutex;
use std::io::Error;
use std::ops::Range;
use std::sync::Arc;
use tantivy::HasLen;
use tantivy::directory::FileHandle;
use tantivy::directory::OwnedBytes;

#[derive(Debug)]
pub(crate) enum ReadProtection {
    Unpublished,
    Segment {
        _pins: Arc<Mutex<Option<PinCushion>>>,
    },
    IndexFile,
}

#[derive(Debug)]
pub struct SegmentComponentReader {
    block_list: LinkedBytesList,
    entry: FileEntry,
    component: Option<tantivy::index::SegmentComponent>,
    protection: ReadProtection,
}

impl SegmentComponentReader {
    /// # Safety
    /// IndexFile requires a finalized registry entry and a relation lock covering all reads,
    /// including escaped bytes. Registry payloads are never reclaimed within that lifetime.
    pub(crate) unsafe fn new(
        indexrel: &PgSearchRelation,
        entry: FileEntry,
        component: Option<tantivy::index::SegmentComponent>,
        protection: ReadProtection,
    ) -> Self {
        let block_list = LinkedBytesList::open(indexrel, entry.starting_block);

        Self {
            block_list,
            entry,
            component,
            protection,
        }
    }

    fn read_bytes_raw(&self, range: Range<usize>) -> Result<OwnedBytes, Error> {
        unsafe {
            let end = range.end.min(self.len());
            let range = range.start..end;

            // read one or more pages
            Ok(self
                .block_list
                .get_bytes_range(range, matches!(self.protection, ReadProtection::IndexFile)))
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

    fn read_bytes_chunks(
        &self,
        range: Range<usize>,
        visitor: &mut dyn FnMut(&[u8]),
    ) -> Result<(), Error> {
        let range = range.start..range.end.min(self.len());
        let is_vector = matches!(
            &self.component,
            Some(tantivy::index::SegmentComponent::Custom(ext)) if ext == VECTOR_VEC_EXT
        );
        if is_vector {
            let published = matches!(self.protection, ReadProtection::Segment { .. });
            let mut chunks = unsafe {
                self.block_list
                    .get_bytes_range_page_chunks(range, published)
            };
            loop {
                let chunk = match &self.component {
                    Some(component) => io_stats::record(component, || chunks.next()),
                    None => chunks.next(),
                };
                let Some(chunk) = chunk else {
                    break;
                };
                visitor(chunk.as_ref());
            }
            return Ok(());
        }
        let mut chunks = unsafe {
            self.block_list.get_bytes_range_chunks(
                range,
                false,
                matches!(self.protection, ReadProtection::IndexFile),
            )
        };
        loop {
            let chunk = match &self.component {
                Some(component) => io_stats::record(component, || chunks.next()),
                None => chunks.next(),
            };
            let Some(chunk) = chunk else {
                break;
            };
            visitor(&chunk);
        }
        Ok(())
    }

    fn read_byte(&self, offset: usize) -> Result<u8, Error> {
        let read = || Ok(unsafe { self.block_list.get_byte(offset) });
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
        let bytes: Vec<u8> = (1..=251).cycle().take(page_size * 24 + 13).collect();
        let segment = format!("{}.term", uuid::Uuid::new_v4());
        let path = Path::new(segment.as_str());

        let mut writer = unsafe { SegmentComponentWriter::new(&indexrel, path) };
        writer.write_all(&bytes).unwrap();
        let file_entry = writer.file_entry();
        writer.terminate().unwrap();

        let reader =
            SegmentComponentReader::new(&indexrel, file_entry, None, ReadProtection::Unpublished);

        assert_eq!(reader.len(), bytes.len());
        assert_eq!(
            reader
                .read_bytes(bytes.len() - 2..bytes.len())
                .unwrap()
                .as_ref(),
            &bytes[bytes.len() - 2..]
        );
        assert_eq!(
            reader
                .read_bytes(bytes.len() - 1..bytes.len() + 1)
                .unwrap()
                .as_ref(),
            &bytes[bytes.len() - 1..]
        );
        assert_eq!(reader.read_bytes(0..bytes.len()).unwrap().as_ref(), &bytes);
        let vector_reader = SegmentComponentReader::new(
            &indexrel,
            file_entry,
            Some(tantivy::index::SegmentComponent::Custom(
                VECTOR_VEC_EXT.to_owned(),
            )),
            ReadProtection::Unpublished,
        );
        assert_eq!(
            vector_reader.read_bytes(0..bytes.len()).unwrap().as_ref(),
            &bytes
        );
        let retained = vector_reader.read_bytes(1..33).unwrap();
        let retained_clone = retained.clone();
        for reader in [&reader, &vector_reader] {
            for range in [
                0..0,
                0..bytes.len(),
                page_size - 1..page_size + 1,
                page_size..page_size * 2,
                bytes.len() - 1..bytes.len() + 1,
            ] {
                let mut copied = Vec::new();
                reader
                    .read_bytes_chunks(range.clone(), &mut |chunk| {
                        assert!(!chunk.is_empty());
                        assert!(chunk.len() <= page_size);
                        copied.extend_from_slice(chunk);
                    })
                    .unwrap();
                assert_eq!(copied, bytes[range.start..range.end.min(bytes.len())]);
            }

            let mut copied = Vec::new();
            reader
                .read_bytes_chunks(0..page_size * 2 + 13, &mut |chunk| {
                    if copied.is_empty() {
                        assert_eq!(chunk, &bytes[..page_size]);
                        assert_eq!(
                            reader
                                .read_bytes(page_size..page_size * 19)
                                .unwrap()
                                .as_ref(),
                            &bytes[page_size..page_size * 19]
                        );
                        assert_eq!(chunk, &bytes[..page_size]);

                        let mut nested = Vec::new();
                        reader
                            .read_bytes_chunks(
                                page_size * 20 + 3..page_size * 22 + 11,
                                &mut |part| {
                                    nested.extend_from_slice(part);
                                    assert_eq!(chunk, &bytes[..page_size]);
                                },
                            )
                            .unwrap();
                        assert_eq!(nested, bytes[page_size * 20 + 3..page_size * 22 + 11]);
                        assert_eq!(chunk, &bytes[..page_size]);
                    }
                    copied.extend_from_slice(chunk);
                })
                .unwrap();
            assert_eq!(copied, bytes[..page_size * 2 + 13]);
        }
        drop(vector_reader);
        assert_eq!(retained.as_ref(), &bytes[1..33]);
        assert_eq!(retained_clone.as_ref(), &bytes[1..33]);
    }
}

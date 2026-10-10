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
use crate::postgres::storage::block::{FileEntry, LinkedList, VECTOR_VEC_EXT, bm25_max_free_space};

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
        io_stats: Option<io_stats::ComponentStats>,
    ) -> Self {
        let mut block_list =
            LinkedBytesList::open(indexrel, entry.starting_block).with_length(entry.total_bytes);
        block_list.bman_mut().set_io_stats(io_stats);

        Self {
            block_list,
            entry,
            component,
            protection,
        }
    }

    pub(crate) unsafe fn new_uncommitted(
        indexrel: &PgSearchRelation,
        entry: FileEntry,
        io_stats: Option<io_stats::ComponentStats>,
    ) -> Self {
        let mut block_list = LinkedBytesList::open(indexrel, entry.starting_block);
        block_list.bman_mut().set_io_stats(io_stats);
        Self {
            block_list,
            entry,
            component: None,
            protection: ReadProtection::Unpublished,
        }
    }

    fn read_bytes_raw(&self, range: Range<usize>) -> Result<OwnedBytes, Error> {
        unsafe {
            let end = range.end.min(self.len());
            let range = range.start..end;
            let published = matches!(self.protection, ReadProtection::IndexFile);

            // read one or more pages
            Ok(self.block_list.get_bytes_range(range, published))
        }
    }
}

impl FileHandle for SegmentComponentReader {
    fn read_bytes(&self, range: Range<usize>) -> Result<OwnedBytes, Error> {
        self.read_bytes_raw(range)
    }

    fn read_bytes_chunks(
        &self,
        range: Range<usize>,
        visitor: &mut dyn FnMut(&[u8]),
    ) -> Result<(), Error> {
        let vector = matches!(
            &self.component,
            Some(tantivy::index::SegmentComponent::Custom(ext)) if ext == VECTOR_VEC_EXT
        );
        if vector || matches!(self.protection, ReadProtection::IndexFile) {
            let range = range.start..range.end.min(self.len());
            let published = !matches!(self.protection, ReadProtection::Unpublished);
            for chunk in unsafe {
                self.block_list
                    .get_bytes_range_page_chunks(range, published)
            } {
                visitor(chunk.as_ref());
            }
        } else if !range.is_empty() {
            visitor(&self.read_bytes(range)?);
        }
        Ok(())
    }

    fn read_byte(&self, offset: usize) -> Result<u8, Error> {
        Ok(unsafe { self.block_list.get_byte(offset) })
    }

    fn storage_block_len(&self) -> Option<usize> {
        Some(bm25_max_free_space())
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
    use crate::api::HashMap;
    use crate::index::directory::utils::save_index_files;
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
        let indexrel = PgSearchRelation::with_lock(relation_oid, pg_sys::AccessShareLock as _);

        let page_size = crate::postgres::storage::block::bm25_max_free_space();
        let bytes: Vec<u8> = (1..=251).cycle().take(page_size * 24 + 13).collect();
        let segment = format!("{}.term", uuid::Uuid::new_v4());
        let path = Path::new(segment.as_str());

        let mut writer = unsafe { SegmentComponentWriter::new(&indexrel, path) };
        writer.write_all(&bytes).unwrap();
        let file_entry = writer.file_entry();
        writer.terminate().unwrap();

        let reader = SegmentComponentReader::new(
            &indexrel,
            file_entry,
            None,
            ReadProtection::Unpublished,
            None,
        );

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
            None,
        );
        assert_eq!(
            vector_reader.read_bytes(0..bytes.len()).unwrap().as_ref(),
            &bytes
        );
        let index_path = Path::new("reader-test.bin");
        let mut writer = SegmentComponentWriter::new(&indexrel, index_path);
        writer.write_all(&bytes).unwrap();
        let mut entries = HashMap::default();
        entries.insert(index_path.to_path_buf(), writer.file_entry());
        writer.terminate().unwrap();
        save_index_files(&indexrel, &mut entries, None).unwrap();
        let entry = indexrel.index_file(index_path).unwrap().unwrap();
        let index_reader = SegmentComponentReader::new(
            &indexrel,
            entry.file_entry,
            None,
            ReadProtection::IndexFile,
            None,
        );
        let retained = vector_reader.read_bytes(1..33).unwrap();
        let retained_clone = retained.clone();
        let index_retained = index_reader.read_bytes(1..33).unwrap();
        let index_retained_clone = index_retained.clone();
        for reader in [&reader, &vector_reader, &index_reader] {
            for range in [
                0..0,
                0..bytes.len(),
                page_size - 1..page_size + 1,
                page_size..page_size * 2,
                bytes.len() - 1..bytes.len() + 1,
            ] {
                let copied = reader.read_bytes(range.clone()).unwrap();
                assert_eq!(
                    copied.as_ref(),
                    &bytes[range.start..range.end.min(bytes.len())]
                );
            }
            let mut offset = 0;
            reader
                .read_bytes_chunks(0..bytes.len(), &mut |chunk| {
                    let nested = reader.read_bytes(page_size * 20..page_size * 22).unwrap();
                    assert_eq!(nested.as_ref(), &bytes[page_size * 20..page_size * 22]);
                    assert_eq!(chunk, &bytes[offset..offset + chunk.len()]);
                    offset += chunk.len();
                })
                .unwrap();
            assert_eq!(offset, bytes.len());
            reader
                .read_bytes_chunks(0..0, &mut |_| panic!("empty range visited"))
                .unwrap();
            assert_eq!(reader.storage_block_len(), Some(page_size));
            let retained = reader.read_bytes(0..page_size).unwrap();
            let copied = reader.read_bytes(0..page_size * 2 + 13).unwrap();
            let nested = reader
                .read_bytes(page_size * 20 + 3..page_size * 22 + 11)
                .unwrap();
            assert_eq!(retained.as_ref(), &bytes[..page_size]);
            assert_eq!(copied.as_ref(), &bytes[..page_size * 2 + 13]);
            assert_eq!(
                nested.as_ref(),
                &bytes[page_size * 20 + 3..page_size * 22 + 11]
            );
        }
        drop(vector_reader);
        assert_eq!(retained.as_ref(), &bytes[1..33]);
        assert_eq!(retained_clone.as_ref(), &bytes[1..33]);
        drop(index_reader);
        assert_eq!(index_retained.as_ref(), &bytes[1..33]);
        assert_eq!(index_retained_clone.as_ref(), &bytes[1..33]);
    }

    #[pg_test]
    unsafe fn test_endpoint_reads() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING paradedb (id, data)").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();
        let indexrel = PgSearchRelation::with_lock(relation_oid, pg_sys::AccessShareLock as _);
        let page_size = bm25_max_free_space();
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
            let mut writer = unsafe { SegmentComponentWriter::new(&indexrel, path) };
            writer.write_all(&bytes).unwrap();
            let file_entry = writer.file_entry();
            writer.terminate().unwrap();

            for finalized in [false, true] {
                let reader = if finalized {
                    SegmentComponentReader::new(
                        &indexrel,
                        file_entry,
                        None,
                        ReadProtection::Unpublished,
                        None,
                    )
                } else {
                    SegmentComponentReader::new_uncommitted(&indexrel, file_entry, None)
                };
                assert_eq!(reader.storage_block_len(), Some(page_size));
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

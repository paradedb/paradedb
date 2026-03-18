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

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::FileEntry;
use crate::postgres::storage::dsm_cache;

use crate::postgres::storage::LinkedBytesList;
use anyhow::Result;
use pgrx::pg_sys;
use std::io::Error;
use std::ops::Range;
use tantivy::directory::FileHandle;
use tantivy::directory::OwnedBytes;
use tantivy::HasLen;

/// Optional metadata for DSM caching of segment component data.
pub struct CacheInfo {
    pub index_oid: pg_sys::Oid,
    pub segment_id: [u8; 16],
    pub is_fieldnorm: bool,
}

#[derive(Debug)]
pub struct SegmentComponentReader {
    block_list: LinkedBytesList,
    entry: FileEntry,
    cache_info: Option<CacheInfo>,
}

// CacheInfo contains only Copy types; Debug not needed but we suppress the derive for the parent
impl std::fmt::Debug for CacheInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CacheInfo")
            .field("index_oid", &self.index_oid)
            .field("is_fieldnorm", &self.is_fieldnorm)
            .finish()
    }
}

impl SegmentComponentReader {
    pub unsafe fn new(
        indexrel: &PgSearchRelation,
        entry: FileEntry,
        cache_info: Option<CacheInfo>,
    ) -> Self {
        let block_list = LinkedBytesList::open(indexrel, entry.starting_block);

        Self {
            block_list,
            entry,
            cache_info,
        }
    }

    fn read_bytes_raw(&self, range: Range<usize>) -> Result<OwnedBytes, Error> {
        unsafe {
            let end = range.end.min(self.len());
            let range = range.start..end;

            // Try DSM cache for fieldnorm components
            if let Some(ref info) = self.cache_info {
                if info.is_fieldnorm && !range.is_empty() && crate::gucs::enable_dsm_fieldnorms() {
                    let key = dsm_cache::CacheKey {
                        index_oid: info.index_oid,
                        segment_id: info.segment_id,
                        tag: dsm_cache::CacheTag::FieldNorms,
                        sub_key: range.start as u32,
                    };
                    if let Some(dsm_slice) = dsm_cache::get_or_create(
                        &key,
                        range.len(),
                        |buf| {
                            self.block_list.get_bytes_range_into(range.clone(), buf);
                        },
                    ) {
                        return Ok(dsm_slice.into_owned_bytes());
                    }
                }
            }

            // Fall back to buffer pool read
            Ok(self.block_list.get_bytes_range(range))
        }
    }
}

impl FileHandle for SegmentComponentReader {
    fn read_bytes(&self, range: Range<usize>) -> Result<OwnedBytes, Error> {
        self.read_bytes_raw(range)
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

        let reader = SegmentComponentReader::new(&indexrel, file_entry, None);

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

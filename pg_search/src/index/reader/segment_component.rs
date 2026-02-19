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

use crate::postgres::storage::LinkedBytesList;
use anyhow::Result;
use std::fmt;
use std::io::Error;
use std::ops::{Deref, Range};
use std::sync::{Arc, Mutex, Weak};
use tantivy::directory::FileHandle;
use tantivy::directory::OwnedBytes;
use tantivy::HasLen;

/// Wrapper that adapts `Arc<dyn Deref<Target = [u8]>>` to implement
/// `Deref<Target = [u8]>` directly, as required by `OwnedBytes::new()`.
struct ArcBytes(Arc<dyn Deref<Target = [u8]> + Sync + Send>);

impl Deref for ArcBytes {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.0.deref().deref()
    }
}

// Safety: The data is behind an Arc which is StableDeref. The double-deref
// through the trait object always returns the same pointer for the same Arc.
unsafe impl stable_deref_trait::StableDeref for ArcBytes {}

/// A weak reference to OwnedBytes data that doesn't prevent deallocation.
///
/// Stores a `Weak` pointer to the backing allocation plus the byte offset and
/// length of the slice within that allocation. `upgrade()` succeeds only while
/// at least one strong `OwnedBytes` (or clone) referencing the same allocation
/// is still alive.
struct WeakOwnedBytes {
    weak: Weak<dyn Deref<Target = [u8]> + Sync + Send>,
    offset: usize,
    len: usize,
}

impl WeakOwnedBytes {
    /// Create a weak reference from an existing `OwnedBytes`.
    fn downgrade(bytes: &OwnedBytes) -> Self {
        let arc = bytes.inner_arc();
        let full_slice: &[u8] = arc.deref().deref();
        let data_slice: &[u8] = bytes.as_slice();
        let offset = data_slice.as_ptr() as usize - full_slice.as_ptr() as usize;
        WeakOwnedBytes {
            weak: Arc::downgrade(arc),
            offset,
            len: data_slice.len(),
        }
    }

    /// Try to upgrade the weak reference back to `OwnedBytes`.
    ///
    /// Returns `Some` if the backing data is still alive, `None` if it has
    /// been freed.
    fn upgrade(&self) -> Option<OwnedBytes> {
        let arc = self.weak.upgrade()?;
        let full = OwnedBytes::new(ArcBytes(arc));
        Some(full.slice(self.offset..self.offset + self.len))
    }
}

pub struct SegmentComponentReader {
    block_list: LinkedBytesList,
    entry: FileEntry,
    /// Weak cache for deduplicating repeated reads of the same byte range.
    ///
    /// When multiple callers request the same range (e.g. multiple terms in a
    /// disjunction all reading the same field's fieldnorm data), the first read
    /// allocates and the weak reference is stored. Subsequent reads for the same
    /// range upgrade the weak reference if the data is still alive, avoiding
    /// redundant allocations. The weak reference does not prevent the data from
    /// being freed when all strong references are dropped.
    cache: Mutex<Option<(Range<usize>, WeakOwnedBytes)>>,
}

impl fmt::Debug for SegmentComponentReader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SegmentComponentReader")
            .field("block_list", &self.block_list)
            .field("entry", &self.entry)
            .finish()
    }
}

impl SegmentComponentReader {
    pub unsafe fn new(indexrel: &PgSearchRelation, entry: FileEntry) -> Self {
        let block_list = LinkedBytesList::open(indexrel, entry.starting_block);

        Self {
            block_list,
            entry,
            cache: Mutex::new(None),
        }
    }

    fn read_bytes_raw(&self, range: Range<usize>) -> Result<OwnedBytes, Error> {
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
        // Only cache multi-page reads (> 1 PG page). Single-page reads return
        // sliced ImmutablePage-backed OwnedBytes where the Weak round-trip has issues.
        // Multi-page reads allocate a fresh Vec, which round-trips cleanly.
        const CACHE_THRESHOLD: usize = 8192;

        if range.len() >= CACHE_THRESHOLD {
            // Check if the weak cache has data for this exact range
            let cache = self.cache.lock().unwrap();
            if let Some((cached_range, weak)) = cache.as_ref() {
                if *cached_range == range {
                    if let Some(bytes) = weak.upgrade() {
                        return Ok(bytes);
                    }
                }
            }
        }

        // Cache miss, weak expired, or small read — read fresh
        let bytes = self.read_bytes_raw(range.clone())?;

        if range.len() >= CACHE_THRESHOLD {
            // Store weak reference (doesn't keep the data alive)
            let mut cache = self.cache.lock().unwrap();
            *cache = Some((range, WeakOwnedBytes::downgrade(&bytes)));
        }

        Ok(bytes)
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

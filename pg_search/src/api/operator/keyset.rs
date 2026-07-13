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

//! A memory-bounded set of key-field values, used by the per-row query filter to answer
//! "did this row's key match the query?" without materializing the whole result set in RAM.
//!
//! The set is built once (from the index search) and probed once per scan row. It stays in an
//! in-memory hash set while it fits within `work_mem`; past that it spills to a temporary file:
//! the keys are externally sorted (Postgres `tuplesort`, which itself spills), streamed to a
//! `BufFile` as length-prefixed records, and probed by binary search over a small in-RAM **sparse**
//! index (one entry per `INDEX_STRIDE` records). Both the sort and the record file live on disk and
//! are bounded by `work_mem` / `temp_file_limit`; only the sparse index is resident, and it is
//! `1/INDEX_STRIDE` the size of the key set.
//!
//! Keys are compared by the bytes of their `postcard` serialization. That order is arbitrary (not
//! numeric), but it is *consistent* between the sort side (`bytea` `memcmp`) and the probe side
//! (`[u8]` `Ord`), which is all binary-search membership needs.

use crate::api::HashSet;
use crate::gucs::WorkMem;
use crate::postgres::types::TantivyValue;
use pgrx::pg_sys;
use std::os::raw::c_int;

/// One sparse-index entry per this many spilled records.
const INDEX_STRIDE: usize = 1024;

/// Serialize a key to the bytes used for both sorting and probing.
fn key_bytes(value: &TantivyValue) -> Vec<u8> {
    postcard::to_allocvec(value).expect("a key-field TantivyValue should serialize")
}

/// A built, memory-bounded membership structure over key-field values.
pub enum KeySet {
    /// Every row matches (e.g. `pdb.all()`); no keys are stored.
    All,
    /// No row matches.
    None,
    /// Small enough to keep resident.
    InMemory(HashSet<TantivyValue>),
    /// Spilled to a temporary file, probed via a sparse in-RAM index.
    Spilled(Spilled),
}

impl KeySet {
    /// Does `value` belong to the set?
    pub fn contains(&self, value: &TantivyValue) -> bool {
        match self {
            KeySet::All => true,
            KeySet::None => false,
            KeySet::InMemory(set) => set.contains(value),
            KeySet::Spilled(spilled) => spilled.contains(value),
        }
    }
}

/// Accumulates keys into an in-memory set, transparently spilling to an external sort once the
/// resident size would exceed `work_mem`.
pub struct KeySetBuilder {
    budget: usize,
    resident_bytes: usize,
    in_memory: HashSet<TantivyValue>,
    /// `Some` once we have started spilling; further keys go straight to the sort.
    sort: Option<Sorter>,
}

impl Default for KeySetBuilder {
    fn default() -> Self {
        Self {
            budget: WorkMem::Postgres.bytes(),
            resident_bytes: 0,
            in_memory: HashSet::default(),
            sort: None,
        }
    }
}

impl KeySetBuilder {
    pub fn push(&mut self, value: TantivyValue) {
        if let Some(sort) = self.sort.as_mut() {
            sort.put(&key_bytes(&value));
            return;
        }

        // Rough resident-size estimate: the value plus hash-set slot overhead.
        self.resident_bytes += key_bytes(&value).len() + 48;
        self.in_memory.insert(value);

        if self.resident_bytes > self.budget {
            // Transition to spilling: move everything collected so far into the sorter. The caller
            // detects the resulting `KeySet::Spilled` and warns once per scan.
            let mut sort = Sorter::new(self.budget);
            for v in self.in_memory.drain() {
                sort.put(&key_bytes(&v));
            }
            self.sort = Some(sort);
        }
    }

    pub fn finish(self) -> KeySet {
        match self.sort {
            None => KeySet::InMemory(self.in_memory),
            Some(sort) => KeySet::Spilled(sort.finish()),
        }
    }
}

/// Thin wrapper over a `bytea` `tuplesort` used to externally sort the serialized keys.
struct Sorter {
    state: *mut pg_sys::Tuplesortstate,
}

impl Sorter {
    fn new(budget: usize) -> Self {
        unsafe {
            // `<` operator for bytea, so the sort orders by memcmp of the serialized keys.
            let tcache =
                pg_sys::lookup_type_cache(pg_sys::BYTEAOID, pg_sys::TYPECACHE_LT_OPR as c_int);
            let lt_op = (*tcache).lt_opr;
            let state = pg_sys::tuplesort_begin_datum(
                pg_sys::BYTEAOID,
                lt_op,
                pg_sys::InvalidOid, // bytea has no collation
                false,              // nullsFirstFlag (there are no nulls)
                (budget / 1024).max(64) as c_int,
                std::ptr::null_mut(), // not parallel
                0,                    // sortopt: no random access
            );
            Sorter { state }
        }
    }

    fn put(&mut self, bytes: &[u8]) {
        unsafe {
            let datum = bytea_datum(bytes);
            pg_sys::tuplesort_putdatum(self.state, datum, false);
            // `tuplesort_putdatum` copies the value, so free our transient bytea.
            pg_sys::pfree(datum.cast_mut_ptr());
        }
    }

    /// Drain the sorted keys into a `BufFile` + sparse index.
    fn finish(self) -> Spilled {
        unsafe {
            pg_sys::tuplesort_performsort(self.state);

            // The BufFile is probed on later scan rows, so both the file (via its resource owner)
            // and the BufFile *struct* (via the current memory context) must outlive the current
            // per-tuple ExprContext. Create it against the transaction's owner/context; `Spilled`'s
            // Drop closes it when the function's cache is torn down at end of query.
            let saved_owner = pg_sys::CurrentResourceOwner;
            let saved_cxt = pg_sys::CurrentMemoryContext;
            pg_sys::CurrentResourceOwner = pg_sys::CurTransactionResourceOwner;
            pg_sys::CurrentMemoryContext = pg_sys::CurTransactionContext;
            let file = pg_sys::BufFileCreateTemp(false);
            pg_sys::CurrentResourceOwner = saved_owner;
            pg_sys::CurrentMemoryContext = saved_cxt;
            let mut index: Vec<IndexEntry> = Vec::new();
            let mut count: usize = 0;

            let mut val: pg_sys::Datum = pg_sys::Datum::from(0);
            let mut is_null = false;
            let mut abbrev: pg_sys::Datum = pg_sys::Datum::from(0);
            while tuplesort_getdatum_forward(self.state, &mut val, &mut is_null, &mut abbrev) {
                let bytes = datum_bytea(val);

                if count.is_multiple_of(INDEX_STRIDE) {
                    let (fileno, offset) = buffile_tell(file);
                    index.push(IndexEntry {
                        key: bytes.to_vec(),
                        fileno,
                        offset,
                    });
                }

                let len = bytes.len() as u32;
                buffile_write(file, &len.to_ne_bytes());
                buffile_write(file, bytes);
                count += 1;
            }

            pg_sys::tuplesort_end(self.state);

            Spilled { file, index, count }
        }
    }
}

/// A spilled key set: a temp file of sorted length-prefixed records plus a sparse in-RAM index.
pub struct Spilled {
    file: *mut pg_sys::BufFile,
    index: Vec<IndexEntry>,
    count: usize,
}

struct IndexEntry {
    key: Vec<u8>,
    fileno: c_int,
    offset: pg_sys::off_t,
}

impl Spilled {
    fn contains(&self, value: &TantivyValue) -> bool {
        if self.count == 0 {
            return false;
        }
        let needle = key_bytes(value);

        // Sparse index binary search: find the block whose first key is <= needle (an exact hit on
        // a block-boundary key means the value is present).
        let block = match self
            .index
            .binary_search_by(|e| e.key.as_slice().cmp(&needle))
        {
            Ok(_) => return true,
            Err(0) => return false, // smaller than every key
            Err(pos) => pos - 1,
        };

        // Scan forward from that block's offset, at most INDEX_STRIDE records, until we reach or
        // pass the needle.
        unsafe {
            let entry = &self.index[block];
            pg_sys::BufFileSeek(self.file, entry.fileno, entry.offset, 0 /* SEEK_SET */);
            for _ in 0..INDEX_STRIDE {
                let Some(rec) = self.read_record() else {
                    return false;
                };
                match rec.as_slice().cmp(&needle) {
                    std::cmp::Ordering::Equal => return true,
                    std::cmp::Ordering::Greater => return false,
                    std::cmp::Ordering::Less => continue,
                }
            }
            false
        }
    }

    /// Read one length-prefixed record at the current file position; `None` at EOF.
    unsafe fn read_record(&self) -> Option<Vec<u8>> {
        let mut len_buf = [0u8; 4];
        let n = pg_sys::BufFileRead(self.file, len_buf.as_mut_ptr().cast(), 4);
        if n == 0 {
            return None;
        }
        let len = u32::from_ne_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        buffile_read_exact(self.file, buf.as_mut_ptr().cast(), len);
        Some(buf)
    }
}

impl Drop for Spilled {
    fn drop(&mut self) {
        unsafe { pg_sys::BufFileClose(self.file) }
    }
}

/// Build a transient `bytea` Datum from `bytes` (palloc'd in the current context).
unsafe fn bytea_datum(bytes: &[u8]) -> pg_sys::Datum {
    use pgrx::IntoDatum;
    bytes
        .to_vec()
        .into_datum()
        .expect("byte slice should convert to a bytea datum")
}

/// View a `bytea` Datum's payload as bytes (valid until the datum is freed/overwritten).
unsafe fn datum_bytea<'a>(datum: pg_sys::Datum) -> &'a [u8] {
    let varlena = datum.cast_mut_ptr::<pg_sys::varlena>();
    pgrx::varlena_to_byte_slice(varlena)
}

unsafe fn buffile_tell(file: *mut pg_sys::BufFile) -> (c_int, pg_sys::off_t) {
    let mut fileno: c_int = 0;
    let mut offset: pg_sys::off_t = 0;
    pg_sys::BufFileTell(file, &mut fileno, &mut offset);
    (fileno, offset)
}

// The following wrap Postgres APIs whose signatures differ across supported versions.

/// `tuplesort_getdatum`, always reading forward. `copy: false` -- the returned datum is valid until
/// the next call, which is all we need. (PG16 added the `copy` parameter.)
unsafe fn tuplesort_getdatum_forward(
    state: *mut pg_sys::Tuplesortstate,
    val: *mut pg_sys::Datum,
    is_null: *mut bool,
    abbrev: *mut pg_sys::Datum,
) -> bool {
    #[cfg(feature = "pg15")]
    {
        pg_sys::tuplesort_getdatum(state, true, val, is_null, abbrev)
    }
    #[cfg(not(feature = "pg15"))]
    {
        pg_sys::tuplesort_getdatum(state, true, false, val, is_null, abbrev)
    }
}

/// Write `data` to `file`. (PG15's `BufFileWrite` takes `*mut`; PG16+ takes `*const`.)
unsafe fn buffile_write(file: *mut pg_sys::BufFile, data: &[u8]) {
    #[cfg(feature = "pg15")]
    pg_sys::BufFileWrite(file, data.as_ptr() as *mut std::ffi::c_void, data.len());
    #[cfg(not(feature = "pg15"))]
    pg_sys::BufFileWrite(file, data.as_ptr().cast::<std::ffi::c_void>(), data.len());
}

/// Read exactly `size` bytes into `ptr`. (`BufFileReadExact` was added in PG16; emulate it on PG15.)
unsafe fn buffile_read_exact(file: *mut pg_sys::BufFile, ptr: *mut std::ffi::c_void, size: usize) {
    #[cfg(feature = "pg15")]
    {
        let n = pg_sys::BufFileRead(file, ptr, size);
        assert_eq!(n, size, "short read from spilled key file");
    }
    #[cfg(not(feature = "pg15"))]
    {
        pg_sys::BufFileReadExact(file, ptr, size);
    }
}

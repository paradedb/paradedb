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

//! Shared-buffer reads that first try the buffer that last held the block.
//!
//! A backend-local table maps (relation file, block) to a buffer id. An entry is a hint:
//! `ReadRecentBuffer` pins the buffer only while it is valid and still tagged with the block, and
//! otherwise the read is `ReadBufferExtended`'s.

use std::cell::UnsafeCell;

use pgrx::pg_sys;

/// Slots in a backend's table. A slot is 20 bytes, so the table is 1,280 KiB.
const SLOT_BITS: u32 = 16;

#[derive(Clone, Copy)]
struct Slot {
    file: pg_sys::RelFileLocator,
    block: pg_sys::BlockNumber,
    buffer: pg_sys::Buffer,
}

/// Remembers the buffer that last held a block of a relation file. Holds no pins.
///
/// Direct-mapped: recording a block replaces whatever block shared its slot.
struct RecentBuffers {
    slots: Box<[Slot]>,
}

impl RecentBuffers {
    fn new() -> Self {
        let empty = Slot {
            file: pg_sys::RelFileLocator {
                spcOid: pg_sys::InvalidOid,
                dbOid: pg_sys::InvalidOid,
                relNumber: pg_sys::InvalidOid,
            },
            block: pg_sys::InvalidBlockNumber,
            buffer: pg_sys::InvalidBuffer as pg_sys::Buffer,
        };
        Self {
            slots: vec![empty; 1 << SLOT_BITS].into_boxed_slice(),
        }
    }

    fn get(
        &self,
        file: pg_sys::RelFileLocator,
        block: pg_sys::BlockNumber,
    ) -> Option<pg_sys::Buffer> {
        let slot = &self.slots[slot_index(&file, block)];
        (slot.buffer != pg_sys::InvalidBuffer as pg_sys::Buffer
            && slot.block == block
            && same_file(&slot.file, &file))
        .then_some(slot.buffer)
    }

    fn record(
        &mut self,
        file: pg_sys::RelFileLocator,
        block: pg_sys::BlockNumber,
        buffer: pg_sys::Buffer,
    ) {
        self.slots[slot_index(&file, block)] = Slot {
            file,
            block,
            buffer,
        };
    }
}

fn same_file(a: &pg_sys::RelFileLocator, b: &pg_sys::RelFileLocator) -> bool {
    a.relNumber == b.relNumber && a.dbOid == b.dbOid && a.spcOid == b.spcOid
}

fn slot_index(file: &pg_sys::RelFileLocator, block: pg_sys::BlockNumber) -> usize {
    let file = u64::from(file.relNumber.to_u32())
        ^ u64::from(file.dbOid.to_u32()).rotate_left(21)
        ^ u64::from(file.spcOid.to_u32()).rotate_left(42);
    let mixed = file.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ u64::from(block).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    (mixed >> (64 - SLOT_BITS)) as usize
}

/// The backend's table, created on first use.
struct BackendTable(UnsafeCell<Option<RecentBuffers>>);

// SAFETY: the table is read and written only on the Postgres main thread, which `read` asserts,
// the way the `LinkedBytesList` block cache is.
unsafe impl Sync for BackendTable {}

static TABLE: BackendTable = BackendTable(UnsafeCell::new(None));

#[cfg(any(test, feature = "pg_test"))]
pub(crate) static RECENT_BUFFER_HITS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Pins `blockno` of `rel`'s main fork, as `ReadBufferExtended(rel, MAIN_FORKNUM, blockno,
/// RBM_NORMAL, NULL)` does: a pinned, unlocked shared buffer, counted as that call counts it.
///
/// # Safety
///
/// `rel` must be an open relation that uses shared buffers, and the caller must be in a
/// transaction on the Postgres main thread.
pub(crate) unsafe fn read(rel: pg_sys::Relation, blockno: pg_sys::BlockNumber) -> pg_sys::Buffer {
    debug_assert!(
        crate::postgres::customscan::mpp::interrupt::is_main_thread(),
        "remembered buffers must only be read on the Postgres main thread"
    );
    // SAFETY: only the main thread reaches here and nothing it calls below re-enters `read`, so
    // this is the only reference to the table.
    let table = unsafe { &mut *TABLE.0.get() }.get_or_insert_with(RecentBuffers::new);
    // SAFETY: `rel` is open (caller contract).
    let file = unsafe { (*rel).rd_locator };
    if let Some(buffer) = table.get(file, blockno)
        // SAFETY: `buffer` came from `ReadBufferExtended` in this backend, so it names a shared
        // buffer; `ReadRecentBuffer` pins it only after checking under the buffer header lock that
        // it is valid and tagged with this file, fork and block.
        && unsafe { pg_sys::ReadRecentBuffer(file, pg_sys::ForkNumber::MAIN_FORKNUM, blockno, buffer) }
    {
        // SAFETY: `rel` is open, on the main thread (caller contract).
        unsafe { count_hit(rel) };
        #[cfg(any(test, feature = "pg_test"))]
        RECENT_BUFFER_HITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        return buffer;
    }
    // SAFETY: the caller's contract is `ReadBufferExtended`'s.
    let buffer = unsafe {
        pg_sys::ReadBufferExtended(
            rel,
            pg_sys::ForkNumber::MAIN_FORKNUM,
            blockno,
            pg_sys::ReadBufferMode::RBM_NORMAL,
            std::ptr::null_mut(),
        )
    };
    table.record(file, blockno, buffer);
    buffer
}

/// Counts a hit as `PinBufferForBlock` counts one, beyond `pgBufferUsage.shared_blks_hit`, which
/// `ReadRecentBuffer` counts itself: the relation's fetched and hit blocks, the I/O hit, and the
/// vacuum cost.
///
/// # Safety
///
/// `rel` must be an open relation, on the Postgres main thread.
unsafe fn count_hit(rel: pg_sys::Relation) {
    // SAFETY: `rel` is open, so `pgstat_info` is null or points at its live stats entry, which
    // `pgstat_assoc_relation` sets; the I/O counters and vacuum cost globals are this backend's.
    unsafe {
        if should_count_relation(rel) {
            let counts = &mut (*(*rel).pgstat_info).counts;
            counts.blocks_fetched += 1;
            counts.blocks_hit += 1;
        }
        pg_sys::pgstat_count_io_op(
            pg_sys::IOObject::IOOBJECT_RELATION,
            pg_sys::IOContext::IOCONTEXT_NORMAL,
            pg_sys::IOOp::IOOP_HIT,
            1,
            0,
        );
        if pg_sys::VacuumCostActive {
            pg_sys::VacuumCostBalance += pg_sys::VacuumCostPageHit;
        }
    }
}

/// `pgstat_should_count_relation`: whether `rel` has, or now gets, a stats entry to count into.
///
/// # Safety
///
/// `rel` must be an open relation, on the Postgres main thread.
unsafe fn should_count_relation(rel: pg_sys::Relation) -> bool {
    // SAFETY: `rel` is open (caller contract).
    unsafe {
        if !(*rel).pgstat_info.is_null() {
            true
        } else if (*rel).pgstat_enabled {
            pg_sys::pgstat_assoc_relation(rel);
            true
        } else {
            false
        }
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use std::sync::atomic::Ordering::Relaxed;

    use pgrx::prelude::*;

    use super::*;
    use crate::postgres::rel::PgSearchRelation;
    use crate::postgres::storage::buffer::BufferManager;

    /// Buffer usage and the relation's block counters.
    #[derive(Debug, PartialEq)]
    struct Counts {
        shared_hit: i64,
        shared_read: i64,
        fetched: i64,
        hit: i64,
    }

    fn counts(rel: &PgSearchRelation) -> Counts {
        // SAFETY: this backend's own counters, and `rel` is open.
        unsafe {
            let usage = std::ptr::addr_of!(pg_sys::pgBufferUsage).read();
            let info = (*rel.as_ptr()).pgstat_info;
            let (fetched, hit) = if info.is_null() {
                (0, 0)
            } else {
                ((*info).counts.blocks_fetched, (*info).counts.blocks_hit)
            };
            Counts {
                shared_hit: usage.shared_blks_hit,
                shared_read: usage.shared_blks_read,
                fetched,
                hit,
            }
        }
    }

    fn delta(after: Counts, before: Counts) -> Counts {
        Counts {
            shared_hit: after.shared_hit - before.shared_hit,
            shared_read: after.shared_read - before.shared_read,
            fetched: after.fetched - before.fetched,
            hit: after.hit - before.hit,
        }
    }

    /// The page through the share-locked read, with the remembered-buffer hits and counters it
    /// added.
    fn read(rel: &PgSearchRelation, block: pg_sys::BlockNumber) -> (Vec<u8>, usize, Counts) {
        let hits = RECENT_BUFFER_HITS.load(Relaxed);
        let before = counts(rel);
        let bytes = BufferManager::new(rel)
            .get_buffer(block)
            .page()
            .as_slice()
            .to_vec();
        (
            bytes,
            RECENT_BUFFER_HITS.load(Relaxed) - hits,
            delta(counts(rel), before),
        )
    }

    /// The page through `ReadBufferExtended` alone, with the counters it added.
    fn read_directly(rel: &PgSearchRelation, block: pg_sys::BlockNumber) -> (Vec<u8>, Counts) {
        let before = counts(rel);
        // SAFETY: `rel` is open and `block` exists; the buffer is share-locked while the page is
        // copied, then released.
        let bytes = unsafe {
            let buffer = pg_sys::ReadBufferExtended(
                rel.as_ptr(),
                pg_sys::ForkNumber::MAIN_FORKNUM,
                block,
                pg_sys::ReadBufferMode::RBM_NORMAL,
                std::ptr::null_mut(),
            );
            pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_SHARE as i32);
            // The range `Page::as_slice` covers: the line pointers up to `pd_lower`.
            let page = pg_sys::BufferGetPage(buffer) as *const u8;
            let start = std::mem::offset_of!(pg_sys::PageHeaderData, pd_linp);
            let end = (*(page as *const pg_sys::PageHeaderData)).pd_lower as usize;
            let bytes = std::slice::from_raw_parts(page.add(start), end - start).to_vec();
            pg_sys::UnlockReleaseBuffer(buffer);
            bytes
        };
        (bytes, delta(counts(rel), before))
    }

    fn open(name: &str) -> PgSearchRelation {
        // SAFETY: makes this transaction's catalog changes visible to the lookup below.
        unsafe { pg_sys::CommandCounterIncrement() };
        let oid = Spi::get_one::<pg_sys::Oid>(&format!("SELECT '{name}'::regclass::oid"))
            .unwrap()
            .unwrap();
        PgSearchRelation::open(oid)
    }

    #[pg_test]
    fn reads_fall_back_whenever_the_remembered_buffer_moved_on() {
        Spi::run(
            "CREATE TABLE recent_read (id SERIAL, data TEXT);
             CREATE INDEX recent_read_idx ON recent_read USING paradedb (id, data);
             INSERT INTO recent_read (data) SELECT 'row ' || g FROM generate_series(1, 100) g;",
        )
        .unwrap();
        let rel = open("recent_read_idx");

        // A second read re-pins the remembered buffer, with the bytes and counters of a direct read.
        read(&rel, 0);
        let (bytes, hits, counted) = read(&rel, 0);
        let (direct_bytes, direct_counted) = read_directly(&rel, 0);
        assert_eq!(hits, 1);
        assert_eq!(bytes, direct_bytes);
        assert_eq!(counted, direct_counted);
        assert_eq!(counted.shared_read, 0);

        // A slot naming a buffer that holds another block falls back to the buffer mapping table.
        // SAFETY: the main thread's only reference to the table.
        let table = unsafe { (*TABLE.0.get()).as_mut() }.unwrap();
        // SAFETY: `rel` is open.
        let file = unsafe { (*rel.as_ptr()).rd_locator };
        let block0 = table.get(file, 0).unwrap();
        table.record(file, 1, block0);
        let (bytes, hits, _) = read(&rel, 1);
        assert_eq!(hits, 0);
        assert_eq!(bytes, read_directly(&rel, 1).0);

        // REINDEX gives the index a new file. A slot naming the old file's buffer for the new
        // file's block falls back; the read after it re-pins the recorded buffer.
        drop(rel);
        Spi::run("REINDEX INDEX recent_read_idx").unwrap();
        let rewritten = open("recent_read_idx");
        // SAFETY: `rewritten` is open.
        let rewritten_file = unsafe { (*rewritten.as_ptr()).rd_locator };
        assert!(!same_file(&file, &rewritten_file));
        // SAFETY: the main thread's only reference to the table.
        let table = unsafe { (*TABLE.0.get()).as_mut() }.unwrap();
        table.record(rewritten_file, 0, block0);
        let (bytes, hits, _) = read(&rewritten, 0);
        assert_eq!(hits, 0);
        assert_eq!(bytes, read_directly(&rewritten, 0).0);
        assert_eq!(read(&rewritten, 0).1, 1);
    }

    #[cfg(test)]
    fn file(rel: u32) -> pg_sys::RelFileLocator {
        pg_sys::RelFileLocator {
            spcOid: pg_sys::Oid::from(1663u32),
            dbOid: pg_sys::Oid::from(5u32),
            relNumber: pg_sys::Oid::from(rel),
        }
    }

    #[test]
    fn records_and_returns_a_buffer() {
        let mut recent = RecentBuffers::new();
        assert_eq!(recent.get(file(16384), 7), None);
        recent.record(file(16384), 7, 42);
        assert_eq!(recent.get(file(16384), 7), Some(42));
        assert_eq!(recent.get(file(16384), 8), None);
    }

    #[test]
    fn a_block_sharing_the_slot_replaces_the_entry() {
        let mut recent = RecentBuffers::new();
        let slot = slot_index(&file(16384), 7);
        let other = (8..)
            .find(|&block| slot_index(&file(16384), block) == slot)
            .unwrap();
        recent.record(file(16384), 7, 42);
        recent.record(file(16384), other, 43);
        assert_eq!(recent.get(file(16384), 7), None);
        assert_eq!(recent.get(file(16384), other), Some(43));
    }

    #[test]
    fn another_file_with_the_same_block_misses() {
        let mut recent = RecentBuffers::new();
        recent.record(file(16384), 7, 42);
        assert_eq!(recent.get(file(16385), 7), None);
        let other_database = pg_sys::RelFileLocator {
            dbOid: pg_sys::Oid::from(6u32),
            ..file(16384)
        };
        assert_eq!(recent.get(other_database, 7), None);
    }
}

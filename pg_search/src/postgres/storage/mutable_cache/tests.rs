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

use pgrx::pg_sys;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tantivy::Directory;
use tantivy::directory::RamDirectory;
use tantivy::index::SegmentId;

use super::directory::SharedMemoryDirectory;
use super::pack::{PACKED_MAGIC, pack_segment, unpack_toc};
use super::{
    ActiveReaderGuard, ArenaSpan, CacheSlot, InflightBuild, MAX_INFLIGHT_BUILDS, MAX_SLOTS,
    MutableCacheHeader, MutableCacheKey, SlotState,
};
use crate::postgres::storage::block::{
    MutableSegmentBound, SegmentMetaEntry, SegmentMetaEntryImmutable, SegmentMetaEntryMutable,
};

#[test]
fn test_pack_unpack_and_shmem_directory() {
    let ram_dir = RamDirectory::default();
    let segment_id = SegmentId::generate_random();

    let fast_name = PathBuf::from(format!("{}.fast", segment_id.uuid_string()));
    let term_name = PathBuf::from(format!("{}.term", segment_id.uuid_string()));
    let idx_name = PathBuf::from(format!("{}.idx", segment_id.uuid_string()));

    let fast_data: Vec<u8> = (0..128).map(|x| (x * 3) as u8).collect();
    let term_data: Vec<u8> = (0..256).map(|x| (x * 7) as u8).collect();
    let idx_data: Vec<u8> = (0..64).map(|x| (x * 11) as u8).collect();

    ram_dir.atomic_write(&fast_name, &fast_data).unwrap();
    ram_dir.atomic_write(&term_name, &term_data).unwrap();
    ram_dir.atomic_write(&idx_name, &idx_data).unwrap();

    let packed = pack_segment(&ram_dir, &segment_id).expect("packing must succeed");
    assert!(packed.len() >= fast_data.len() + term_data.len() + idx_data.len());

    let toc = unpack_toc(&packed).expect("TOC unpacking must succeed");
    assert_eq!(toc.len(), 3);
    assert!(toc.contains_key(&fast_name));
    assert!(toc.contains_key(&term_name));
    assert!(toc.contains_key(&idx_name));

    // Verify byte ranges in packed slice
    let fast_range = &toc[&fast_name];
    assert_eq!(&packed[fast_range.clone()], fast_data.as_slice());
    let term_range = &toc[&term_name];
    assert_eq!(&packed[term_range.clone()], term_data.as_slice());
    let idx_range = &toc[&idx_name];
    assert_eq!(&packed[idx_range.clone()], idx_data.as_slice());

    // Test SharedMemoryDirectory
    let reader_guard = Arc::new(ActiveReaderGuard { slot_index: 0 });
    let shmem_dir = SharedMemoryDirectory::new(packed.as_ptr(), packed.len(), reader_guard, toc);

    assert!(shmem_dir.exists(&fast_name).unwrap());
    assert!(shmem_dir.exists(&term_name).unwrap());
    assert!(shmem_dir.exists(&idx_name).unwrap());
    assert!(!shmem_dir.exists(Path::new("non_existent")).unwrap());

    // Atomic read
    assert_eq!(shmem_dir.atomic_read(&fast_name).unwrap(), fast_data);
    assert_eq!(shmem_dir.atomic_read(&term_name).unwrap(), term_data);
    assert_eq!(shmem_dir.atomic_read(&idx_name).unwrap(), idx_data);

    // Open read & sub-slice
    let file_slice = shmem_dir.open_read(&term_name).unwrap();
    assert_eq!(
        file_slice.read_bytes().unwrap().as_slice(),
        term_data.as_slice()
    );
    let sub_slice = file_slice.slice(10..50);
    assert_eq!(
        sub_slice.read_bytes().unwrap().as_slice(),
        &term_data[10..50]
    );

    // Read only: write operations must fail
    assert!(shmem_dir.open_write(Path::new("dummy")).is_err());
    assert!(
        shmem_dir
            .atomic_write(Path::new("dummy"), &[1, 2, 3])
            .is_err()
    );
    assert!(shmem_dir.delete(Path::new("dummy")).is_ok());
}

#[test]
fn test_unpack_toc_invalid_headers() {
    let mut bad_magic = vec![0u8; 128];
    bad_magic[0..4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());
    assert!(unpack_toc(&bad_magic).is_none());

    let mut bad_version = vec![0u8; 128];
    bad_version[0..4].copy_from_slice(&PACKED_MAGIC.to_le_bytes());
    bad_version[4..8].copy_from_slice(&999u32.to_le_bytes());
    assert!(unpack_toc(&bad_version).is_none());

    let truncated = vec![0u8; 8];
    assert!(unpack_toc(&truncated).is_none());
}

#[test]
fn test_mutable_cache_key_from_meta() {
    let segment_id = SegmentId::generate_random();
    let mutable_meta = SegmentMetaEntry::new_mutable(
        segment_id,
        50,
        pg_sys::InvalidTransactionId,
        SegmentMetaEntryMutable {
            header_block: 1,
            num_deleted_docs: 3,
            frozen: false,
        },
    );

    let db_oid = pg_sys::Oid::from(100);
    let index_oid = pg_sys::Oid::from(200);

    let key =
        MutableCacheKey::from_meta(db_oid, index_oid, &mutable_meta).expect("must create key");
    assert_eq!(key.database_oid, db_oid);
    assert_eq!(key.index_oid, index_oid);
    assert_eq!(key.segment_id, *segment_id.uuid_bytes());
    assert_eq!(
        key.bound,
        MutableSegmentBound {
            max_doc: 50,
            num_deleted_docs: 3,
        }
    );

    let immutable_meta = SegmentMetaEntry::new_immutable(
        segment_id,
        50,
        pg_sys::InvalidTransactionId,
        SegmentMetaEntryImmutable::default(),
    );
    assert!(MutableCacheKey::from_meta(db_oid, index_oid, &immutable_meta).is_none());
}

unsafe fn test_header(arena_capacity: u32) -> MutableCacheHeader {
    let mut header: MutableCacheHeader = std::mem::zeroed();
    header.arena_capacity = arena_capacity;
    (*header.slab_pool.get()).init(arena_capacity as usize);
    header
}

#[test]
fn test_clock_sweep_eviction_and_usage_count() {
    let header = unsafe { test_header(1024 * 1024) }; // 1MB arena
    let slots = unsafe { &mut *header.slots.get() };
    let slab_pool = unsafe { &mut *header.slab_pool.get() };

    // Allocate two 512KB slabs (full capacity)
    let s0 = slab_pool.allocate(500 * 1024).unwrap();
    let s1 = slab_pool.allocate(500 * 1024).unwrap();

    // Slot 0: hot (usage_count = 3)
    slots[0] = CacheSlot {
        key: MutableCacheKey::default(),
        span: s0,
        active_readers: AtomicU32::new(0),
        usage_count: AtomicU32::new(3),
        state: SlotState::Ready,
    };

    // Slot 1: cold (usage_count = 0)
    slots[1] = CacheSlot {
        key: MutableCacheKey::default(),
        span: s1,
        active_readers: AtomicU32::new(0),
        usage_count: AtomicU32::new(0),
        state: SlotState::Ready,
    };

    // Allocate another 512KB slab. Slab pool is full, so evict_with_clock_sweep is triggered!
    let s2 = header
        .evict_with_clock_sweep(500 * 1024, slots, slab_pool)
        .expect("must evict cold slot and allocate");

    // Slot 1 (cold, usage_count = 0) must be evicted!
    assert_eq!(slots[1].state, SlotState::Empty);
    // Slot 0 (hot) should still be Ready, but its usage_count decremented!
    assert_eq!(slots[0].state, SlotState::Ready);
    assert_eq!(slots[0].usage_count.load(Ordering::Relaxed), 2);
    // New allocation took Slot 1's freed slab
    assert_eq!(s2.offset, s1.offset);
}

#[test]
fn test_active_readers_block_clock_sweep() {
    let header = unsafe { test_header(1024 * 1024) }; // 1MB arena
    let slots = unsafe { &mut *header.slots.get() };
    let slab_pool = unsafe { &mut *header.slab_pool.get() };

    let s0 = slab_pool.allocate(1000 * 1024).unwrap();
    slots[0] = CacheSlot {
        key: MutableCacheKey::default(),
        span: s0,
        active_readers: AtomicU32::new(2), // 2 readers reading!
        usage_count: AtomicU32::new(0),
        state: SlotState::Ready,
    };

    // Arena is full, and slot 0 has active readers > 0 -> eviction must fail!
    let blocked = header.evict_with_clock_sweep(500 * 1024, slots, slab_pool);
    assert!(blocked.is_none(), "Must not evict slot with active readers");
    assert_eq!(slots[0].state, SlotState::Ready);

    // After readers finish:
    slots[0].active_readers.store(0, Ordering::Release);
    let allowed = header.evict_with_clock_sweep(500 * 1024, slots, slab_pool);
    assert!(allowed.is_some(), "Must evict after readers finish");
    assert_eq!(slots[0].state, SlotState::Empty);
}

#[test]
fn test_eager_reclamation_of_superseded_slots() {
    let header = unsafe { test_header(1024 * 1024) }; // 1MB arena
    let slots = unsafe { &mut *header.slots.get() };
    let slab_pool = unsafe { &mut *header.slab_pool.get() };

    let s0 = slab_pool.allocate(500 * 1024).unwrap();
    slots[0] = CacheSlot {
        key: MutableCacheKey::default(),
        span: s0,
        active_readers: AtomicU32::new(0),
        usage_count: AtomicU32::new(0),
        state: SlotState::Superseded,
    };

    // Superseded slot with 0 readers is reclaimed on the clock sweep immediately
    let s1 = header
        .evict_with_clock_sweep(500 * 1024, slots, slab_pool)
        .expect("must reclaim superseded slot");
    assert_eq!(slots[0].state, SlotState::Empty);
    assert_eq!(s1.offset, s0.offset);
}

#[test]
fn test_slot_invalidation() {
    let mut slots = [const { CacheSlot::empty() }; MAX_SLOTS];
    let seg_a = [1u8; 16];
    let seg_b = [2u8; 16];

    let key_a1 = MutableCacheKey {
        database_oid: pg_sys::Oid::from_u32(10),
        index_oid: pg_sys::Oid::from_u32(100),
        segment_id: seg_a,
        bound: MutableSegmentBound {
            max_doc: 10,
            num_deleted_docs: 0,
        },
    };
    let key_a2 = MutableCacheKey {
        database_oid: pg_sys::Oid::from_u32(10),
        index_oid: pg_sys::Oid::from_u32(100),
        segment_id: seg_a,
        bound: MutableSegmentBound {
            max_doc: 20,
            num_deleted_docs: 0,
        },
    };
    let key_b = MutableCacheKey {
        database_oid: pg_sys::Oid::from_u32(10),
        index_oid: pg_sys::Oid::from_u32(100),
        segment_id: seg_b,
        bound: MutableSegmentBound {
            max_doc: 5,
            num_deleted_docs: 0,
        },
    };
    let key_other_index = MutableCacheKey {
        database_oid: pg_sys::Oid::from_u32(10),
        index_oid: pg_sys::Oid::from_u32(200),
        segment_id: seg_a,
        bound: MutableSegmentBound {
            max_doc: 10,
            num_deleted_docs: 0,
        },
    };

    slots[0] = CacheSlot {
        key: key_a1,
        span: ArenaSpan {
            offset: 0,
            len: 100,
            order: 0,
        },
        state: SlotState::Ready,
        active_readers: AtomicU32::new(0),
        usage_count: AtomicU32::new(0),
    };
    slots[1] = CacheSlot {
        key: key_a2,
        span: ArenaSpan {
            offset: 100,
            len: 100,
            order: 0,
        },
        state: SlotState::Ready,
        active_readers: AtomicU32::new(0),
        usage_count: AtomicU32::new(0),
    };
    slots[2] = CacheSlot {
        key: key_b,
        span: ArenaSpan {
            offset: 200,
            len: 100,
            order: 0,
        },
        state: SlotState::Ready,
        active_readers: AtomicU32::new(0),
        usage_count: AtomicU32::new(0),
    };
    slots[3] = CacheSlot {
        key: key_other_index,
        span: ArenaSpan {
            offset: 300,
            len: 100,
            order: 0,
        },
        state: SlotState::Ready,
        active_readers: AtomicU32::new(0),
        usage_count: AtomicU32::new(0),
    };

    // Invalidate segment A in index 100
    for slot in &mut slots {
        if slot.state != SlotState::Empty
            && slot.key.database_oid == pg_sys::Oid::from_u32(10)
            && slot.key.index_oid == pg_sys::Oid::from_u32(100)
            && slot.key.segment_id == seg_a
        {
            *slot = CacheSlot::empty();
        }
    }

    assert_eq!(slots[0].state, SlotState::Empty);
    assert_eq!(slots[1].state, SlotState::Empty);
    assert_eq!(slots[2].state, SlotState::Ready);
    assert_eq!(slots[3].state, SlotState::Ready);

    // Invalidate all of index 100
    for slot in &mut slots {
        if slot.state != SlotState::Empty
            && slot.key.database_oid == pg_sys::Oid::from_u32(10)
            && slot.key.index_oid == pg_sys::Oid::from_u32(100)
        {
            *slot = CacheSlot::empty();
        }
    }

    assert_eq!(slots[2].state, SlotState::Empty);
    assert_eq!(slots[3].state, SlotState::Ready);
}

#[test]
fn test_inflight_build_table() {
    let mut inflights = [const { InflightBuild::empty() }; MAX_INFLIGHT_BUILDS];
    assert!(inflights[0].is_empty());

    let key = MutableCacheKey {
        database_oid: pg_sys::Oid::from_u32(1),
        index_oid: pg_sys::Oid::from_u32(2),
        segment_id: [42u8; 16],
        bound: MutableSegmentBound {
            max_doc: 100,
            num_deleted_docs: 0,
        },
    };

    // Claim entry
    inflights[0].key = key;
    inflights[0].builder_pid = 12345;
    assert!(!inflights[0].is_empty());

    // Match entry
    let found = inflights
        .iter()
        .any(|b| !b.is_empty() && b.key == key && b.builder_pid == 12345);
    assert!(found);

    // Clear entry
    inflights[0] = InflightBuild::empty();
    assert!(inflights[0].is_empty());
}

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

#[pgrx::pg_schema]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::thread::sleep;
    use std::time::{Duration, Instant};

    use pgrx::pg_sys;
    use pgrx::prelude::*;
    use tantivy::Directory;
    use tantivy::directory::RamDirectory;
    use tantivy::index::SegmentId;

    use super::super::directory::SharedMemoryDirectory;
    use super::super::pack::{PACKED_MAGIC, pack_segment, unpack_toc};
    use super::super::*;
    use crate::postgres::storage::block::{
        MutableSegmentBound, SegmentMetaEntry, SegmentMetaEntryImmutable, SegmentMetaEntryMutable,
    };

    fn lifecycle_key(id: i32) -> MutableCacheKey {
        let mut segment_id = [0; 16];
        segment_id[..4].copy_from_slice(&id.to_le_bytes());
        MutableCacheKey {
            database_oid: unsafe { pg_sys::MyDatabaseId },
            index_oid: pg_sys::InvalidOid,
            segment_id,
            bound: MutableSegmentBound {
                max_doc: 1,
                num_deleted_docs: 0,
            },
        }
    }

    #[pg_extern]
    fn test_mutable_cache_access(
        id: i32,
        pause_build: bool,
        pause_reader: bool,
        build_gate: default!(i64, 0),
    ) -> bool {
        let key = lifecycle_key(id);
        let path = PathBuf::from(format!(
            "{}.fast",
            SegmentId::from_bytes(key.segment_id).uuid_string()
        ));
        let directory = get_or_build(&key, || {
            if build_gate != 0 {
                Spi::run(&format!("SELECT pg_advisory_xact_lock({build_gate})")).unwrap();
            }
            if pause_build {
                Spi::run("SELECT pg_sleep(60)").unwrap();
            }
            let directory = RamDirectory::default();
            directory.atomic_write(&path, &[1, 2, 3])?;
            Ok(directory)
        })
        .unwrap();
        if pause_reader {
            Spi::run("SELECT pg_sleep(60)").unwrap();
        }
        directory.atomic_read(&path).unwrap() == [1, 2, 3]
    }

    #[pg_extern]
    fn test_mutable_cache_pin_arena(id: i32) {
        let key = lifecycle_key(id);
        let cache = load_cache().unwrap();
        let guard = cache.lock.acquire_exclusive();
        let span = cache
            .allocate(cache.arena_capacity as usize, &guard)
            .unwrap();
        let slots = unsafe { cache.slots_mut(&guard) };
        let idx = slots
            .iter()
            .position(|slot| slot.state() == SlotState::Empty)
            .unwrap();
        slots[idx].key = key;
        slots[idx].span = span;
        slots[idx].set_state(SlotState::Ready);
        slots[idx].active_readers.store(1, Ordering::Release);
        BACKEND_READERS[idx].fetch_add(1, Ordering::Relaxed);
        drop(guard);
        let _reader = ActiveReaderGuard { slot_index: idx };
        cache.invalidate_segment(key.database_oid, key.index_oid, &key.segment_id);
        Spi::run("SELECT pg_sleep(60)").unwrap();
    }

    #[pg_extern]
    fn test_mutable_cache_invalidation_race(id: i32, reader_pid: i32) {
        const WAIT_CLASS_MASK: u32 = 0xFF00_0000;
        const TIMEOUT: Duration = Duration::from_secs(5);
        const POLL_INTERVAL: Duration = Duration::from_millis(1);
        let key = lifecycle_key(id);
        let reader_proc = unsafe { pg_sys::BackendPidGetProc(reader_pid) };
        assert!(!reader_proc.is_null());
        let cache = load_cache().unwrap();
        let guard = cache.lock.acquire_exclusive();
        let slot = unsafe { cache.slots_mut(&guard) }
            .iter_mut()
            .find(|slot| slot.key == key)
            .unwrap();
        assert_eq!(slot.active_readers.load(Ordering::Acquire), 1);
        unsafe { pg_sys::pgstat_report_wait_start(pg_sys::PG_WAIT_EXTENSION) };
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let wait_class = unsafe { std::ptr::read_volatile(&(*reader_proc).wait_event_info) }
                & WAIT_CLASS_MASK;
            if slot.active_readers.load(Ordering::Acquire) == 0
                && matches!(wait_class, pg_sys::PG_WAIT_LWLOCK | pg_sys::PG_WAIT_CLIENT)
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "reader did not release its last pin"
            );
            sleep(POLL_INTERVAL);
        }
        unsafe { pg_sys::pgstat_report_wait_end() };
        slot.set_state(SlotState::Superseded);
    }

    #[pg_test]
    fn test_oversized_allocation_preserves_cached_segments() {
        let cache = load_cache().unwrap();
        let key = lifecycle_key(65255);
        assert!(test_mutable_cache_access(65255, false, false, 0));
        let guard = cache.lock.acquire_exclusive();
        let slot = unsafe { cache.slots() }
            .iter()
            .find(|slot| slot.key == key)
            .unwrap();
        let clock_hand = cache.clock_hand.load(Ordering::Relaxed);
        let usage = slot.usage_count.load(Ordering::Relaxed);
        assert!(
            cache
                .allocate(cache.arena_capacity as usize + 1, &guard)
                .is_none()
        );
        assert_eq!(slot.state(), SlotState::Ready);
        assert_eq!(slot.usage_count.load(Ordering::Relaxed), usage);
        assert_eq!(cache.clock_hand.load(Ordering::Relaxed), clock_hand);
        drop(guard);
        drop(
            get_or_build(&key, || {
                panic!("oversized allocation evicted cached segment")
            })
            .unwrap(),
        );
        cache.invalidate_segment(key.database_oid, key.index_oid, &key.segment_id);
    }

    #[pg_extern]
    fn test_mutable_cache_readers(id: i32, invalidate: bool) -> i64 {
        let key = lifecycle_key(id);
        let cache = load_cache().expect("cache must be preloaded");
        if invalidate {
            cache.invalidate_segment(key.database_oid, key.index_oid, &key.segment_id);
        }
        let _guard = cache.lock.acquire_exclusive();
        unsafe { cache.slots() }
            .iter()
            .find(|slot| slot.state() != SlotState::Empty && slot.key == key)
            .map(|slot| i64::from(slot.active_readers.load(Ordering::Acquire)))
            .unwrap_or(-1)
    }

    #[pg_test]
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

        let packed = pack_segment(&ram_dir, &segment_id, usize::MAX)
            .expect("packing must succeed")
            .unwrap();
        assert!(
            pack_segment(&ram_dir, &segment_id, packed.len() - 1)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            pack_segment(&ram_dir, &segment_id, packed.len())
                .unwrap()
                .unwrap(),
            packed
        );
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
        let reader_guard = Arc::new(ActiveReaderGuard {
            slot_index: MAX_SLOTS,
        });
        let shmem_dir =
            SharedMemoryDirectory::new(packed.as_ptr(), packed.len(), reader_guard, toc);

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

    #[pg_test]
    fn test_shared_cache_hit_and_invalidation() {
        assert!(load_cache().is_some(), "cache must be preloaded");
        let segment_id = SegmentId::generate_random();
        let key = MutableCacheKey {
            database_oid: unsafe { pg_sys::MyDatabaseId },
            index_oid: pg_sys::InvalidOid,
            segment_id: *segment_id.uuid_bytes(),
            bound: MutableSegmentBound {
                max_doc: 1,
                num_deleted_docs: 0,
            },
        };
        let path = PathBuf::from(format!("{}.fast", segment_id.uuid_string()));
        let first = get_or_build(&key, || {
            let directory = RamDirectory::default();
            directory.atomic_write(&path, &[1, 2, 3])?;
            Ok(directory)
        })
        .unwrap();
        let cached = get_or_build(&key, || panic!("cache hit must not rebuild")).unwrap();
        invalidate_segment(key.database_oid, key.index_oid, &key.segment_id);
        assert_eq!(cached.atomic_read(&path).unwrap(), [1, 2, 3]);
        drop(first);
        drop(cached);
        let replacement = get_or_build(&key, || {
            let directory = RamDirectory::default();
            directory.atomic_write(&path, &[4, 5, 6])?;
            Ok(directory)
        })
        .unwrap();
        assert_eq!(replacement.atomic_read(&path).unwrap(), [4, 5, 6]);
        invalidate_segment(key.database_oid, key.index_oid, &key.segment_id);
    }

    #[pg_test]
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

    #[pg_test]
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

    #[pg_test]
    unsafe fn test_clock_sweep_eviction_and_usage_count() {
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
            state: AtomicU32::new(SlotState::Ready as u32),
        };

        // Slot 1: cold (usage_count = 0)
        slots[1] = CacheSlot {
            key: MutableCacheKey::default(),
            span: s1,
            active_readers: AtomicU32::new(0),
            usage_count: AtomicU32::new(0),
            state: AtomicU32::new(SlotState::Ready as u32),
        };

        // Allocate another 512KB slab. Slab pool is full, so evict_with_clock_sweep is triggered!
        let s2 = header
            .evict_with_clock_sweep(500 * 1024, slots, slab_pool)
            .expect("must evict cold slot and allocate");

        // Slot 1 (cold, usage_count = 0) must be evicted!
        assert_eq!(slots[1].state(), SlotState::Empty);
        // Slot 0 (hot) should still be Ready, but its usage_count decremented!
        assert_eq!(slots[0].state(), SlotState::Ready);
        assert_eq!(slots[0].usage_count.load(Ordering::Relaxed), 2);
        // New allocation took Slot 1's freed slab
        assert_eq!(s2.offset, s1.offset);
    }

    #[pg_test]
    unsafe fn test_active_readers_block_clock_sweep() {
        let header = unsafe { test_header(1024 * 1024) }; // 1MB arena
        let slots = unsafe { &mut *header.slots.get() };
        let slab_pool = unsafe { &mut *header.slab_pool.get() };

        let s0 = slab_pool.allocate(1000 * 1024).unwrap();
        slots[0] = CacheSlot {
            key: MutableCacheKey::default(),
            span: s0,
            active_readers: AtomicU32::new(2), // 2 readers reading!
            usage_count: AtomicU32::new(0),
            state: AtomicU32::new(SlotState::Ready as u32),
        };

        // Arena is full, and slot 0 has active readers > 0 -> eviction must fail!
        let blocked = header.evict_with_clock_sweep(500 * 1024, slots, slab_pool);
        assert!(blocked.is_none(), "Must not evict slot with active readers");
        assert_eq!(slots[0].state(), SlotState::Ready);

        // After readers finish:
        slots[0].active_readers.store(0, Ordering::Release);
        let allowed = header.evict_with_clock_sweep(500 * 1024, slots, slab_pool);
        assert!(allowed.is_some(), "Must evict after readers finish");
        assert_eq!(slots[0].state(), SlotState::Empty);
    }

    #[pg_test]
    unsafe fn test_eager_reclamation_of_superseded_slots() {
        let header = unsafe { test_header(1024 * 1024) }; // 1MB arena
        let slots = unsafe { &mut *header.slots.get() };
        let slab_pool = unsafe { &mut *header.slab_pool.get() };

        let s0 = slab_pool.allocate(500 * 1024).unwrap();
        slots[0] = CacheSlot {
            key: MutableCacheKey::default(),
            span: s0,
            active_readers: AtomicU32::new(0),
            usage_count: AtomicU32::new(0),
            state: AtomicU32::new(SlotState::Superseded as u32),
        };

        // Superseded slot with 0 readers is reclaimed on the clock sweep immediately
        let s1 = header
            .evict_with_clock_sweep(500 * 1024, slots, slab_pool)
            .expect("must reclaim superseded slot");
        assert_eq!(slots[0].state(), SlotState::Empty);
        assert_eq!(s1.offset, s0.offset);
    }

    #[pg_test]
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
            state: AtomicU32::new(SlotState::Ready as u32),
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
            state: AtomicU32::new(SlotState::Ready as u32),
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
            state: AtomicU32::new(SlotState::Ready as u32),
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
            state: AtomicU32::new(SlotState::Ready as u32),
            active_readers: AtomicU32::new(0),
            usage_count: AtomicU32::new(0),
        };

        // Invalidate segment A in index 100
        for slot in &mut slots {
            if slot.state() != SlotState::Empty
                && slot.key.database_oid == pg_sys::Oid::from_u32(10)
                && slot.key.index_oid == pg_sys::Oid::from_u32(100)
                && slot.key.segment_id == seg_a
            {
                *slot = CacheSlot::empty();
            }
        }

        assert_eq!(slots[0].state(), SlotState::Empty);
        assert_eq!(slots[1].state(), SlotState::Empty);
        assert_eq!(slots[2].state(), SlotState::Ready);
        assert_eq!(slots[3].state(), SlotState::Ready);

        // Invalidate all of index 100
        for slot in &mut slots {
            if slot.state() != SlotState::Empty
                && slot.key.database_oid == pg_sys::Oid::from_u32(10)
                && slot.key.index_oid == pg_sys::Oid::from_u32(100)
            {
                *slot = CacheSlot::empty();
            }
        }

        assert_eq!(slots[2].state(), SlotState::Empty);
        assert_eq!(slots[3].state(), SlotState::Ready);
    }

    #[pg_test]
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
}

#[cfg(test)]
mod backend {
    use postgres::error::SqlState;
    use postgres::{Client, NoTls};
    use rstest::rstest;
    use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
    use std::sync::{Mutex, Once};
    use std::thread::{sleep, spawn};
    use std::time::{Duration, Instant};

    static SCHEDULE: Mutex<()> = Mutex::new(());

    fn connect() -> Client {
        let mut client = if let Ok(url) = std::env::var("PG_SEARCH_TEST_DATABASE_URL") {
            Client::connect(&url, NoTls).unwrap()
        } else {
            static INITIALIZE: Once = Once::new();
            INITIALIZE.call_once(|| {
                crate::pg_test::setup(vec![]);
                pgrx_tests::run_test(
                    "bootstrap_test_server",
                    None,
                    crate::pg_test::postgresql_conf_options(),
                )
                .unwrap();
            });
            pgrx_tests::client().unwrap().0
        };
        client.batch_execute("SET statement_timeout='10s'").unwrap();
        client
    }

    fn wait_until(description: &str, mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !condition() {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {description}"
            );
            sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn failed_admission_releases_waiters_to_build_concurrently() {
        const PIN_KEY: i32 = 65256;
        const BUILD_KEY: i32 = 65257;
        const BUILDER_GATE: i64 = 652560;
        const WAITER_GATE: i64 = BUILDER_GATE + 1;
        const WAITERS: usize = 3;
        let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
        let mut control = connect();
        let mut pin = connect();
        let pin_pid: i32 = pin
            .query_one("SELECT pg_backend_pid()", &[])
            .unwrap()
            .get(0);
        let pin =
            spawn(move || pin.query("SELECT tests.test_mutable_cache_pin_arena($1)", &[&PIN_KEY]));
        wait_until("arena pinned", || {
            control
                .query_one(
                    "SELECT tests.test_mutable_cache_readers($1, false)",
                    &[&PIN_KEY],
                )
                .unwrap()
                .get::<_, i64>(0)
                == 1
        });
        control
            .query(
                "SELECT pg_advisory_lock($1), pg_advisory_lock($2)",
                &[&BUILDER_GATE, &WAITER_GATE],
            )
            .unwrap();
        let mut builder = connect();
        let builder_pid: i32 = builder
            .query_one("SELECT pg_backend_pid()", &[])
            .unwrap()
            .get(0);
        let builder = spawn(move || {
            builder.query_one(
                "SELECT tests.test_mutable_cache_access($1, false, false, $2)",
                &[&BUILD_KEY, &BUILDER_GATE],
            )
        });
        wait_until("first builder gated", || {
            control.query_one(
            "SELECT EXISTS(SELECT FROM pg_stat_activity WHERE pid=$1 AND wait_event='advisory')", &[&builder_pid],
        ).unwrap().get(0)
        });
        let waiters: Vec<_> = (0..WAITERS)
            .map(|_| {
                let mut waiter = connect();
                let pid: i32 = waiter
                    .query_one("SELECT pg_backend_pid()", &[])
                    .unwrap()
                    .get(0);
                let handle = spawn(move || {
                    waiter.query_one(
                        "SELECT tests.test_mutable_cache_access($1, false, false, $2)",
                        &[&BUILD_KEY, &WAITER_GATE],
                    )
                });
                (pid, handle)
            })
            .collect();
        let waiter_pids: Vec<_> = waiters.iter().map(|(pid, _)| *pid).collect();
        wait_until("all queries waiting for first build", || {
            control.query_one(
            "SELECT count(*) FROM pg_stat_activity WHERE pid=ANY($1) AND wait_event_type='Extension'", &[&waiter_pids],
        ).unwrap().get::<_, i64>(0) == WAITERS as i64
        });
        control
            .query("SELECT pg_advisory_unlock($1)", &[&BUILDER_GATE])
            .unwrap();
        assert!(builder.join().unwrap().unwrap().get::<_, bool>(0));
        let result = catch_unwind(AssertUnwindSafe(|| {
            wait_until("all fallback builders running concurrently", || {
                control.query_one(
                "SELECT count(*) FROM pg_stat_activity WHERE pid=ANY($1) AND wait_event='advisory'", &[&waiter_pids],
            ).unwrap().get::<_, i64>(0) == WAITERS as i64
            });
        }));
        control
            .query("SELECT pg_advisory_unlock($1)", &[&WAITER_GATE])
            .unwrap();
        for (_, waiter) in waiters {
            assert!(waiter.join().unwrap().unwrap().get::<_, bool>(0));
        }
        control
            .query("SELECT pg_cancel_backend($1)", &[&pin_pid])
            .unwrap();
        assert!(pin.join().unwrap().is_err());
        if let Err(error) = result {
            resume_unwind(error);
        }
    }

    #[test]
    fn last_reader_reclaims_concurrently_superseded_slot() {
        const KEY: i32 = 65258;
        let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
        let mut control = connect();
        let mut reader = connect();
        let reader_pid: i32 = reader
            .query_one("SELECT pg_backend_pid()", &[])
            .unwrap()
            .get(0);
        let reader = spawn(move || {
            let result = reader.query_one(
                "SELECT tests.test_mutable_cache_access($1, false, true)",
                &[&KEY],
            );
            (reader, result)
        });
        wait_until("reader pinned", || {
            control.query_one(
            "SELECT EXISTS(SELECT FROM pg_stat_activity WHERE pid=$1 AND wait_event='PgSleep')", &[&reader_pid],
        ).unwrap().get(0)
        });
        let mut invalidator = connect();
        let invalidator_pid: i32 = invalidator
            .query_one("SELECT pg_backend_pid()", &[])
            .unwrap()
            .get(0);
        let invalidator = spawn(move || {
            invalidator.query(
                "SELECT tests.test_mutable_cache_invalidation_race($1, $2)",
                &[&KEY, &reader_pid],
            )
        });
        wait_until("invalidation observed the active reader", || {
            control.query_one(
            "SELECT EXISTS(SELECT FROM pg_stat_activity WHERE pid=$1 AND wait_event_type='Extension')", &[&invalidator_pid],
        ).unwrap().get(0)
        });
        control
            .query("SELECT pg_cancel_backend($1)", &[&reader_pid])
            .unwrap();
        invalidator.join().unwrap().unwrap();
        let (_reader, result) = reader.join().unwrap();
        assert!(result.is_err());
        assert_eq!(
            control
                .query_one(
                    "SELECT tests.test_mutable_cache_readers($1, false)",
                    &[&KEY]
                )
                .unwrap()
                .get::<_, i64>(0),
            -1
        );
    }

    #[rstest]
    #[case::cancel_reader(false, false, 65251)]
    #[case::terminate_reader(true, false, 65252)]
    #[case::cancel_builder(false, true, 65253)]
    #[case::terminate_builder(true, true, 65254)]
    fn backend_exit_releases_cache_resources(
        #[case] terminate: bool,
        #[case] building: bool,
        #[case] id: i32,
    ) {
        let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
        let mut control = connect();
        let mut victim = connect();
        let pid: i32 = victim
            .query_one("SELECT pg_backend_pid()", &[])
            .unwrap()
            .get(0);
        let victim = spawn(move || {
            victim.query_one(
                "SELECT tests.test_mutable_cache_access($1, $2, $3)",
                &[&id, &building, &!building],
            )
        });
        wait_until("cache holder parking", || {
            control
                .query_one(
                    "SELECT EXISTS(SELECT FROM pg_stat_activity WHERE pid=$1 AND wait_event='PgSleep')",
                    &[&pid],
                )
                .unwrap()
                .get(0)
        });
        let (waiter, keeper) = if building {
            let mut waiter = connect();
            let waiter_pid: i32 = waiter
                .query_one("SELECT pg_backend_pid()", &[])
                .unwrap()
                .get(0);
            let handle = spawn(move || {
                waiter.query_one(
                    "SELECT tests.test_mutable_cache_access($1, false, false)",
                    &[&id],
                )
            });
            wait_until("query waiting for cache builder", || {
                control.query_one(
                    "SELECT EXISTS(SELECT FROM pg_stat_activity WHERE pid=$1 AND wait_event='Extension')",
                    &[&waiter_pid],
                ).unwrap().get(0)
            });
            (Some(handle), None)
        } else {
            let mut keeper = connect();
            let keeper_pid: i32 = keeper
                .query_one("SELECT pg_backend_pid()", &[])
                .unwrap()
                .get(0);
            let handle = spawn(move || {
                keeper.query_one(
                    "SELECT tests.test_mutable_cache_access($1, false, true)",
                    &[&id],
                )
            });
            wait_until("second cache reader parking", || {
                control.query_one(
                    "SELECT EXISTS(SELECT FROM pg_stat_activity WHERE pid=$1 AND wait_event='PgSleep')",
                    &[&keeper_pid],
                ).unwrap().get(0)
            });
            let readers: i64 = control
                .query_one("SELECT tests.test_mutable_cache_readers($1, true)", &[&id])
                .unwrap()
                .get(0);
            assert_eq!(readers, 2);
            (None, Some((keeper_pid, handle)))
        };
        let signal = if terminate {
            "pg_terminate_backend"
        } else {
            "pg_cancel_backend"
        };
        assert!(
            control
                .query_one(&format!("SELECT {signal}($1)"), &[&pid])
                .unwrap()
                .get::<_, bool>(0)
        );
        assert!(victim.join().unwrap().is_err());
        if let Some(waiter) = waiter {
            wait_until("waiter recovering after builder exits", || {
                waiter.is_finished()
            });
            assert!(waiter.join().unwrap().unwrap().get::<_, bool>(0));
        }
        if let Some((pid, keeper)) = keeper {
            wait_until("only the surviving reader's pin remaining", || {
                control
                    .query_one("SELECT tests.test_mutable_cache_readers($1, false)", &[&id])
                    .unwrap()
                    .get::<_, i64>(0)
                    == 1
            });
            assert!(
                control
                    .query_one("SELECT pg_cancel_backend($1)", &[&pid])
                    .unwrap()
                    .get::<_, bool>(0)
            );
            assert!(keeper.join().unwrap().is_err());
        }
        wait_until("reader pins reclaimed", || {
            control
                .query_one("SELECT tests.test_mutable_cache_readers($1, true)", &[&id])
                .unwrap()
                .get::<_, i64>(0)
                == -1
        });
        assert_eq!(
            control.query_one("SELECT 1", &[]).unwrap().get::<_, i32>(0),
            1
        );
    }

    #[test]
    fn live_toast_corruption_is_not_cached() {
        let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
        let mut client = connect();
        client
            .batch_execute(
                "CREATE EXTENSION IF NOT EXISTS pg_surgery;
                 SET max_parallel_workers_per_gather=0;
                 DROP TABLE IF EXISTS cache_corrupt_toast;
                 CREATE TABLE cache_corrupt_toast(id bigint PRIMARY KEY, body text)
                     WITH(autovacuum_enabled=false);
                 ALTER TABLE cache_corrupt_toast ALTER COLUMN body SET STORAGE EXTERNAL;
                 CREATE INDEX cache_corrupt_toast_idx ON cache_corrupt_toast USING paradedb(id, body)
                     WITH(mutable_segment_rows=1000, layer_sizes='0', background_layer_sizes='0');
                 INSERT INTO cache_corrupt_toast VALUES
                     (1, 'marker'), (2, 'marker ' || repeat(md5('2') || ' ', 200)), (3, 'marker');
                 DO $$
                 DECLARE toast_relation regclass;
                 BEGIN
                     SELECT reltoastrelid INTO toast_relation FROM pg_class
                     WHERE oid='cache_corrupt_toast'::regclass;
                     EXECUTE format('SELECT heap_force_kill(%L::regclass, array_agg(ctid)) FROM %s',
                                    toast_relation, toast_relation);
                 END $$;",
            )
            .unwrap();
        let error = client
            .query(
                "SELECT length(body) FROM cache_corrupt_toast WHERE id=2",
                &[],
            )
            .unwrap_err();
        assert_eq!(error.code(), Some(&SqlState::DATA_CORRUPTED));

        let query = "SELECT id FROM cache_corrupt_toast WHERE body ||| 'marker' ORDER BY id";
        for enabled in [false, true] {
            client
                .batch_execute(&format!(
                    "SET paradedb.enable_mutable_segment_cache={enabled}"
                ))
                .unwrap();
            for _ in 0..2 {
                let error = client.query(query, &[]).unwrap_err();
                assert_eq!(error.code(), Some(&SqlState::DATA_CORRUPTED));
            }
        }

        let mut snapshot = connect();
        snapshot
            .batch_execute(
                "BEGIN ISOLATION LEVEL REPEATABLE READ;
                 SELECT id FROM cache_corrupt_toast;",
            )
            .unwrap();
        client
            .execute("DELETE FROM cache_corrupt_toast WHERE id=2", &[])
            .unwrap();
        for _ in 0..2 {
            let error = client.query(query, &[]).unwrap_err();
            assert_eq!(error.code(), Some(&SqlState::DATA_CORRUPTED));
        }
        snapshot.batch_execute("COMMIT").unwrap();

        client
            .batch_execute(
                "TRUNCATE cache_corrupt_toast;
                 INSERT INTO cache_corrupt_toast VALUES (1, 'marker'), (3, 'marker');",
            )
            .unwrap();
        for _ in 0..2 {
            let ids: Vec<i64> = client
                .query(query, &[])
                .unwrap()
                .iter()
                .map(|row| row.get(0))
                .collect();
            assert_eq!(ids, [1, 3]);
        }
        client
            .batch_execute("DROP TABLE cache_corrupt_toast")
            .unwrap();
    }

    #[test]
    fn recently_dead_expression_uses_detoasted_values() {
        let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
        let mut client = connect();
        client
            .batch_execute(
                "CREATE EXTENSION IF NOT EXISTS pg_surgery;
             DROP TABLE IF EXISTS cache_expression_toast;
             CREATE TABLE cache_expression_toast(id bigint PRIMARY KEY, body text)
                 WITH(autovacuum_enabled=false);
             ALTER TABLE cache_expression_toast ALTER COLUMN body SET STORAGE EXTERNAL;
             CREATE INDEX cache_expression_toast_idx ON cache_expression_toast
                 USING paradedb(id, (tests.test_toast_expression(body)::pdb.simple))
                 WITH(mutable_segment_rows=1000, layer_sizes='0', background_layer_sizes='0');
             INSERT INTO cache_expression_toast VALUES (1, repeat(md5('toast') || ' ', 200));
             CREATE TEMP TABLE cache_expression_toast_chunks(toast_relation regclass, tids tid[]);
             DO $$
             DECLARE toast_relation regclass;
             BEGIN
                 SELECT reltoastrelid INTO toast_relation FROM pg_class
                 WHERE oid='cache_expression_toast'::regclass;
                 EXECUTE format('INSERT INTO cache_expression_toast_chunks SELECT %L::regclass, array_agg(ctid) FROM %s',
                                toast_relation, toast_relation);
             END $$;",
            )
            .unwrap();
        let ctid: String = client
            .query_one("SELECT ctid::text FROM cache_expression_toast", &[])
            .unwrap()
            .get(0);
        let mut snapshot = connect();
        snapshot
            .batch_execute(
                "BEGIN ISOLATION LEVEL REPEATABLE READ; SELECT * FROM cache_expression_toast;",
            )
            .unwrap();
        client
            .batch_execute("DELETE FROM cache_expression_toast")
            .unwrap();
        assert!(client.query_one(
            "SELECT tests.test_materialize_toast_expression('cache_expression_toast_idx'::regclass, $1::text::tid)",
            &[&ctid],
        ).unwrap().get::<_, bool>(0));
        let error = snapshot
            .query("SELECT md5(body) FROM cache_expression_toast", &[])
            .unwrap_err();
        assert_eq!(error.code(), Some(&SqlState::DATA_CORRUPTED));
        snapshot.batch_execute("ROLLBACK").unwrap();
        client
            .batch_execute("DROP TABLE cache_expression_toast")
            .unwrap();
    }

    #[test]
    fn maintenance_merge_rechecks_heap_visibility() {
        let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
        let mut client = connect();
        let interval: i32 = client
            .query_one(
                "SELECT setting::int FROM pg_settings WHERE name='paradedb.mutable_sweep_interval'",
                &[],
            )
            .unwrap()
            .get(0);
        let result = catch_unwind(AssertUnwindSafe(|| {
            if interval != 0 {
                client
                    .batch_execute("ALTER SYSTEM SET paradedb.mutable_sweep_interval=0")
                    .unwrap();
                client.simple_query("SELECT pg_reload_conf()").unwrap();
            }
            wait_until("periodic flushing paused", || {
                client
                    .query_one(
                        "SELECT current_setting('paradedb.mutable_sweep_interval')::int=0
                     AND NOT EXISTS(SELECT FROM pg_stat_activity
                         WHERE datname=current_database() AND backend_type='pg_search mutable sweep')",
                        &[],
                    )
                    .unwrap()
                    .get(0)
            });
            client
                .batch_execute(
                    "SET max_parallel_workers_per_gather=0;
                 SET paradedb.enable_aggregate_custom_scan=off;",
                )
                .unwrap();
            let mut sizes = Vec::new();
            for enabled in [false, true] {
                let table = format!("cache_dead_merge_{enabled}");
                client.batch_execute(&format!(
                "SET paradedb.enable_mutable_segment_cache={enabled};
                 DROP TABLE IF EXISTS {table};
                 CREATE TABLE {table}(id bigint PRIMARY KEY, body text) WITH(autovacuum_enabled=false);
                 CREATE INDEX {table}_idx ON {table} USING paradedb(id, body)
                     WITH(mutable_segment_rows=200, target_segment_count=1, layer_sizes='0', background_layer_sizes='0');"
            )).unwrap();
                client.execute(&format!(
                "INSERT INTO {table}
                 SELECT i, 'marker ' || (SELECT string_agg(md5(g::text), ' ') FROM generate_series(1,500) g)
                 FROM generate_series(1,200) i"
            ), &[]).unwrap();
                let query = format!("SELECT count(*) FROM {table} WHERE body ||| 'marker'");
                let plan = client
                    .query(&format!("EXPLAIN (COSTS OFF) {query}"), &[])
                    .unwrap();
                assert!(
                    plan.iter()
                        .any(|row| row.get::<_, String>(0).contains("Custom Scan"))
                );
                assert_eq!(client.query_one(&query, &[]).unwrap().get::<_, i64>(0), 200);
                client
                    .execute(&format!("DELETE FROM {table}"), &[])
                    .unwrap();
                client
                    .batch_execute(&format!(
                        "ALTER INDEX {table}_idx SET(mutable_segment_rows=0)"
                    ))
                    .unwrap();
                assert_eq!(
                    client
                        .query_one(
                            &format!("SELECT paradedb.test_merge_index('{table}_idx')"),
                            &[]
                        )
                        .unwrap()
                        .get::<_, i64>(0),
                    1
                );
                sizes.push(client.query_one(&format!(
                "SELECT coalesce(sum(byte_size),0)::bigint FROM paradedb.index_info('{table}_idx') WHERE visible"
            ), &[]).unwrap().get::<_, i64>(0));
                client
                    .batch_execute(&format!("DROP TABLE {table}"))
                    .unwrap();
            }
            assert!(
                sizes[1] <= sizes[0] + 8192,
                "cached merge retained dead contents: {sizes:?}"
            );
        }));
        if interval != 0 {
            client
                .batch_execute(&format!(
                    "ALTER SYSTEM SET paradedb.mutable_sweep_interval={interval}"
                ))
                .unwrap();
            client.simple_query("SELECT pg_reload_conf()").unwrap();
        }
        if let Err(error) = result {
            resume_unwind(error);
        }
    }
}

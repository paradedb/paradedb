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

//! Streaming intersection cursors over a PostgreSQL TIDBitmap.
//!
//! The index's doc streams are ctid-ascending (the planner gates harvesting on it),
//! so a heap-filter scorer intersects by merging: one forward-only cursor per
//! `(consumer, segment)` stream over the bitmap's page iteration, no materialized
//! representation and no random access.
//!
//! Serial scans iterate the leader-local TIDBitmap privately (multiple concurrent
//! private iterators each hold their own position). Parallel scans iterate shared
//! state: the build owner calls `tbm_prepare_shared_iterate` for every stream and
//! publishes a claim table in a DSA area; whichever process ends up owning a
//! segment claims its entry and attaches. A stream has one live cursor at a time:
//! a claim while the previous cursor is still open means a collector broke the
//! one-scorer-per-stream invariant, and errors. Dropping the cursor releases the
//! stream, because one execution can legitimately build a scorer for the same
//! segment more than once: TopK queries again with a larger chunk when the first
//! batch loses rows to visibility, and a window aggregate runs its own search
//! after the TopK one. Every pass has to probe the same bitmap, since TopK carries
//! its offset from one query to the next. A private stream rewinds with a fresh
//! iterator. A shared iteration state cannot rewind, and only the bitmap's
//! creating backend can mint one, so each participant of a parallel scan gets one
//! state, minted by the owner at DSM initialization, and drains it once into a
//! process-local page cache. The bitmap is the same for every stream, so every
//! cursor of that participant is a position into the cache, and a second pass
//! replays the pages the first one pulled before pulling more.
//!
//! The claim table exists even though the bitmap itself is immutable because
//! segments are checked out dynamically: the owner cannot know which process will
//! consume which stream, so the eventual consumer self-serves from the table.
//! Core's shared iteration state assumes parallel work-stealing consumption (a
//! spinlocked position in DSA); here each state has exactly one consumer, and
//! each stream exactly one live cursor, which the claim flag enforces.

use crate::query::heap_field_filter::TidProbe;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};

/// Everything a non-owner needs to attach a published shared bitmap: the
/// owner's DSA area and the claim table within it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SharedBitmapHandle {
    pub(crate) area: pg_sys::dsa_handle,
    pub(crate) table: pg_sys::dsa_pointer,
}
use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tantivy::index::SegmentId;

/// Safe over-approximation of MaxHeapTuplesPerPage (which divides BLCKSZ by >= 28
/// per tuple) for any BLCKSZ; the macro is not in the bindings.
const OFFSETS_CAP: usize = (pg_sys::BLCKSZ as usize) / 16;

/// Late-bound source slot: installed on covered HeapFilters at attach time and
/// filled (or swapped, when a serial-context build is upgraded to shared at DSM
/// initialization) once the bitmap and claim table exist. Cloneable and
/// equality-neutral so it can ride inside `SearchQueryInput`.
#[derive(Clone, Default)]
pub struct BitmapCell(
    std::sync::Arc<std::sync::RwLock<Option<std::sync::Arc<BitmapCursorSource>>>>,
);

impl BitmapCell {
    pub(crate) fn fill(&self, source: std::sync::Arc<BitmapCursorSource>) {
        *self.0.write().unwrap() = Some(source);
    }

    pub(crate) fn get(&self) -> Option<std::sync::Arc<BitmapCursorSource>> {
        self.0.read().unwrap().clone()
    }
}

impl PartialEq for BitmapCell {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl std::fmt::Debug for BitmapCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BitmapCell")
    }
}

impl std::fmt::Debug for BitmapCursorSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Private { .. } => f.write_str("BitmapCursorSource::Private"),
            Self::Shared { .. } => f.write_str("BitmapCursorSource::Shared"),
        }
    }
}

/// Create the scan's own DSA area. pg17+ dropped the `dsa_create` function for a
/// macro over `dsa_create_ext`, so the sizes are spelled out there (dsa.h's
/// defaults: 1MB initial segment, `1 << DSA_OFFSET_WIDTH` max).
///
/// Uses the built-in parallel-query-DSA lock tranche: `LWLockNewTrancheId`
/// hands out from a non-recyclable cluster-wide pool of ~64K ids, so a
/// per-query allocation would exhaust it. The tranche only names the area's
/// locks in monitoring views.
pub(crate) unsafe fn create_area() -> *mut pg_sys::dsa_area {
    unsafe {
        let tranche = pg_sys::BuiltinTrancheIds::LWTRANCHE_PARALLEL_QUERY_DSA as i32;
        #[cfg(not(any(feature = "pg17", feature = "pg18")))]
        {
            pg_sys::dsa_create(tranche)
        }
        #[cfg(any(feature = "pg17", feature = "pg18"))]
        {
            pg_sys::dsa_create_ext(tranche, 1024 * 1024, 1 << 40)
        }
    }
}

/// Cross-process cursor counters for EXPLAIN ANALYZE, accumulated by every cursor
/// of the scan. Lives either process-local (serial) or inside the DSA table header
/// (parallel), always addressed through raw atomic pointers with the source's
/// lifetime.
#[repr(C)]
#[derive(Debug, Default)]
pub(crate) struct CursorCounters {
    pub(crate) exact_pages: AtomicU64,
    pub(crate) lossy_pages: AtomicU64,
    pub(crate) recheck_pages: AtomicU64,
    pub(crate) rejected_docs: AtomicU64,
}

/// One `(consumer, segment)` stream in the shared claim table.
#[repr(C)]
struct SharedEntry {
    consumer_id: u32,
    /// Nonzero while a cursor is open on this stream.
    live: AtomicU32,
    segment_id: [u8; 16],
}

// The slot states follow the entries directly, so the entries must keep them
// aligned for `dsa_pointer`.
const _: () = assert!(
    std::mem::size_of::<SharedEntry>().is_multiple_of(std::mem::align_of::<pg_sys::dsa_pointer>())
);

/// Header of the claim table allocation: `nentries` `SharedEntry`s follow it, then
/// `slots` `dsa_pointer`s, one shared iteration state per participant slot.
#[repr(C)]
struct SharedHeader {
    nentries: u64,
    slots: u32,
    counters: CursorCounters,
}

impl SharedHeader {
    unsafe fn entries(this: *mut Self) -> *mut SharedEntry {
        unsafe { this.add(1).cast::<SharedEntry>() }
    }

    unsafe fn slot_states(this: *mut Self) -> *mut pg_sys::dsa_pointer {
        unsafe {
            Self::entries(this)
                .add((*this).nentries as usize)
                .cast::<pg_sys::dsa_pointer>()
        }
    }
}

/// One page of a shared bitmap, as a participant pulled it from its slot.
pub(crate) struct CachedPage {
    block: u32,
    lossy: bool,
    recheck: bool,
    offsets: Vec<pg_sys::OffsetNumber>,
}

/// A participant's view of its slot: the iterator it attached, the pages pulled
/// so far in iteration order, and whether the slot has run out.
pub(crate) struct SharedPages {
    iter: *mut pg_sys::TBMSharedIterator,
    pages: Vec<CachedPage>,
    drained: bool,
}

impl SharedPages {
    /// Pull the next page of the slot into the cache; `false` once the slot is drained.
    unsafe fn pull(
        &mut self,
        area: *mut pg_sys::dsa_area,
        table: pg_sys::dsa_pointer,
        slot: u32,
    ) -> bool {
        if self.drained {
            return false;
        }
        unsafe {
            if self.iter.is_null() {
                let header = pg_sys::dsa_get_address(area, table).cast::<SharedHeader>();
                let state = *SharedHeader::slot_states(header).add(slot as usize);
                self.iter = pg_sys::tbm_attach_shared_iterate(area, state);
            }
            match self.iterate() {
                Some(page) => {
                    self.pages.push(page);
                    true
                }
                None => {
                    self.drained = true;
                    false
                }
            }
        }
    }

    #[cfg(feature = "pg18")]
    unsafe fn iterate(&mut self) -> Option<CachedPage> {
        unsafe {
            let mut res = pg_sys::TBMIterateResult::default();
            if !pg_sys::tbm_shared_iterate(self.iter, &mut res) {
                return None;
            }
            let offsets = if res.lossy {
                Vec::new()
            } else {
                let mut offsets = vec![0; OFFSETS_CAP];
                let n = pg_sys::tbm_extract_page_tuple(
                    &mut res,
                    offsets.as_mut_ptr(),
                    OFFSETS_CAP as u32,
                ) as usize;
                offsets.truncate(n);
                offsets
            };
            Some(CachedPage {
                block: res.blockno,
                lossy: res.lossy,
                recheck: res.recheck,
                offsets,
            })
        }
    }

    #[cfg(not(feature = "pg18"))]
    unsafe fn iterate(&mut self) -> Option<CachedPage> {
        unsafe {
            let res = pg_sys::tbm_shared_iterate(self.iter);
            if res.is_null() {
                return None;
            }
            let res = &*res;
            let lossy = res.ntuples < 0;
            let offsets = if lossy {
                Vec::new()
            } else {
                let n = (res.ntuples as usize).min(OFFSETS_CAP);
                // The result struct is reused by the next iterate call; copy out.
                std::slice::from_raw_parts(res.offsets.as_ptr(), n).to_vec()
            };
            Some(CachedPage {
                block: res.blockno,
                lossy,
                recheck: res.recheck,
                offsets,
            })
        }
    }
}

/// Where cursors come from. Owned by the scan (behind an `Arc`) and outlives every
/// cursor claimed from it.
pub(crate) enum BitmapCursorSource {
    /// Serial: private iterators over the build owner's local TIDBitmap.
    Private {
        tbm: *mut pg_sys::TIDBitmap,
        claims: Mutex<Vec<(u32, SegmentId)>>,
        counters: Box<CursorCounters>,
    },
    /// Parallel: this participant's slot of the published table in the DSA area,
    /// and the pages it has pulled from it.
    Shared {
        area: *mut pg_sys::dsa_area,
        table: pg_sys::dsa_pointer,
        slot: u32,
        pages: RefCell<SharedPages>,
    },
}

// SAFETY: PostgreSQL doesn't execute within threads despite Tantivy expecting it.
unsafe impl Send for BitmapCursorSource {}
unsafe impl Sync for BitmapCursorSource {}

impl BitmapCursorSource {
    pub(crate) fn private(tbm: *mut pg_sys::TIDBitmap) -> Self {
        Self::Private {
            tbm,
            claims: Mutex::new(Vec::new()),
            counters: Box::default(),
        }
    }

    /// Attach a published claim table as participant `slot` (the owner is slot 0,
    /// a parallel worker is its worker number plus one).
    pub(crate) fn shared(
        area: *mut pg_sys::dsa_area,
        table: pg_sys::dsa_pointer,
        slot: u32,
    ) -> Self {
        Self::Shared {
            area,
            table,
            slot,
            pages: RefCell::new(SharedPages {
                iter: std::ptr::null_mut(),
                pages: Vec::new(),
                drained: false,
            }),
        }
    }

    /// Claim the `(consumer, segment)` stream and open its cursor.
    ///
    /// One scorer at a time may consume each stream. A claim while the previous
    /// cursor is still open, or of a stream absent from the table, means a
    /// collector broke the one-scorer-per-stream invariant, and raises an
    /// execution error rather than silently degrading. A claim after the previous
    /// cursor dropped is a new pass over the same bitmap: a private stream opens a
    /// fresh iterator, a shared stream starts over on this participant's page cache.
    pub(crate) unsafe fn claim(
        self: &Arc<Self>,
        consumer_id: u32,
        segment: SegmentId,
    ) -> BitmapCursor {
        match self.as_ref() {
            Self::Private {
                tbm,
                claims,
                counters,
            } => {
                let mut claims = claims.lock().unwrap();
                if claims.contains(&(consumer_id, segment)) {
                    pgrx::error!(
                        "bitmap intersection stream (consumer {consumer_id}, segment {}) claimed twice",
                        segment.uuid_string()
                    );
                }
                claims.push((consumer_id, segment));
                let cursor = unsafe {
                    BitmapCursor::private(*tbm, counters.as_ref() as *const CursorCounters)
                };
                cursor.claimed_from(self, StreamClaim::Private(consumer_id, segment))
            }
            Self::Shared { area, table, .. } => unsafe {
                let header = pg_sys::dsa_get_address(*area, *table).cast::<SharedHeader>();
                let entry = shared_entry(header, consumer_id, segment).unwrap_or_else(|| {
                    pgrx::error!(
                        "bitmap intersection stream (consumer {consumer_id}, segment {}) missing from the shared table",
                        segment.uuid_string()
                    )
                });
                if (*entry)
                    .live
                    .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
                {
                    pgrx::error!(
                        "bitmap intersection stream (consumer {consumer_id}, segment {}) claimed twice",
                        segment.uuid_string()
                    );
                }
                let cursor = BitmapCursor::cached(&(*header).counters as *const CursorCounters);
                cursor.claimed_from(self, StreamClaim::Shared(entry))
            },
        }
    }

    /// Release the stream a dropped cursor held, so the next pass can claim it.
    fn release(&self, claim: StreamClaim) {
        match (self, claim) {
            (Self::Private { claims, .. }, StreamClaim::Private(consumer_id, segment)) => {
                let mut claims = claims.lock().unwrap();
                claims.retain(|claim| *claim != (consumer_id, segment));
            }
            (Self::Shared { .. }, StreamClaim::Shared(entry)) => unsafe {
                (*entry).live.store(0, Ordering::Release);
            },
            _ => unreachable!("a cursor releases the source it was claimed from"),
        }
    }

    /// Counter totals for EXPLAIN ANALYZE.
    pub(crate) fn counters(&self) -> (u64, u64, u64, u64) {
        let c: &CursorCounters = match self {
            Self::Private { counters, .. } => counters.as_ref(),
            Self::Shared { area, table, .. } => unsafe {
                let header = pg_sys::dsa_get_address(*area, *table).cast::<SharedHeader>();
                &(*header).counters
            },
        };
        (
            c.exact_pages.load(Ordering::Relaxed),
            c.lossy_pages.load(Ordering::Relaxed),
            c.recheck_pages.load(Ordering::Relaxed),
            c.rejected_docs.load(Ordering::Relaxed),
        )
    }
}

/// Build owner only: publish the claim table into `area` with one entry per
/// `(consumer, segment)` stream and one shared iteration state per participant
/// slot. The TIDBitmap must have been created over the same `area`.
pub(crate) unsafe fn publish_shared_table(
    tbm: *mut pg_sys::TIDBitmap,
    area: *mut pg_sys::dsa_area,
    consumers: u32,
    segments: &[SegmentId],
    slots: u32,
) -> pg_sys::dsa_pointer {
    assert!(slots > 0, "a shared bitmap needs a slot for its owner");
    unsafe {
        let nentries = consumers as usize * segments.len();
        let size = std::mem::size_of::<SharedHeader>()
            + nentries * std::mem::size_of::<SharedEntry>()
            + slots as usize * std::mem::size_of::<pg_sys::dsa_pointer>();
        let table = pg_sys::dsa_allocate_extended(area, size, pg_sys::DSA_ALLOC_ZERO as _);
        let header = pg_sys::dsa_get_address(area, table).cast::<SharedHeader>();
        (*header).nentries = nentries as u64;
        (*header).slots = slots;
        let entries = SharedHeader::entries(header);
        let mut i = 0;
        for consumer_id in 0..consumers {
            for segment in segments {
                let entry = &mut *entries.add(i);
                entry.consumer_id = consumer_id;
                entry.segment_id = *segment.uuid_bytes();
                i += 1;
            }
        }
        let states = SharedHeader::slot_states(header);
        for slot in 0..slots as usize {
            *states.add(slot) = pg_sys::tbm_prepare_shared_iterate(tbm);
        }
        table
    }
}

/// The claim table entry for `(consumer, segment)`.
unsafe fn shared_entry(
    header: *mut SharedHeader,
    consumer_id: u32,
    segment: SegmentId,
) -> Option<*const SharedEntry> {
    unsafe {
        let entries = SharedHeader::entries(header);
        (0..(*header).nentries as usize)
            .map(|i| entries.add(i) as *const SharedEntry)
            .find(|entry| {
                (**entry).consumer_id == consumer_id
                    && (**entry).segment_id == *segment.uuid_bytes()
            })
    }
}

/// Build owner only, after all consumers have stopped: free every slot's
/// iteration state and the claim table itself.
pub(crate) unsafe fn free_shared_table(area: *mut pg_sys::dsa_area, table: pg_sys::dsa_pointer) {
    unsafe {
        let header = pg_sys::dsa_get_address(area, table).cast::<SharedHeader>();
        let states = SharedHeader::slot_states(header);
        for slot in 0..(*header).slots as usize {
            pg_sys::tbm_free_shared_area(area, *states.add(slot));
        }
        pg_sys::dsa_free(area, table);
    }
}

/// Where a cursor's pages come from.
enum CursorIter {
    /// A private iterator the cursor owns.
    #[cfg(not(feature = "pg18"))]
    Private(*mut pg_sys::TBMIterator),
    #[cfg(feature = "pg18")]
    Private(*mut pg_sys::TBMPrivateIterator),
    /// The next page to read from the source's page cache.
    Cached { next: usize },
}

/// What a cursor holds on its source, released when the cursor drops.
enum StreamClaim {
    Private(u32, SegmentId),
    Shared(*const SharedEntry),
}

/// The current page's decoded state.
enum PageState {
    NotStarted,
    Exhausted,
    Page {
        block: u32,
        lossy: bool,
        recheck: bool,
        noffsets: usize,
        pos: usize,
    },
}

/// A forward-only merge cursor over one bitmap iteration stream.
pub(crate) struct BitmapCursor {
    iter: CursorIter,
    state: PageState,
    offsets: [pg_sys::OffsetNumber; OFFSETS_CAP],
    counters: *const CursorCounters,
    /// The source and the stream this cursor holds on it, released on drop.
    claim: Option<(Arc<BitmapCursorSource>, StreamClaim)>,
    #[cfg(debug_assertions)]
    last_ctid: u64,
}

// SAFETY: PostgreSQL doesn't execute within threads despite Tantivy expecting it.
unsafe impl Send for BitmapCursor {}
unsafe impl Sync for BitmapCursor {}

impl BitmapCursor {
    unsafe fn private(tbm: *mut pg_sys::TIDBitmap, counters: *const CursorCounters) -> Self {
        unsafe {
            #[cfg(not(feature = "pg18"))]
            let iter = CursorIter::Private(pg_sys::tbm_begin_iterate(tbm));
            #[cfg(feature = "pg18")]
            let iter = CursorIter::Private(pg_sys::tbm_begin_private_iterate(tbm));
            Self::new(iter, counters)
        }
    }

    fn cached(counters: *const CursorCounters) -> Self {
        Self::new(CursorIter::Cached { next: 0 }, counters)
    }

    fn new(iter: CursorIter, counters: *const CursorCounters) -> Self {
        Self {
            iter,
            state: PageState::NotStarted,
            offsets: [0; OFFSETS_CAP],
            counters,
            claim: None,
            #[cfg(debug_assertions)]
            last_ctid: 0,
        }
    }

    /// Tie the cursor to the stream it claimed, so dropping it releases the stream.
    fn claimed_from(mut self, source: &Arc<BitmapCursorSource>, claim: StreamClaim) -> Self {
        self.claim = Some((Arc::clone(source), claim));
        self
    }

    /// Probe one ctid. Ctids must arrive in nondecreasing order (the ctid-sorted
    /// planner gate guarantees it per stream); duplicates are fine.
    pub(crate) unsafe fn probe(&mut self, ctid: u64) -> TidProbe {
        #[cfg(debug_assertions)]
        {
            debug_assert!(
                ctid >= self.last_ctid,
                "bitmap cursor probed backwards: {ctid} after {}",
                self.last_ctid
            );
            self.last_ctid = ctid;
        }
        let block = (ctid >> 16) as u32;
        let offset = (ctid & 0xffff) as pg_sys::OffsetNumber;
        loop {
            match &mut self.state {
                PageState::NotStarted => unsafe { self.next_page() },
                PageState::Exhausted => {
                    self.count_rejected();
                    return TidProbe::Reject;
                }
                PageState::Page { block: b, .. } if *b < block => unsafe { self.next_page() },
                PageState::Page { block: b, .. } if *b > block => {
                    self.count_rejected();
                    return TidProbe::Reject;
                }
                PageState::Page { lossy: true, .. } => return TidProbe::NeedsRecheck,
                PageState::Page {
                    recheck,
                    noffsets,
                    pos,
                    ..
                } => {
                    while *pos < *noffsets && self.offsets[*pos] < offset {
                        *pos += 1;
                    }
                    if *pos >= *noffsets || self.offsets[*pos] != offset {
                        self.count_rejected();
                        return TidProbe::Reject;
                    }
                    return if *recheck {
                        TidProbe::NeedsRecheck
                    } else {
                        TidProbe::Candidate
                    };
                }
            }
        }
    }

    fn count_rejected(&self) {
        let c = unsafe { &*self.counters };
        c.rejected_docs.fetch_add(1, Ordering::Relaxed);
    }

    unsafe fn count_page(&self, lossy: bool, recheck: bool) {
        let c = unsafe { &*self.counters };
        if lossy {
            c.lossy_pages.fetch_add(1, Ordering::Relaxed);
        } else {
            c.exact_pages.fetch_add(1, Ordering::Relaxed);
            if recheck {
                c.recheck_pages.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    unsafe fn next_page(&mut self) {
        match self.iter {
            CursorIter::Private(_) => unsafe { self.next_private_page() },
            CursorIter::Cached { next } => unsafe { self.next_cached_page(next) },
        }
    }

    /// Read page `next` of the source's cache, pulling it from the slot first if
    /// the cache ends there.
    unsafe fn next_cached_page(&mut self, next: usize) {
        let source = Arc::clone(&self.claim.as_ref().expect("a cached cursor has a source").0);
        let BitmapCursorSource::Shared {
            area,
            table,
            slot,
            pages,
        } = source.as_ref()
        else {
            unreachable!("a cached cursor comes from a shared source");
        };
        let mut pages = pages.borrow_mut();
        if next == pages.pages.len() && !unsafe { pages.pull(*area, *table, *slot) } {
            self.state = PageState::Exhausted;
            return;
        }
        let page = &pages.pages[next];
        let noffsets = page.offsets.len();
        self.offsets[..noffsets].copy_from_slice(&page.offsets);
        unsafe { self.count_page(page.lossy, page.recheck) };
        self.state = PageState::Page {
            block: page.block,
            lossy: page.lossy,
            recheck: page.recheck,
            noffsets,
            pos: 0,
        };
        self.iter = CursorIter::Cached { next: next + 1 };
    }

    #[cfg(feature = "pg18")]
    unsafe fn next_private_page(&mut self) {
        unsafe {
            let mut res = pg_sys::TBMIterateResult::default();
            let more = match &mut self.iter {
                CursorIter::Private(iter) => pg_sys::tbm_private_iterate(*iter, &mut res),
                CursorIter::Cached { .. } => unreachable!(),
            };
            if !more {
                self.state = PageState::Exhausted;
                return;
            }
            let noffsets = if res.lossy {
                0
            } else {
                pg_sys::tbm_extract_page_tuple(
                    &mut res,
                    self.offsets.as_mut_ptr(),
                    OFFSETS_CAP as u32,
                ) as usize
            };
            self.count_page(res.lossy, res.recheck);
            self.state = PageState::Page {
                block: res.blockno,
                lossy: res.lossy,
                recheck: res.recheck,
                noffsets,
                pos: 0,
            };
        }
    }

    #[cfg(not(feature = "pg18"))]
    unsafe fn next_private_page(&mut self) {
        let res = match &mut self.iter {
            CursorIter::Private(iter) => unsafe { pg_sys::tbm_iterate(*iter) },
            CursorIter::Cached { .. } => unreachable!(),
        };
        if res.is_null() {
            self.state = PageState::Exhausted;
            return;
        }
        let res = unsafe { &*res };
        let lossy = res.ntuples < 0;
        let noffsets = if lossy {
            0
        } else {
            let n = (res.ntuples as usize).min(OFFSETS_CAP);
            // The result struct is reused by the next iterate call; copy out.
            unsafe {
                std::ptr::copy_nonoverlapping(res.offsets.as_ptr(), self.offsets.as_mut_ptr(), n);
            }
            n
        };
        unsafe { self.count_page(lossy, res.recheck) };
        self.state = PageState::Page {
            block: res.blockno,
            lossy,
            recheck: res.recheck,
            noffsets,
            pos: 0,
        };
    }
}

crate::impl_safe_drop!(BitmapCursor, |self| {
    unsafe {
        match self.iter {
            #[cfg(not(feature = "pg18"))]
            CursorIter::Private(iter) => pg_sys::tbm_end_iterate(iter),
            #[cfg(feature = "pg18")]
            CursorIter::Private(iter) => pg_sys::tbm_end_private_iterate(iter),
            CursorIter::Cached { .. } => {}
        }
        if let Some((source, claim)) = self.claim.take() {
            source.release(claim);
        }
    }
});

// The attached slot iterator is this process's own allocation, ended once the
// last cursor is gone with the source.
crate::impl_safe_drop!(BitmapCursorSource, |self| {
    if let Self::Shared { pages, .. } = self {
        let iter = pages.get_mut().iter;
        if !iter.is_null() {
            unsafe { pg_sys::tbm_end_shared_iterate(iter) };
        }
    }
});

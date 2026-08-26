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
//! state: the build owner calls `tbm_prepare_shared_iterate` once per stream and
//! publishes a claim table in a DSA area; whichever process ends up owning a
//! segment claims its entry and attaches. Every entry is take-once: a second claim
//! means a collector broke the one-scorer-per-stream invariant, and errors.

use crate::query::heap_field_filter::TidProbe;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};

/// Everything a non-owner needs to attach a published shared bitmap: the
/// owner's DSA area and the claim table within it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SharedBitmapHandle {
    pub area: pg_sys::dsa_handle,
    pub table: pg_sys::dsa_pointer,
}
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
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
    pub fn fill(&self, source: std::sync::Arc<BitmapCursorSource>) {
        *self.0.write().unwrap() = Some(source);
    }

    pub fn get(&self) -> Option<std::sync::Arc<BitmapCursorSource>> {
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
pub unsafe fn create_area() -> *mut pg_sys::dsa_area {
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
pub struct CursorCounters {
    pub exact_pages: AtomicU64,
    pub lossy_pages: AtomicU64,
    pub recheck_pages: AtomicU64,
    pub rejected_docs: AtomicU64,
}

/// One `(consumer, segment)` slot in the shared claim table.
#[repr(C)]
struct SharedEntry {
    consumer_id: u32,
    claimed: AtomicU32,
    segment_id: [u8; 16],
    iterator: pg_sys::dsa_pointer,
}

/// Header of the claim table allocation; `SharedEntry`s follow contiguously.
#[repr(C)]
struct SharedHeader {
    nentries: u64,
    counters: CursorCounters,
}

/// Where cursors come from. Owned by the scan (behind an `Arc`) and outlives every
/// cursor claimed from it.
pub enum BitmapCursorSource {
    /// Serial: private iterators over the build owner's local TIDBitmap.
    Private {
        tbm: *mut pg_sys::TIDBitmap,
        claims: Mutex<Vec<(u32, SegmentId)>>,
        counters: Box<CursorCounters>,
    },
    /// Parallel: shared iterator states claimed from the table in the DSA area.
    Shared {
        area: *mut pg_sys::dsa_area,
        table: pg_sys::dsa_pointer,
    },
}

// SAFETY: PostgreSQL doesn't execute within threads despite Tantivy expecting it.
unsafe impl Send for BitmapCursorSource {}
unsafe impl Sync for BitmapCursorSource {}

impl BitmapCursorSource {
    pub fn private(tbm: *mut pg_sys::TIDBitmap) -> Self {
        Self::Private {
            tbm,
            claims: Mutex::new(Vec::new()),
            counters: Box::default(),
        }
    }

    /// Attach a published claim table (workers; the owner uses the same view).
    pub fn shared(area: *mut pg_sys::dsa_area, table: pg_sys::dsa_pointer) -> Self {
        Self::Shared { area, table }
    }

    /// Claim the `(consumer, segment)` stream and open its cursor.
    ///
    /// Exactly one scorer may consume each stream. A second claim — or a stream
    /// absent from the table — means a collector broke the one-scorer-per-stream
    /// invariant, and raises an execution error rather than silently degrading.
    pub unsafe fn claim(&self, consumer_id: u32, segment: SegmentId) -> BitmapCursor {
        unsafe {
            match self {
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
                    BitmapCursor::private(*tbm, counters.as_ref() as *const CursorCounters)
                }
                Self::Shared { area, table } => {
                    let header = pg_sys::dsa_get_address(*area, *table).cast::<SharedHeader>();
                    let entries = header.add(1).cast::<SharedEntry>();
                    let nentries = (*header).nentries as usize;
                    for i in 0..nentries {
                        let entry = &*entries.add(i);
                        if entry.consumer_id == consumer_id
                            && entry.segment_id == *segment.uuid_bytes()
                        {
                            if entry
                                .claimed
                                .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
                                .is_err()
                            {
                                pgrx::error!(
                                    "bitmap intersection stream (consumer {consumer_id}, segment {}) claimed twice",
                                    segment.uuid_string()
                                );
                            }
                            let iter = pg_sys::tbm_attach_shared_iterate(*area, entry.iterator);
                            return BitmapCursor::shared(
                                iter,
                                &(*header).counters as *const CursorCounters,
                            );
                        }
                    }
                    pgrx::error!(
                        "bitmap intersection stream (consumer {consumer_id}, segment {}) missing from the shared table",
                        segment.uuid_string()
                    );
                }
            }
        }
    }

    /// Counter totals for EXPLAIN ANALYZE.
    pub fn counters(&self) -> (u64, u64, u64, u64) {
        unsafe {
            let c = match self {
                Self::Private { counters, .. } => counters.as_ref() as *const CursorCounters,
                Self::Shared { area, table } => {
                    let header = pg_sys::dsa_get_address(*area, *table).cast::<SharedHeader>();
                    &(*header).counters as *const CursorCounters
                }
            };
            (
                (*c).exact_pages.load(Ordering::Relaxed),
                (*c).lossy_pages.load(Ordering::Relaxed),
                (*c).recheck_pages.load(Ordering::Relaxed),
                (*c).rejected_docs.load(Ordering::Relaxed),
            )
        }
    }
}

/// Build owner only: prepare one shared iteration state per `(consumer, segment)`
/// stream and publish the claim table into `area`. The TIDBitmap must have been
/// created over the same `area`.
pub unsafe fn publish_shared_table(
    tbm: *mut pg_sys::TIDBitmap,
    area: *mut pg_sys::dsa_area,
    consumers: u32,
    segments: &[SegmentId],
) -> pg_sys::dsa_pointer {
    unsafe {
        let nentries = consumers as usize * segments.len();
        let size =
            std::mem::size_of::<SharedHeader>() + nentries * std::mem::size_of::<SharedEntry>();
        let table = pg_sys::dsa_allocate_extended(area, size, pg_sys::DSA_ALLOC_ZERO as _);
        let header = pg_sys::dsa_get_address(area, table).cast::<SharedHeader>();
        (*header).nentries = nentries as u64;
        let entries = header.add(1).cast::<SharedEntry>();
        let mut i = 0;
        for consumer_id in 0..consumers {
            for segment in segments {
                let entry = &mut *entries.add(i);
                entry.consumer_id = consumer_id;
                entry.segment_id = *segment.uuid_bytes();
                entry.iterator = pg_sys::tbm_prepare_shared_iterate(tbm);
                i += 1;
            }
        }
        table
    }
}

/// Build owner only, after all consumers have stopped: free every prepared
/// iteration state and the claim table itself.
pub unsafe fn free_shared_table(area: *mut pg_sys::dsa_area, table: pg_sys::dsa_pointer) {
    unsafe {
        let header = pg_sys::dsa_get_address(area, table).cast::<SharedHeader>();
        let entries = header.add(1).cast::<SharedEntry>();
        for i in 0..(*header).nentries as usize {
            pg_sys::tbm_free_shared_area(area, (*entries.add(i)).iterator);
        }
        pg_sys::dsa_free(area, table);
    }
}

/// Version-specific iterator handle.
enum CursorIter {
    #[cfg(not(feature = "pg18"))]
    Private(*mut pg_sys::TBMIterator),
    #[cfg(feature = "pg18")]
    Private(*mut pg_sys::TBMPrivateIterator),
    Shared(*mut pg_sys::TBMSharedIterator),
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
pub struct BitmapCursor {
    iter: CursorIter,
    state: PageState,
    offsets: [pg_sys::OffsetNumber; OFFSETS_CAP],
    counters: *const CursorCounters,
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

    fn shared(iter: *mut pg_sys::TBMSharedIterator, counters: *const CursorCounters) -> Self {
        Self::new(CursorIter::Shared(iter), counters)
    }

    fn new(iter: CursorIter, counters: *const CursorCounters) -> Self {
        Self {
            iter,
            state: PageState::NotStarted,
            offsets: [0; OFFSETS_CAP],
            counters,
            #[cfg(debug_assertions)]
            last_ctid: 0,
        }
    }

    /// Probe one ctid. Ctids must arrive in nondecreasing order (the ctid-sorted
    /// planner gate guarantees it per stream); duplicates are fine.
    pub unsafe fn probe(&mut self, ctid: u64) -> TidProbe {
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
        unsafe {
            (*self.counters)
                .rejected_docs
                .fetch_add(1, Ordering::Relaxed)
        };
    }

    unsafe fn count_page(&self, lossy: bool, recheck: bool) {
        unsafe {
            let c = &*self.counters;
            if lossy {
                c.lossy_pages.fetch_add(1, Ordering::Relaxed);
            } else {
                c.exact_pages.fetch_add(1, Ordering::Relaxed);
                if recheck {
                    c.recheck_pages.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    #[cfg(feature = "pg18")]
    unsafe fn next_page(&mut self) {
        unsafe {
            let mut res = pg_sys::TBMIterateResult::default();
            let more = match &mut self.iter {
                CursorIter::Private(iter) => pg_sys::tbm_private_iterate(*iter, &mut res),
                CursorIter::Shared(iter) => pg_sys::tbm_shared_iterate(*iter, &mut res),
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
    unsafe fn next_page(&mut self) {
        unsafe {
            let res = match &mut self.iter {
                CursorIter::Private(iter) => pg_sys::tbm_iterate(*iter),
                CursorIter::Shared(iter) => pg_sys::tbm_shared_iterate(*iter),
            };
            if res.is_null() {
                self.state = PageState::Exhausted;
                return;
            }
            let lossy = (*res).ntuples < 0;
            let noffsets = if lossy {
                0
            } else {
                let n = ((*res).ntuples as usize).min(OFFSETS_CAP);
                // The result struct is reused by the next iterate call; copy out.
                std::ptr::copy_nonoverlapping(
                    (*res).offsets.as_ptr(),
                    self.offsets.as_mut_ptr(),
                    n,
                );
                n
            };
            self.count_page(lossy, (*res).recheck);
            self.state = PageState::Page {
                block: (*res).blockno,
                lossy,
                recheck: (*res).recheck,
                noffsets,
                pos: 0,
            };
        }
    }
}

impl Drop for BitmapCursor {
    fn drop(&mut self) {
        unsafe {
            match self.iter {
                #[cfg(not(feature = "pg18"))]
                CursorIter::Private(iter) => pg_sys::tbm_end_iterate(iter),
                #[cfg(feature = "pg18")]
                CursorIter::Private(iter) => pg_sys::tbm_end_private_iterate(iter),
                CursorIter::Shared(iter) => pg_sys::tbm_end_shared_iterate(iter),
            }
        }
    }
}

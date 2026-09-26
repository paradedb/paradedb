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

use std::collections::VecDeque;
use std::io;
use std::ops::{Deref, Range};
use std::sync::Arc;

use crate::api::HashMap;
use crate::api::version::Version;
use crate::gucs::enable_visibility_map_shortcuts;
use crate::index::ctid_map::BlockToDocIdMap;
use crate::index::fast_fields_helper::FFHelper;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::composite::CompositeSlotValues;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::buffer::{BorrowedBuffer, BufferManager, PinnedBuffer};
use crate::postgres::utils;
use crate::schema::{CategorizedFieldData, FieldSource, SearchField};
use parking_lot::Mutex;
use pgrx::pg_sys::{self, BlockNumber};
use pgrx::{PgList, PgTupleDesc, check_for_interrupts};
use tantivy::SegmentReader;
use tantivy::index::SegmentId;
use tantivy::{DocId, Order, SegmentOrdinal, TantivyDocument};
use tantivy_common::TinySet;

// Prefer per-match CTID checks at this heap-block span per live document.
const BLOCKS_PER_DOC_FOR_LAZY_VISIBILITY: u64 = 256;

/// A pinned heap buffer that releases its pin on drop. It stays off the index block tracker
/// that `PinnedBuffer` feeds. That tracker keys blocks by number with no relation, so a heap
/// pin would collide with the index's tracked blocks under the `block_tracker` feature, and a
/// `CREATE INDEX` reads the heap while its own index buffers are tracked.
pub(crate) struct HeapBufferPin(pg_sys::Buffer);

impl HeapBufferPin {
    /// Reads and pins `blockno` of `heaprel`'s main fork.
    ///
    /// # Safety
    /// `heaprel` must stay open for the lifetime of the returned pin.
    pub(crate) unsafe fn read(heaprel: &PgSearchRelation, blockno: BlockNumber) -> Self {
        Self(pg_sys::ReadBufferExtended(
            heaprel.as_ptr(),
            pg_sys::ForkNumber::MAIN_FORKNUM,
            blockno,
            pg_sys::ReadBufferMode::RBM_NORMAL,
            std::ptr::null_mut(),
        ))
    }

    pub(crate) fn buffer(&self) -> pg_sys::Buffer {
        self.0
    }
}

crate::impl_safe_drop!(HeapBufferPin, |self| {
    unsafe {
        if crate::postgres::utils::IsTransactionState() {
            pg_sys::ReleaseBuffer(self.0);
        }
    }
});

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct SegmentVisibilityStats {
    skipped: bool,
    blocks_requiring_checks: Option<u64>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct VisibilityStats {
    blocks_total: u64,
    segments: HashMap<SegmentId, SegmentVisibilityStats>,
}

impl VisibilityStats {
    pub(crate) fn merge(&mut self, other: Self) {
        self.blocks_total = self.blocks_total.max(other.blocks_total);
        self.segments.extend(other.segments);
    }

    pub(crate) fn record_segment(&mut self, segment: &SegmentReader, skipped: bool) {
        self.segments
            .entry(segment.segment_id())
            .and_modify(|stats| stats.skipped = skipped)
            .or_insert(SegmentVisibilityStats {
                skipped,
                blocks_requiring_checks: skipped.then_some(0),
            });
    }

    /// Total counts heap blocks once; dirty counts come directly from each segment's VM scan.
    pub(crate) fn totals(&self) -> [u64; 4] {
        let mut totals = [0, 0, self.blocks_total, 0];
        for stats in self.segments.values() {
            totals[if stats.skipped { 0 } else { 1 }] += 1;
            totals[3] += stats.blocks_requiring_checks.unwrap_or_default();
        }
        // Without a VM scan, conservatively report the heap as requiring checks.
        if self
            .segments
            .values()
            .any(|stats| stats.blocks_requiring_checks.is_none())
        {
            totals[3] = self.blocks_total;
        }
        totals
    }
}

/// Helper to validate that a "ctid" is currently visible to a snapshot.
///
/// When querying ParadeDB indexes, individual ctid entries may be stale. After an UPDATE,
/// the old tuple is marked dead and a new tuple is created at a new ctid, but the
/// index still has the old ctid until VACUUM runs.
///
/// The visibility checker supports two operational modes:
/// 1. Fast-path visibility confirmation ([`VisibilityChecker::check_segment_docs`]):
///    Checks the PostgreSQL visibility map first. On all-visible blocks, visibility is
///    guaranteed for all active snapshots, so heap page access is bypassed entirely.
///    The returned CTID is the raw index CTID, which may be an index root pointing to
///    a HOT redirect (`LP_REDIRECT`). This is safe and optimal for execution plan nodes
///    like `VisibilityFilterExec` and `BatchScanner` whose downstream tuple fetcher
///    (e.g. `JoinScanState::build_result_tuple` or `BaseScan`) uses `table_index_fetch_tuple`
///    to resolve the HOT redirect to the physical tuple at final output time.
/// 2. Full physical HOT resolution ([`VisibilityChecker::resolve_segment_docs`]):
///    Forces a heap check for every tuple, bypassing the visibility map all-visible check,
///    and resolves the CTID to the exact current physical heap location of the tuple visible
///    under this snapshot. This is required only when the caller cannot resolve HOT chains
///    later (e.g. in `collect_ctidset` for bitmap index scans).
pub struct VisibilityChecker {
    scan: *mut pg_sys::IndexFetchTableData,
    snapshot: pg_sys::Snapshot,
    tid: pg_sys::ItemPointerData,

    // we hold onto this b/c `scan` points to the relation this does
    heaprel: PgSearchRelation,
    bman: BufferManager,

    vm_block_no: Option<BlockNumber>,
    vmbuff: pg_sys::Buffer,
    // tracks our previous block visibility so we can elide checking again
    blockvis: (BlockNumber, bool),

    /// Cached relation size (in blocks) at scan start. Used to cheaply skip
    /// stale ctids pointing to pages truncated by a previous VACUUM.
    nblocks: BlockNumber,

    /// Pin on the heap block last checked by `resolve_visible`, held across
    /// calls since consecutive checks tend to hit the same block.
    cached_heap_block: BlockNumber,
    cached_heap_pin: Option<PinnedBuffer>,

    pub heap_tuple_check_count: usize,
    pub invisible_tuple_count: usize,

    /// False for a `visibility => 'raw'` aggregate, which trades snapshot
    /// accuracy for skipping the heap: every ctid then passes as-is.
    check_visibility: bool,

    // TODO: Make this non-optional in the future once all call sites provide an FFHelper.
    ffhelper: Option<Arc<FFHelper>>,
    raw_ctids_scratch: Vec<Option<u64>>,
    dirty_blocks: HashMap<SegmentId, Arc<[Range<BlockNumber>]>>,
    /// Caches whether each segment has been proven all-visible under this checker's snapshot.
    segment_visibility: HashMap<SegmentOrdinal, bool>,
    segment_checks: HashMap<SegmentOrdinal, Option<Arc<[Range<DocId>]>>>,
    visibility_stats: Option<Arc<Mutex<VisibilityStats>>>,
}

// TODO: Use of clone results in new metrics in the clone. Should put them in `Rc<RefCell<usize>>`.
impl Clone for VisibilityChecker {
    fn clone(&self) -> Self {
        let mut checker = Self::with_rel_and_snap(&self.heaprel, self.snapshot);
        checker.check_visibility = self.check_visibility;
        checker.ffhelper = self.ffhelper.clone();
        checker.visibility_stats = self.visibility_stats.clone();
        checker
    }
}

crate::impl_safe_drop!(VisibilityChecker, |self| {
    unsafe {
        if crate::postgres::utils::IsTransactionState() {
            if self.vmbuff != pg_sys::InvalidBuffer as pg_sys::Buffer {
                pg_sys::ReleaseBuffer(self.vmbuff);
            }
            pg_sys::table_index_fetch_end(self.scan);
        }
    }
});

impl VisibilityChecker {
    /// Construct a new [`VisibilityChecker`] that can validate ctid visibility against the specified
    /// `relation` and `snapshot`
    pub fn with_rel_and_snap(heaprel: &PgSearchRelation, snapshot: pg_sys::Snapshot) -> Self {
        unsafe {
            let nblocks =
                pg_sys::RelationGetNumberOfBlocksInFork(heaprel.as_ptr(), heaprel.fork_number());
            Self {
                scan: pg_sys::table_index_fetch_begin(heaprel.as_ptr()),
                snapshot,
                tid: pg_sys::ItemPointerData::default(),
                heaprel: Clone::clone(heaprel),
                bman: BufferManager::new(heaprel),
                vm_block_no: None,
                vmbuff: pg_sys::InvalidBuffer as pg_sys::Buffer,
                blockvis: (pg_sys::InvalidBlockNumber, false),
                nblocks,
                cached_heap_block: pg_sys::InvalidBlockNumber,
                cached_heap_pin: None,
                heap_tuple_check_count: 0,
                invisible_tuple_count: 0,
                check_visibility: true,
                ffhelper: None,
                raw_ctids_scratch: Vec::new(),
                dirty_blocks: HashMap::default(),
                segment_visibility: HashMap::default(),
                segment_checks: HashMap::default(),
                visibility_stats: None,
            }
        }
    }

    pub(crate) fn with_visibility_stats(
        mut self,
        stats: Option<Arc<Mutex<VisibilityStats>>>,
    ) -> Self {
        if let Some(stats) = &stats {
            stats.lock().blocks_total = u64::from(self.nblocks);
        }
        self.visibility_stats = stats;
        self
    }

    /// Attaches an [`FFHelper`] for resolving segment `DocId`s to ctids directly.
    pub fn with_ffhelper(mut self, ffhelper: Arc<FFHelper>) -> Self {
        self.segment_checks.clear();
        self.dirty_blocks.clear();
        self.segment_visibility.clear();
        self.ffhelper = Some(ffhelper);
        self
    }

    pub fn set_ffhelper(&mut self, ffhelper: Arc<FFHelper>) {
        self.segment_checks.clear();
        self.dirty_blocks.clear();
        self.segment_visibility.clear();
        self.ffhelper = Some(ffhelper);
    }

    pub fn ffhelper(&self) -> Option<&Arc<FFHelper>> {
        self.ffhelper.as_ref()
    }

    /// Whether the heap is checked at all, the `solve_mvcc` decision of the
    /// Tantivy backend. The single- and batch-checking methods honor a `false`;
    /// the tuple-fetching helpers always check.
    pub fn with_check_visibility(mut self, check_visibility: bool) -> Self {
        self.check_visibility = check_visibility;
        self
    }

    pub fn checks_visibility(&self) -> bool {
        self.check_visibility
    }

    /// If the specified `ctid` is visible in the heap, run the provided closure and return its
    /// result as `Some(T)`.  If it's not visible, return `None` without running the provided closure.
    ///
    /// This uses table_index_fetch_tuple which is designed for ctids from an INDEX scan.
    /// For ctids from a sequential scan, use `fetch_tuple_direct` instead.
    ///
    /// NOTE: Does _not_ check the visibility map first: is for use in contexts which have already
    /// applied visibility checking if needed.
    pub fn exec_if_visible<T, F: FnMut(pg_sys::Relation) -> T>(
        &mut self,
        ctid: u64,
        slot: *mut pg_sys::TupleTableSlot,
        mut func: F,
    ) -> Option<T> {
        let blockno = (ctid >> 16) as BlockNumber;
        if blockno >= self.nblocks {
            self.invisible_tuple_count += 1;
            return None;
        }
        self.heap_tuple_check_count += 1;

        utils::u64_to_item_pointer(ctid, &mut self.tid);

        let mut call_again = false;
        let mut all_dead = false;
        let found = unsafe {
            pg_sys::table_index_fetch_tuple(
                self.scan,
                &mut self.tid,
                self.snapshot,
                slot,
                &mut call_again,
                &mut all_dead,
            )
        };

        if found {
            Some(func(unsafe { (*self.scan).rel }))
        } else {
            self.invisible_tuple_count += 1;
            None
        }
    }

    /// Fetch a tuple directly by ctid, without going through the index fetch machinery.
    ///
    /// This is the correct method to use when the ctid was obtained from a sequential scan
    /// (e.g., from building a hash table). Unlike exec_if_visible which uses table_index_fetch_tuple
    /// and handles HOT chains from index ctids, this uses table_tuple_fetch_row_version which
    /// directly fetches the tuple at the given ctid.
    ///
    /// Returns true if the tuple was found and visible, false otherwise.
    pub fn fetch_tuple_direct(&self, ctid: u64, slot: *mut pg_sys::TupleTableSlot) -> bool {
        unsafe {
            let blockno = (ctid >> 16) as BlockNumber;
            if blockno >= self.nblocks {
                return false;
            }

            let mut tid = pg_sys::ItemPointerData::default();
            utils::u64_to_item_pointer(ctid, &mut tid);

            pg_sys::table_tuple_fetch_row_version(
                self.heaprel.as_ptr(),
                &mut tid,
                self.snapshot,
                slot,
            )
        }
    }

    /// Returns true if the block is all visible.
    pub fn is_block_all_visible(&mut self, blockno: BlockNumber) -> bool {
        if blockno == self.blockvis.0 {
            return self.blockvis.1;
        }
        self.blockvis.0 = blockno;

        let vm_block_no = blockno / util::HEAPBLOCKS_PER_PAGE;
        unsafe {
            let status = if Some(vm_block_no) == self.vm_block_no
                && self.vmbuff != pg_sys::InvalidBuffer as pg_sys::Buffer
            {
                debug_assert_eq!(
                    pg_sys::BufferGetBlockNumber(self.vmbuff),
                    vm_block_no,
                    "pinned vmbuff does not cover the expected VM mapBlock"
                );
                // Fast path: we already hold a pinned, valid `vmbuff` for exactly this
                // mapBlock, so the C function is guaranteed to take its bit-math branch
                // and will NOT call `vm_readbuf`. That makes it safe to skip the pgrx
                // `pg_guard` wrapper and avoid its per-call overhead.
                // See `raw::visibilitymap_get_status` for the safety contract.
                util::visibilitymap_get_status(self.heaprel.as_ptr(), blockno, &mut self.vmbuff)
            } else {
                // Slow path: either we have no pinned VM page yet, or `blockno` crossed a
                // VM-page boundary. The C function may release the old buffer and call
                // `vm_readbuf` (which can `ereport`), so we MUST go through the guarded
                // wrapper. This also (re)pins `vmbuff` to the correct mapBlock so the
                // fast path can be taken on subsequent calls.
                pg_sys::visibilitymap_get_status(self.heaprel.as_ptr(), blockno, &mut self.vmbuff)
            };

            self.vm_block_no = Some(vm_block_no);
            self.blockvis.1 = status != 0;
        }
        self.blockvis.1
    }

    /// Returns `Some(self)` if the segment requires visibility checking, or `None` if the segment
    /// is proven all-visible under this checker's snapshot.
    ///
    /// Callers can use this to bypass buffering, visibility filtering, and compaction for all-visible
    /// segments, and avoid reading CTIDs unless explicitly required.
    pub(crate) fn for_segment(
        &mut self,
        segment_ord: SegmentOrdinal,
    ) -> tantivy::Result<Option<&mut Self>> {
        Ok((!self.is_segment_all_visible(segment_ord)?).then_some(self))
    }

    /// Convenience helper for callers holding an `Arc<Mutex<VisibilityChecker>>`.
    ///
    /// Returns `Some(checker.clone())` if the segment requires visibility checking, or `None` if
    /// the segment is proven all-visible.
    pub(crate) fn for_segment_arc(
        checker: &Arc<Mutex<Self>>,
        segment_ord: SegmentOrdinal,
    ) -> tantivy::Result<Option<Arc<Mutex<Self>>>> {
        Ok((!checker.lock().is_segment_all_visible(segment_ord)?).then(|| checker.clone()))
    }

    /// Checks whether all documents in the segment are guaranteed to be visible under this checker's snapshot.
    ///
    /// The proof requires an MVCC snapshot, an immutable segment with non-zero documents, and either:
    /// 1. An empty set of missing ranges from heap-block presence maps, or
    /// 2. Heap block bounds confirmed all-visible in Postgres's visibility map.
    ///
    /// Results are cached per segment in `self.segment_visibility`.
    pub(crate) fn is_segment_all_visible(
        &mut self,
        segment_ord: SegmentOrdinal,
    ) -> tantivy::Result<bool> {
        if !enable_visibility_map_shortcuts() {
            return Ok(false);
        }
        if let Some(&visible) = self.segment_visibility.get(&segment_ord) {
            return Ok(visible);
        }
        let Some(ffhelper) = self.ffhelper.clone() else {
            return Ok(false);
        };
        let Some(segment) = ffhelper.immutable_segment_reader(segment_ord) else {
            return Ok(false);
        };
        // prove segment is all visible
        let visible = 'proof: {
            if self.snapshot.is_null()
                || unsafe { (*self.snapshot).snapshot_type != pg_sys::SnapshotType::SNAPSHOT_MVCC }
                || segment.num_docs() == 0
            {
                break 'proof false;
            }
            if self
                .segment_checks
                .get(&segment_ord)
                .and_then(|ranges| ranges.as_ref())
                .is_some_and(|ranges| ranges.is_empty())
            {
                break 'proof true;
            }
            let Some(blocks) =
                SearchIndexReader::block_bounds(segment).map_err(io::Error::other)?
            else {
                break 'proof false;
            };
            let (first, last) = (*blocks.start(), *blocks.end());
            // Read CTID bounds before fresh VM bits; FFHelper retains the VACUUM cleanup pin.
            if first > last || last == pg_sys::InvalidBlockNumber {
                break 'proof false;
            }
            self.dirty_blocks_for_segment(segment, first..last + 1)
                .is_some_and(|blocks| blocks.is_empty())
        };
        if visible {
            self.segment_checks
                .entry(segment_ord)
                .or_insert_with(|| Some(Arc::from([])));
        }
        if let Some(stats) = &self.visibility_stats {
            stats.lock().record_segment(segment, visible);
        }
        self.segment_visibility.insert(segment_ord, visible);
        Ok(visible)
    }

    /// Retains dirty ranges from one VM scan, or returns None to check sparse segments lazily.
    fn dirty_blocks_for_segment(
        &mut self,
        segment: &SegmentReader,
        blocks: Range<BlockNumber>,
    ) -> Option<Arc<[Range<BlockNumber>]>> {
        let span = u64::from(blocks.end) - u64::from(blocks.start);
        if span >= BLOCKS_PER_DOC_FOR_LAZY_VISIBILITY * u64::from(segment.num_docs()) {
            return None;
        }
        let segment_id = segment.segment_id();
        if let Some(ranges) = self.dirty_blocks.get(&segment_id) {
            return Some(ranges.clone());
        }
        Some(if self.visibility_stats.is_some() {
            self.scan_dirty_blocks::<true>(segment_id, blocks)
        } else {
            self.scan_dirty_blocks::<false>(segment_id, blocks)
        })
    }

    /// Compiles statistics work out of the ordinary-query scan.
    fn scan_dirty_blocks<const COLLECT_STATS: bool>(
        &mut self,
        segment_id: SegmentId,
        blocks: Range<BlockNumber>,
    ) -> Arc<[Range<BlockNumber>]> {
        self.blockvis = (pg_sys::InvalidBlockNumber, false);
        let mut block = blocks.start / 32 * 32;
        let mut scratch = vec![TinySet::range_lower(32); util::HEAPBLOCKS_PER_PAGE as usize / 32];
        let mut ranges: Vec<Range<BlockNumber>> = Vec::new();
        let mut dirty_count = 0;
        while block < blocks.end {
            check_for_interrupts!();
            let span = util::HEAPBLOCKS_PER_PAGE - block % util::HEAPBLOCKS_PER_PAGE;
            let words = (blocks.end - block).min(span).div_ceil(32) as usize;
            let missing = &mut scratch[..words];
            missing.fill(TinySet::range_lower(32));
            self.retain_invisible_blocks(block, missing);
            for (word, &bits) in missing.iter().enumerate() {
                let mut bits = u64::from_le_bytes(bits.into_bytes());
                if COLLECT_STATS {
                    let word_start = block + word as BlockNumber * 32;
                    let first = blocks.start.saturating_sub(word_start);
                    let end = (blocks.end - word_start).min(32);
                    let counted_bits = bits & (u64::MAX << first) & (u64::MAX >> (64 - end));
                    dirty_count += u64::from(counted_bits.count_ones());
                }
                while bits != 0 {
                    let bit = bits.trailing_zeros();
                    let len = (bits >> bit).trailing_ones();
                    bits &= !((u64::MAX >> (64 - len)) << bit);
                    let start = block + word as BlockNumber * 32 + bit;
                    let range = start.max(blocks.start)..start.saturating_add(len).min(blocks.end);
                    if range.is_empty() {
                        continue;
                    }
                    if let Some(last) = ranges.last_mut().filter(|last| last.end == range.start) {
                        last.end = range.end;
                    } else {
                        ranges.push(range);
                    }
                }
            }
            block = block.saturating_add(words as BlockNumber * 32);
        }
        if COLLECT_STATS {
            self.visibility_stats
                .as_ref()
                .expect("visibility instrumentation must be enabled")
                .lock()
                .segments
                .entry(segment_id)
                .or_default()
                .blocks_requiring_checks = Some(dirty_count);
        }
        let ranges: Arc<[Range<BlockNumber>]> = ranges.into();
        self.dirty_blocks.insert(segment_id, ranges.clone());
        ranges
    }

    /// Clears all-visible heap blocks from the candidate bitmap, pinning each VM page once.
    fn retain_invisible_blocks(&mut self, first_block: BlockNumber, mut blocks: &mut [TinySet]) {
        const ALL_VISIBLE_BITS: u64 = 0x5555_5555_5555_5555;
        const LOW_PAIR_PER_NIBBLE: u64 = 0x3333_3333_3333_3333;
        const LOW_NIBBLE_PER_BYTE: u64 = 0x0f0f_0f0f_0f0f_0f0f;
        const LOW_BYTE_PER_U16: u64 = 0x00ff_00ff_00ff_00ff;
        const LOW_U16_PER_U32: u64 = 0x0000_ffff_0000_ffff;

        assert!(first_block.is_multiple_of(32));
        let mut block = u64::from(first_block);
        while !blocks.is_empty() && block < u64::from(self.nblocks) {
            pgrx::check_for_interrupts!();
            let blockno = block as u32;
            let page_offset = blockno % util::HEAPBLOCKS_PER_PAGE;
            let words = blocks
                .len()
                .min(((util::HEAPBLOCKS_PER_PAGE - page_offset) / 32) as usize);
            let (page_blocks, remaining) = blocks.split_at_mut(words);
            let valid = (self.nblocks - blockno).min(words as u32 * 32);
            let valid_words = valid.div_ceil(32) as usize;
            let last = page_blocks[valid_words - 1];
            self.is_block_all_visible(blockno);
            if self.vmbuff != pg_sys::InvalidBuffer as pg_sys::Buffer {
                let map = unsafe {
                    pg_sys::PageGetContents(pg_sys::BufferGetPage(self.vmbuff))
                        .cast::<u8>()
                        .add((page_offset / util::HEAPBLOCKS_PER_BYTE) as usize)
                };
                // The VM alternates all-visible and all-frozen bits. Keep only all-visible bits,
                // then pack them into a u32 by doubling the occupied group size at each step.
                for (word, blocks) in page_blocks[..valid_words].iter_mut().enumerate() {
                    if blocks.is_empty() {
                        continue;
                    }
                    // The pin keeps the page allocated, but other backends can change its bits.
                    // Read an owned, byte-aligned value without borrowing the shared page.
                    let bytes = unsafe { map.add(word * 8).cast::<[u8; 8]>().read_volatile() };
                    let mut visible = u64::from_le_bytes(bytes) & ALL_VISIBLE_BITS;
                    if visible == ALL_VISIBLE_BITS {
                        blocks.clear();
                        continue;
                    }
                    visible = (visible | (visible >> 1)) & LOW_PAIR_PER_NIBBLE;
                    visible = (visible | (visible >> 2)) & LOW_NIBBLE_PER_BYTE;
                    visible = (visible | (visible >> 4)) & LOW_BYTE_PER_U16;
                    visible = (visible | (visible >> 8)) & LOW_U16_PER_U32;
                    visible = (visible | (visible >> 16)) & u64::from(u32::MAX);
                    *blocks = blocks.intersect(TinySet::deserialize((!visible).to_le_bytes()));
                }
                if !valid.is_multiple_of(32) {
                    page_blocks[valid_words - 1] = page_blocks[valid_words - 1]
                        .union(last.intersect(TinySet::range_greater_or_equal(valid % 32)));
                }
            }
            block += words as u64 * 32;
            blocks = remaining;
        }
    }

    /// Single-ctid visibility check for callers probing one doc at a time
    /// (e.g. the cardinality fast path's visibility filter).
    pub fn check_one(&mut self, ctid: u64) -> bool {
        !self.check_visibility || self.resolve_visible(ctid, None, false).is_some()
    }

    /// Caches the document ranges needing visibility checks for this segment and snapshot.
    fn doc_id_ranges_needing_visibility_checks(
        &mut self,
        segment_ord: SegmentOrdinal,
    ) -> Option<Arc<[Range<DocId>]>> {
        if !enable_visibility_map_shortcuts() {
            return None;
        }
        if !self.segment_checks.contains_key(&segment_ord) {
            // Map dirty VM pages to document ranges once per snapshot.
            let ranges = (|| -> anyhow::Result<Option<Vec<Range<DocId>>>> {
                if self.snapshot.is_null()
                    || unsafe {
                        (*self.snapshot).snapshot_type != pg_sys::SnapshotType::SNAPSHOT_MVCC
                    }
                {
                    return Ok(None);
                }
                let ffhelper = self.ffhelper.clone().expect("FFHelper must be configured");
                let Some(segment) = ffhelper.immutable_segment_reader(segment_ord) else {
                    return Ok(None);
                };
                let descending = ffhelper
                    .sort_order()
                    .is_some_and(|sort| sort.order == Order::Desc);
                let Some(mut map) = BlockToDocIdMap::open(segment)? else {
                    return Ok(None);
                };
                // Reuse the VM scan from the segment proof, or scan once on first use.
                let Some(block_ranges) = self.dirty_blocks_for_segment(segment, map.block_range())
                else {
                    return Ok(None);
                };
                const RANGES_PER_BATCH: usize = 128;
                let mut ranges = Vec::new();
                for blocks in block_ranges.chunks(RANGES_PER_BATCH) {
                    ranges.extend(map.doc_id_ranges_for_blocks(blocks)?);
                }
                // Coalesce adjacent document ranges across batch boundaries.
                ranges.dedup_by(|next, previous| {
                    if previous.end == next.start {
                        previous.end = next.end;
                        true
                    } else {
                        false
                    }
                });
                if descending {
                    for range in &mut ranges {
                        *range = segment.max_doc() - range.end..segment.max_doc() - range.start;
                    }
                    ranges.reverse();
                }
                Ok(Some(ranges))
            })()
            .expect("failed to read heap-block visibility metadata")
            .map(Arc::from);
            self.segment_checks.insert(segment_ord, ranges);
        }
        self.segment_checks[&segment_ord].clone()
    }

    /// Checks visibility without fetching CTIDs for documents outside unresolved ranges.
    pub(crate) fn check_segment_docs_mask(
        &mut self,
        segment_ord: SegmentOrdinal,
        doc_ids: &[DocId],
        mask: &mut [bool],
    ) {
        assert_eq!(doc_ids.len(), mask.len());
        mask.fill(true);
        if doc_ids.is_empty() || !self.check_visibility {
            return;
        }
        assert!(
            doc_ids.is_sorted(),
            "visibility batches must be in doc ID order"
        );
        let ranges = self.doc_id_ranges_needing_visibility_checks(segment_ord);
        let ffhelper = self
            .ffhelper
            .clone()
            .expect("FFHelper must be configured to check segment doc visibility");
        let mut raw_ctids = std::mem::take(&mut self.raw_ctids_scratch);
        let mut ctids = Vec::new();
        let mut check = |start: usize, end: usize| {
            if start == end {
                return;
            }
            raw_ctids.resize(end - start, None);
            ctids.resize(end - start, None);
            ffhelper
                .ctid(segment_ord)
                .as_u64s(&doc_ids[start..end], &mut raw_ctids);
            self.check_raw_ctids_impl(&raw_ctids, &mut ctids, false);
            for (visible, ctid) in mask[start..end].iter_mut().zip(&ctids) {
                *visible = ctid.is_some();
            }
        };
        if let Some(ranges) = ranges {
            let mut start = 0;
            let first_range = ranges.partition_point(|range| range.end <= doc_ids[0]);
            let mut remaining_ranges = &ranges[first_range..];
            while let Some((range, rest)) = remaining_ranges.split_first() {
                remaining_ranges = rest;
                start += doc_ids[start..].partition_point(|&doc| doc < range.start);
                if start == doc_ids.len() {
                    break;
                }
                let end = start + doc_ids[start..].partition_point(|&doc| doc < range.end);
                if start == end {
                    // Jump over ranges that end before the next query match.
                    let skip =
                        remaining_ranges.partition_point(|range| range.end <= doc_ids[start]);
                    remaining_ranges = &remaining_ranges[skip..];
                    continue;
                }
                check(start, end);
                start = end;
            }
        } else {
            check(0, doc_ids.len());
        }
        self.raw_ctids_scratch = raw_ctids;
    }

    /// Checks if a slice of `DocId`s within a segment are visible, fetching ctids directly from
    /// the configured [`FFHelper`].
    ///
    /// For all-visible blocks, visibility is confirmed via the visibility map fast-path without
    /// reading heap buffers. The returned CTID for an all-visible block is the raw index CTID,
    /// which may be an index root pointing to a HOT redirect (`LP_REDIRECT`).
    ///
    /// This is safe and optimal when callers (such as `VisibilityFilterExec` or `BatchScanner`)
    /// pass the CTID to a downstream tuple fetcher (such as `JoinScanState::build_result_tuple`
    /// or `BaseScan::check_visibility`) that follows HOT redirects via `table_index_fetch_tuple`
    /// (`exec_if_visible`) for the final surviving rows, avoiding heap page reads for candidate
    /// rows discarded during query execution.
    pub fn check_segment_docs(
        &mut self,
        segment_ord: SegmentOrdinal,
        doc_ids: &[DocId],
        results: &mut [Option<u64>],
    ) {
        self.check_segment_docs_impl(segment_ord, doc_ids, results, false);
    }

    /// Checks if a slice of `DocId`s within a segment are visible and resolves each CTID to the
    /// physical HOT member visible under this checker's snapshot, fetching ctids directly from
    /// the configured [`FFHelper`].
    ///
    /// Unlike [`Self::check_segment_docs`], this forces a heap buffer read and executes `heap_hot_search_buffer`
    /// for every tuple even on all-visible blocks, guaranteeing that the returned CTID is the exact
    /// physical heap location of the visible tuple (never an index root redirect).
    ///
    /// NOTE: This touches shared buffers for every block and is significantly more expensive than
    /// [`Self::check_segment_docs`]. Use this only when the caller requires the physical heap CTID directly
    /// and will not perform an index-based heap fetch (e.g. in `SearchIndexReader::collect_ctidset`).
    pub fn resolve_segment_docs(
        &mut self,
        segment_ord: SegmentOrdinal,
        doc_ids: &[DocId],
        results: &mut [Option<u64>],
    ) {
        self.check_segment_docs_impl(segment_ord, doc_ids, results, true);
    }

    fn check_segment_docs_impl(
        &mut self,
        segment_ord: SegmentOrdinal,
        doc_ids: &[DocId],
        results: &mut [Option<u64>],
        resolve_hot: bool,
    ) {
        if doc_ids.is_empty() {
            return;
        }
        assert_eq!(doc_ids.len(), results.len());

        let ffhelper = self
            .ffhelper
            .clone()
            .expect("FFHelper must be configured to check segment doc visibility");

        let mut raw_ctids = std::mem::take(&mut self.raw_ctids_scratch);
        raw_ctids.resize(doc_ids.len(), None);
        ffhelper.ctid(segment_ord).as_u64s(doc_ids, &mut raw_ctids);

        if !self.check_visibility {
            results.copy_from_slice(&raw_ctids);
        } else if !resolve_hot
            && doc_ids.is_sorted()
            && let Some(ranges) = self.doc_id_ranges_needing_visibility_checks(segment_ord)
        {
            results.copy_from_slice(&raw_ctids);
            let mut start = 0;
            let first_range = ranges.partition_point(|range| range.end <= doc_ids[0]);
            let mut remaining_ranges = &ranges[first_range..];
            while let Some((range, rest)) = remaining_ranges.split_first() {
                remaining_ranges = rest;
                start += doc_ids[start..].partition_point(|&doc| doc < range.start);
                if start == doc_ids.len() {
                    break;
                }
                let end = start + doc_ids[start..].partition_point(|&doc| doc < range.end);
                if start == end {
                    // Jump over ranges that end before the next query match.
                    let skip =
                        remaining_ranges.partition_point(|range| range.end <= doc_ids[start]);
                    remaining_ranges = &remaining_ranges[skip..];
                    continue;
                }
                self.check_raw_ctids_impl(&raw_ctids[start..end], &mut results[start..end], false);
                start = end;
            }
        } else {
            self.check_raw_ctids_impl(&raw_ctids, results, resolve_hot);
        }

        self.raw_ctids_scratch = raw_ctids;
    }

    fn check_raw_ctids_impl(
        &mut self,
        ctids: &[Option<u64>],
        results: &mut [Option<u64>],
        resolve_hot: bool,
    ) {
        if ctids.is_empty() {
            return;
        }
        assert_eq!(ctids.len(), results.len());
        if !self.check_visibility {
            results.copy_from_slice(ctids);
            return;
        }

        let mut sorted_indices: Vec<(usize, u64)> = ctids
            .iter()
            .map(|maybe_ctid| maybe_ctid.expect("All rows must have ctids."))
            .enumerate()
            .collect();
        sorted_indices.sort_unstable_by_key(|(_, ctid)| *ctid);

        let mut current_buffer: Option<crate::postgres::storage::buffer::Buffer> = None;
        let mut current_block = pg_sys::InvalidBlockNumber;

        for (idx, ctid) in sorted_indices {
            let blockno = (ctid >> 16) as BlockNumber;
            // acquire the block's buffer once per run of same-block ctids and
            // hold its lock across the run; resolve_visible's own VM re-check
            // hits the blockvis cache
            let needs_heap_check =
                blockno < self.nblocks && (resolve_hot || !self.is_block_all_visible(blockno));
            let locked_buffer = if needs_heap_check {
                if current_block != blockno {
                    drop(current_buffer.take());
                    current_buffer = Some(self.bman.get_buffer(blockno));
                    current_block = blockno;
                }
                Some(*current_buffer.as_ref().unwrap().deref())
            } else {
                None
            };
            results[idx] = self.resolve_visible(ctid, locked_buffer, resolve_hot);
        }
    }

    /// Resolves a ctid to its visible ctid under the checker's snapshot,
    /// following any HOT chain; `None` if invisible or stale. A caller
    /// already holding a pin and share lock on the ctid's block passes the
    /// buffer; otherwise the tuple is checked under a short share lock,
    /// reusing a cached pin on the heap block across calls since consecutive
    /// checks tend to hit the same block.
    fn resolve_visible(
        &mut self,
        ctid: u64,
        locked_buffer: Option<pg_sys::Buffer>,
        resolve_hot: bool,
    ) -> Option<u64> {
        let blockno = (ctid >> 16) as BlockNumber;
        if blockno >= self.nblocks {
            self.invisible_tuple_count += 1;
            return None;
        }
        if !resolve_hot && self.is_block_all_visible(blockno) {
            return Some(ctid);
        }
        self.heap_tuple_check_count += 1;
        let (buffer, _lock) = match locked_buffer {
            Some(buffer) => (buffer, None),
            None => {
                if self.cached_heap_block != blockno {
                    drop(self.cached_heap_pin.take());
                    self.cached_heap_pin = Some(self.bman.pinned_buffer(blockno));
                    self.cached_heap_block = blockno;
                }
                let pg_buffer = self.cached_heap_pin.as_ref().unwrap().pg_buffer();
                (
                    pg_buffer,
                    Some(unsafe { BorrowedBuffer::from_pg(pg_buffer) }),
                )
            }
        };

        unsafe {
            utils::u64_to_item_pointer(ctid, &mut self.tid);

            let mut heap_tuple_data: pg_sys::HeapTupleData = std::mem::zeroed();
            let mut all_dead = false;

            let found = pg_sys::heap_hot_search_buffer(
                &mut self.tid,
                self.heaprel.as_ptr(),
                buffer,
                self.snapshot,
                &mut heap_tuple_data,
                &mut all_dead,
                true, // first_call
            );

            if found {
                Some(utils::item_pointer_to_u64(self.tid))
            } else {
                self.invisible_tuple_count += 1;
                None
            }
        }
    }
}

/// A wrapper for an owned scan and slot for repeated use with table_index_fetch_tuple.
///
/// TODO: Similar to `VisibilityChecker`, but uses an owned slot in the shape of the table, rather
/// than borrowing a slot in the shape of the custom scan.
#[derive(Debug)]
pub struct HeapFetchState {
    pub scan: *mut pg_sys::IndexFetchTableData,
    slot: *mut pg_sys::BufferHeapTupleTableSlot,
    // A virtual view of the fetched tuple, handed to expression evaluation by
    // `fetch_eval_slot`. See that method for why a virtual slot is required.
    virtual_slot: *mut pg_sys::TupleTableSlot,
    // Hold a reference to the heap relation to keep it open for the lifetime of the scan.
    // The scan stores an internal pointer to the relation, so it must not be closed early.
    _heaprel: PgSearchRelation,

    /// Cached relation size (in blocks) at scan start. Used to cheaply skip
    /// stale ctids pointing to pages truncated by a previous VACUUM.
    nblocks: BlockNumber,
}

impl HeapFetchState {
    /// Create a HeapFetchState which will fetch the entire content of the given relation.
    pub fn new(heaprel: &PgSearchRelation) -> Self {
        unsafe {
            let scan = pg_sys::table_index_fetch_begin(heaprel.as_ptr());
            let slot = pg_sys::MakeTupleTableSlot(heaprel.rd_att, &pg_sys::TTSOpsBufferHeapTuple);
            let virtual_slot = pg_sys::MakeTupleTableSlot(heaprel.rd_att, &pg_sys::TTSOpsVirtual);
            let nblocks =
                pg_sys::RelationGetNumberOfBlocksInFork(heaprel.as_ptr(), heaprel.fork_number());
            Self {
                scan,
                slot: slot.cast(),
                virtual_slot,
                _heaprel: heaprel.clone(),
                nblocks,
            }
        }
    }

    /// The slot that holds the most recently fetched tuple, as the generic
    /// `TupleTableSlot` type that executor APIs accept.
    ///
    /// This is the raw storage slot: it is the target of the heap fetch and keeps
    /// the heap buffer pinned. [`Self::buffer_heap_slot`] returns the *same* slot
    /// as its concrete `BufferHeapTupleTableSlot` type, for reading
    /// buffer-heap-only fields. To evaluate a PostgreSQL expression against a
    /// fetched tuple, use [`Self::fetch_eval_slot`] instead, which presents the
    /// tuple as a virtual slot the executor can always consume.
    pub fn slot(&self) -> *mut pg_sys::TupleTableSlot {
        self.slot.cast()
    }

    /// The same slot as [`Self::slot`], but as its concrete
    /// `BufferHeapTupleTableSlot` type so callers can read buffer-heap-only fields
    /// such as `buffer` and `base.tuple`. (Rust raw pointers don't upcast
    /// implicitly, so we expose both rather than casting at every call site.)
    pub fn buffer_heap_slot(&self) -> *mut pg_sys::BufferHeapTupleTableSlot {
        self.slot
    }

    /// Fetch the tuple at `ctid` and return it as a virtual slot ready for
    /// PostgreSQL expression evaluation, or `None` if it is not visible.
    ///
    /// The executor may compile a scan `Var` into the `ExecJustScanVarVirt` fast
    /// path (which requires a virtual slot) when the owning plan node advertises
    /// virtual scan-slot ops, as the aggregate custom scan does. Handing it the
    /// buffer-heap fetch slot there trips `Assert(TTS_IS_VIRTUAL(slot))`. A
    /// virtual slot is accepted by both the fast and generic evaluation paths, so
    /// we always present one here.
    pub unsafe fn fetch_eval_slot(
        &self,
        ctid: &mut pg_sys::ItemPointerData,
        snapshot: pg_sys::Snapshot,
    ) -> Option<*mut pg_sys::TupleTableSlot> {
        // `call_again`/`all_dead` are only meaningful when walking a HOT chain
        // for every matching tuple (e.g. a SnapshotAny scan). Callers pass an MVCC
        // snapshot, for which `table_index_fetch_tuple` returns the single visible
        // version directly, so we take that one and ignore both -- as the
        // query-visible path in `mvcc.rs` does.
        let mut call_again = false;
        let mut all_dead = false;
        if !self.fetch_tuple(ctid, snapshot, &mut call_again, &mut all_dead) {
            return None;
        }

        // Present the fetched tuple through the virtual slot. Deform it and
        // shallow-copy the resulting value/null arrays into the virtual slot --
        // byref values still point into the pinned heap buffer, which is sound
        // because the caller evaluates the expression immediately, while this
        // `HeapFetchState` still holds the buffer pin. We deliberately avoid
        // `ExecCopySlot`, which would materialize (palloc + copy) every varlena
        // column on every row.
        let src = self.slot();
        let dst = self.virtual_slot;

        pg_sys::ExecClearTuple(dst);
        pg_sys::slot_getallattrs(src);

        let natts = (*src).tts_nvalid as usize;
        std::ptr::copy_nonoverlapping((*src).tts_values, (*dst).tts_values, natts);
        std::ptr::copy_nonoverlapping((*src).tts_isnull, (*dst).tts_isnull, natts);

        Some(pg_sys::ExecStoreVirtualTuple(dst))
    }

    /// Wrapper around `table_index_fetch_tuple` that guards against stale ctids
    /// referencing heap blocks truncated by VACUUM.
    ///
    /// The BM25 `ambulkdelete` correctly removes dead ctids from the index, but only
    /// when VACUUM actually runs. Between VACUUM cycles, the index may still contain
    /// ctids pointing to pages that a *previous* VACUUM truncated. The normal scan path
    /// (top-K) rarely hits these because it fetches few results, but the heap_filter
    /// path fetches ALL matching documents, making truncated-block hits likely.
    ///
    /// Returns `false` if the block has been truncated or the tuple is not visible.
    pub unsafe fn fetch_tuple(
        &self,
        ctid: &mut pg_sys::ItemPointerData,
        snapshot: pg_sys::Snapshot,
        call_again: &mut bool,
        all_dead: &mut bool,
    ) -> bool {
        let blockno = pgrx::itemptr::item_pointer_get_block_number(ctid);
        if blockno >= self.nblocks {
            return false;
        }
        pg_sys::table_index_fetch_tuple(
            self.scan,
            ctid,
            snapshot,
            self.slot(),
            call_again,
            all_dead,
        )
    }
}

crate::impl_safe_drop!(HeapFetchState, |self| {
    unsafe {
        if crate::postgres::utils::IsTransactionState() {
            pg_sys::ExecDropSingleTupleTableSlot(self.slot.cast());
            pg_sys::ExecDropSingleTupleTableSlot(self.virtual_slot);
            pg_sys::table_index_fetch_end(self.scan);
        }
    }
});

/// A wrapper for expression evaluation state.
#[derive(Debug)]
pub struct ExpressionState {
    econtext: *mut pg_sys::ExprContext,
    expr_states: Vec<*mut pg_sys::ExprState>,
}

impl ExpressionState {
    /// Create an ExpressionState for the given index relation.
    pub fn new(indexrel: &PgSearchRelation) -> Self {
        unsafe {
            Self::new_in_context(indexrel, &mut pgrx::PgMemoryContexts::TopTransactionContext)
        }
    }

    /// The memory context must outlive the returned expression state.
    pub unsafe fn new_in_context(
        indexrel: &PgSearchRelation,
        memory_context: &mut pgrx::PgMemoryContexts,
    ) -> Self {
        memory_context.switch_to(|_| {
            let index_exprs = pg_sys::RelationGetIndexExpressions(indexrel.as_ptr());
            let mut econtext = std::ptr::null_mut();
            let expr_states = if !index_exprs.is_null() {
                econtext = pg_sys::CreateStandaloneExprContext();
                let expr_list: PgList<pg_sys::Node> = PgList::from_pg(index_exprs);
                expr_list
                    .iter_ptr()
                    .map(|expr_node| pg_sys::ExecInitExpr(expr_node.cast(), std::ptr::null_mut()))
                    .collect()
            } else {
                vec![]
            };

            Self {
                econtext,
                expr_states,
            }
        })
    }

    /// Evaluate expressions for the tuple in the given slot.
    pub fn evaluate(&self, slot: *mut pg_sys::TupleTableSlot) -> Vec<(pg_sys::Datum, bool)> {
        self.evaluate_selected(slot, |_| true)
    }

    pub fn evaluate_selected(
        &self,
        slot: *mut pg_sys::TupleTableSlot,
        mut required: impl FnMut(usize) -> bool,
    ) -> Vec<(pg_sys::Datum, bool)> {
        let mut expr_results = Vec::new();
        if !self.econtext.is_null() {
            unsafe {
                (*self.econtext).ecxt_scantuple = slot;
            }
            for (index, expr_state) in self.expr_states.iter().enumerate() {
                if !required(index) {
                    expr_results.push((pg_sys::Datum::from(0), true));
                    continue;
                }
                let mut is_null = false;
                let datum =
                    unsafe { pg_sys::ExecEvalExpr(*expr_state, self.econtext, &mut is_null) };
                expr_results.push((datum, is_null));
            }
        }
        expr_results
    }
}

/// Rebuilds the index document for individual ctids by re-fetching their rows from the heap.
///
/// Bundles the per-relation state needed to turn one ctid into the `TantivyDocument` the index
/// holds for it. Fetches reuse the [`HeapFetchState`]'s buffer pins, so feeding ctids in heap
/// order keeps same-block fetches on a pinned buffer.
///
/// Query-visible mode (`query_visible: true`) fetches each ctid with the active MVCC snapshot
/// so that detoasting is safe: that registered snapshot holds back the global xmin horizon,
/// preventing a concurrent VACUUM from reclaiming the external TOAST chunks we read. Fetching
/// such rows with `SnapshotAny` could instead select DEAD / RECENTLY_DEAD versions whose TOAST
/// has already been (or is being) freed, raising spurious "missing/unexpected chunk number ...
/// in pg_toast_*" errors.
///
/// Maintenance mode (`query_visible: false`) must index every live ctid regardless of any
/// single snapshot, since the resulting segment may serve future snapshots, so it fetches with
/// `SnapshotAny` and filters dead tuples with `HeapTupleSatisfiesVacuum`, walking HOT chains
/// for a live member. See: <https://github.com/paradedb/paradedb/issues/5365>
pub struct HeapDocFetcher<'a> {
    fetch_state: &'a HeapFetchState,
    expression_state: &'a ExpressionState,
    heaprel: &'a PgSearchRelation,
    heaptupdesc: &'a PgTupleDesc<'a>,
    categorized_fields: &'a [(SearchField, CategorizedFieldData)],
    created_by_version: Option<Version>,
    oldest_xmin: pg_sys::TransactionId,
    query_visible: bool,
    root_ctids: bool,
    values: Vec<pg_sys::Datum>,
    isnull: Vec<bool>,
}

impl<'a> HeapDocFetcher<'a> {
    pub fn new(
        fetch_state: &'a HeapFetchState,
        expression_state: &'a ExpressionState,
        heaprel: &'a PgSearchRelation,
        heaptupdesc: &'a PgTupleDesc<'a>,
        categorized_fields: &'a [(SearchField, CategorizedFieldData)],
        created_by_version: Option<Version>,
        query_visible: bool,
    ) -> Self {
        let oldest_xmin = unsafe { pg_sys::GetOldestNonRemovableTransactionId(heaprel.as_ptr()) };
        Self {
            fetch_state,
            expression_state,
            heaprel,
            heaptupdesc,
            categorized_fields,
            created_by_version,
            oldest_xmin,
            query_visible,
            root_ctids: false,
            values: vec![pg_sys::Datum::null(); heaptupdesc.len()],
            isnull: vec![false; heaptupdesc.len()],
        }
    }

    /// The ctids this fetcher will receive are HOT chain roots from a table scan (what an index
    /// build callback hands over), not exact member ctids. In maintenance mode the fetch must
    /// then walk past superseded chain members to the live tail: the member whose values the
    /// inline build callback delivered for the root. Without this, a chain whose root is still
    /// RECENTLY_DEAD, or DELETE_IN_PROGRESS in this transaction, would have the superseded
    /// version's values indexed under the root ctid.
    ///
    /// Exact-ctid callers (rebuilding docs for ctids that an index entry points at) must NOT
    /// set this: with the index in place, HOT guarantees the chain members agree on indexed
    /// columns, and the first surviving member is the version the entry was made for.
    pub fn with_root_ctids(mut self) -> Self {
        self.root_ctids = true;
        self
    }

    /// Fetch `ctid` and build the document the index would hold for it, or `None` when there is
    /// nothing to index at `ctid`: its block was truncated by a previous VACUUM, the tuple is
    /// not visible to the active snapshot (query-visible mode), or every version in its HOT
    /// chain is dead (maintenance mode).
    pub unsafe fn fetch_doc(&mut self, ctid: u64) -> Option<TantivyDocument> {
        unsafe {
            // Guard against stale ctids referencing heap blocks truncated by VACUUM.
            if !utils::ctid_satisfies_nblocks(
                ctid,
                self.heaprel.as_ptr(),
                self.heaprel.fork_number(),
            ) {
                return None;
            }

            let mut ipd = pg_sys::ItemPointerData::default();
            utils::u64_to_item_pointer(ctid, &mut ipd);

            // See the struct docs for why the two modes fetch with different snapshots.
            let fetch_snapshot = if self.query_visible {
                pg_sys::GetActiveSnapshot()
            } else {
                &raw mut pg_sys::SnapshotAnyData
            };
            let mut call_again = false;
            'next_hot_chain: loop {
                let fetched = pg_sys::table_index_fetch_tuple(
                    self.fetch_state.scan,
                    &mut ipd,
                    fetch_snapshot,
                    self.fetch_state.slot(),
                    // call_again: This parameter will be set to true if this `ctid` points to multiple
                    // tuples as part of a HOT chain. We must attempt to find one live version of the
                    // tuple, and it may not be the first one in the chain.
                    &mut call_again,
                    // all_dead: Can hypothetically signal that a `ctid` is dead in all
                    // transactions: in practice, never actually seems to be anything but false
                    // when used with `SnapshotAnyData`.
                    &mut false,
                );

                if !fetched {
                    // Either the tuple is not visible to `fetch_snapshot` (query-visible mode) or
                    // heap page pruning removed it (SnapshotAny mode). In both cases there is no
                    // content to index for this ctid.
                    return None;
                }

                if self.query_visible {
                    // Visible to the snapshot: skip the SnapshotAny dead-tuple filtering below and
                    // index it.
                    break;
                }

                let (mut htsv_result, hot_updated) = {
                    let buffer = (*self.fetch_state.buffer_heap_slot()).buffer;
                    let _lock = BorrowedBuffer::from_pg(buffer);
                    let tuple = (*self.fetch_state.buffer_heap_slot()).base.tuple;
                    (
                        pg_sys::HeapTupleSatisfiesVacuum(tuple, self.oldest_xmin, buffer),
                        u32::from((*(*tuple).t_data).t_infomask2) & pg_sys::HEAP_HOT_UPDATED != 0,
                    )
                };

                if htsv_result == pg_sys::HTSV_Result::HEAPTUPLE_RECENTLY_DEAD {
                    // Our `oldest_xmin` might be stale compared to a concurrent VACUUM.
                    // If VACUUM saw this tuple as DEAD and deleted its TOAST chunks, we
                    // must also see it as DEAD, otherwise we'll crash trying to read them.
                    //
                    // A single re-check is sufficient (no loop needed) because
                    // `GetOldestNonRemovableTransactionId` returns the current global
                    // XID horizon. If the tuple is still RECENTLY_DEAD under this fresh
                    // horizon, then no concurrent VACUUM could have considered it DEAD
                    // (VACUUM uses the same or an older horizon), so its TOAST data is
                    // guaranteed to still exist.
                    let fresh_oldest_xmin =
                        pg_sys::GetOldestNonRemovableTransactionId(self.heaprel.as_ptr());
                    if fresh_oldest_xmin != self.oldest_xmin {
                        let buffer = (*self.fetch_state.buffer_heap_slot()).buffer;
                        let _lock = BorrowedBuffer::from_pg(buffer);
                        htsv_result = pg_sys::HeapTupleSatisfiesVacuum(
                            (*self.fetch_state.buffer_heap_slot()).base.tuple,
                            fresh_oldest_xmin,
                            buffer,
                        );
                    }
                }

                if htsv_result == pg_sys::HTSV_Result::HEAPTUPLE_DEAD {
                    // table_index_fetch_tuple stored this dead tuple in a buffer-backed slot. Since
                    // this branch skips the tuple, clear the slot before any HOT-chain retry or ctid
                    // skip so the slot releases its buffer pin.
                    pg_sys::ExecClearTuple(self.fetch_state.slot());

                    // This copy of the tuple is no longer visible to any transaction. Are there
                    // more in the HOT chain?
                    if call_again {
                        // There are more entries in the hot chain: find the first one that is
                        // visible.
                        continue 'next_hot_chain;
                    } else {
                        // There are no more entries in the HOT chain, so no copy of the tuple is
                        // visible in any transaction.
                        return None;
                    }
                }

                // The raw HOT bit is enough here. `HeapTupleHeaderIsHotUpdated` also wants a
                // valid xmax and xmin, and under the visibility check taken in the same locked
                // block an aborted updater reads as LIVE and an aborted inserter as DEAD, so
                // neither reaches this branch.
                if self.root_ctids
                    && hot_updated
                    && (htsv_result == pg_sys::HTSV_Result::HEAPTUPLE_RECENTLY_DEAD
                        || htsv_result == pg_sys::HTSV_Result::HEAPTUPLE_DELETE_IN_PROGRESS)
                {
                    // This member survives for old snapshots, but a newer HOT member carries
                    // the values the build callback delivered for this root. Skip forward so
                    // the segment holds the live version, matching what
                    // heapam_index_build_range_scan indexes for a broken HOT chain. A member
                    // that was deleted outright (not HOT-updated) is indexed below instead,
                    // as heapam does: a pre-existing snapshot may still need to see it. Under
                    // SnapshotAny `call_again` is set after every fetch, so it says nothing
                    // about whether the chain goes on; the tuple's own HOT flag does.
                    pg_sys::ExecClearTuple(self.fetch_state.slot());
                    continue 'next_hot_chain;
                }

                // We successfully fetched a tuple. Break out to fetch and deform it.
                break;
            }

            // We have a completely valid tuple to index: fetch and deform it.
            //
            // NOTE: We intentionally pass `false` (don't materialize) to keep the
            // buffer pin held by the BufferHeapTupleTableSlot. This pin blocks
            // VACUUM's LockBufferForCleanup, which prevents it from removing the
            // heap tuple and deleting its TOAST chunks while we read them below.
            // See: https://github.com/paradedb/paradedb/issues/5076
            let htup = pg_sys::ExecFetchSlotHeapTuple(
                self.fetch_state.slot(),
                false,
                std::ptr::null_mut(),
            );

            pg_sys::heap_deform_tuple(
                htup,
                self.heaptupdesc.as_ptr(),
                self.values.as_mut_ptr(),
                self.isnull.as_mut_ptr(),
            );

            // Eagerly detoast all variable-length (varlena) datums while the
            // buffer pin is still held. Without this, the lazy detoasting in
            // row_to_search_document can race with VACUUM deleting TOAST chunks
            // after we release the pin (the "missing chunk number 0" crash).
            // pg_detoast_datum is a no-op for already-inline / non-TOASTed data.
            for i in 0..self.heaptupdesc.len() {
                if !self.isnull[i] {
                    let att = self.heaptupdesc.get(i).expect("valid attribute");
                    if att.attlen == -1 {
                        self.values[i] = pg_sys::Datum::from(pg_sys::pg_detoast_datum(
                            self.values[i].cast_mut_ptr(),
                        ));
                    }
                }
            }

            let expr_results = self.expression_state.evaluate(self.fetch_state.slot());

            let mut doc = TantivyDocument::new();

            // Unpack all composites upfront from expr_results
            let unpacked_composites = CompositeSlotValues::from_composites(
                self.categorized_fields.iter().filter_map(|(_, cat)| {
                    if let FieldSource::CompositeField {
                        expression_idx,
                        composite_type_oid,
                        ..
                    } = cat.source
                    {
                        let (datum, is_null) = expr_results[expression_idx];
                        Some((expression_idx, datum, is_null, composite_type_oid))
                    } else {
                        None
                    }
                }),
            );

            utils::row_to_search_document(
                self.categorized_fields.iter().map(|(field, categorized)| {
                    let (datum, is_null) = utils::resolve_field_value(
                        &categorized.source,
                        &self.values,
                        &self.isnull,
                        &expr_results,
                        &unpacked_composites,
                    );
                    (datum, is_null, field, categorized)
                }),
                &mut doc,
                self.created_by_version,
            );

            // Eagerly release the buffer pin now that all datum values have
            // been detoasted into palloc'd memory. Without this, the pin would
            // stay held until the next table_index_fetch_tuple call (which
            // replaces the slot contents) or until HeapFetchState is dropped at
            // end-of-query, unnecessarily blocking VACUUM on this buffer.
            pg_sys::ExecClearTuple(self.fetch_state.slot());

            Some(doc)
        }
    }
}

/// Direct `extern "C"` bindings for Postgres functions that bypass the pgrx `pg_guard` wrapper.
///
/// Functions declared here MUST only be called when the caller has independently established
/// that the underlying C function will not `ereport`. The `pg_guard` wrapper exists to translate
/// Postgres `longjmp` into a Rust panic so destructors run; bypassing it on a code path that
/// could elog risks leaking pinned buffers, locks, and other RAII-tracked resources.
///
/// See `heap::VisibilityChecker::is_block_all_visible` for an example of a caller that
/// speculatively checks the C function's fast-path precondition before bypassing the wrapper.
mod util {
    use pgrx::pg_sys::{self, BlockNumber, Buffer, Relation};

    /// Mirrors `#define HEAPBLOCKS_PER_BYTE` from `src/backend/access/heap/visibilitymap.c`:
    /// `BITS_PER_BYTE / BITS_PER_HEAPBLOCK`. Number of heap blocks represented in one byte.
    pub const HEAPBLOCKS_PER_BYTE: u32 = 8 / pg_sys::BITS_PER_HEAPBLOCK;

    /// Number of usable bitmap bytes on a VM page, mirroring `#define MAPSIZE` from
    /// `src/backend/access/heap/visibilitymap.c`: `BLCKSZ - MAXALIGN(SizeOfPageHeaderData)`.
    /// The page header is NOT available for the bitmap, so this is smaller than `BLCKSZ`.
    const MAPSIZE: u32 = {
        // SizeOfPageHeaderData == offsetof(PageHeaderData, pd_linp); see pgrx `SizeOfPageHeaderData`.
        let header =
            unsafe { pg_sys::MAXALIGN(std::mem::offset_of!(pg_sys::PageHeaderData, pd_linp)) };
        pg_sys::BLCKSZ - header as u32
    };

    /// Mirrors `#define HEAPBLOCKS_PER_PAGE` from `src/backend/access/heap/visibilitymap.c`:
    /// `MAPSIZE * HEAPBLOCKS_PER_BYTE`. Number of heap blocks covered by one VM page
    /// (~32672 for a standard 8KB build).
    ///
    /// This MUST equal Postgres's value: it is the divisor for the VM-buffer cache slot index in
    /// [`crate::postgres::heap::VisibilityChecker::is_block_all_visible`]. If it disagrees with
    /// Postgres's internal `HEAPBLK_TO_MAPBLOCK`, our slot index points at the wrong VM page near
    /// every page boundary, and the unguarded fast path forces a `vm_readbuf` (a safety-contract
    /// violation that also thrashes the VM cache).
    pub const HEAPBLOCKS_PER_PAGE: u32 = MAPSIZE * HEAPBLOCKS_PER_BYTE;

    unsafe extern "C" {
        /// Raw binding to Postgres `visibilitymap_get_status`. Safe to call without the
        /// pgrx wrapper ONLY when the caller has confirmed `*buf` is valid and already
        /// holds the correct mapBlock for `heapBlk` — i.e. the C function will take its
        /// fast bit-math branch and will not invoke `vm_readbuf`.
        pub fn visibilitymap_get_status(
            rel: Relation,
            heapBlk: BlockNumber,
            buf: *mut Buffer,
        ) -> u8;
    }
}

/// Streams ctids in the order given and prefetches their heap blocks `distance` blocks ahead,
/// so that a block miss overlaps with the work on the blocks before it rather than stalling on
/// it. The ctids are expected in heap order: a block's rows then arrive together and the block
/// is prefetched once, when its first row enters the window.
pub struct PrefetchWindow<'a, I: Iterator<Item = u64>> {
    heaprel: &'a PgSearchRelation,
    source: I,
    window: VecDeque<u64>,
    distance: usize,
    /// Distinct blocks among the ctids in `window`.
    blocks_in_window: usize,
}

impl<'a, I: Iterator<Item = u64>> PrefetchWindow<'a, I> {
    /// `distance` is in blocks; `maintenance_io_concurrency` is the usual choice for a build.
    pub fn new(heaprel: &'a PgSearchRelation, source: I, distance: usize) -> Self {
        Self {
            heaprel,
            source,
            window: VecDeque::new(),
            distance,
            blocks_in_window: 0,
        }
    }
}

impl<I: Iterator<Item = u64>> Iterator for PrefetchWindow<'_, I> {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        // Keep the block being read plus `distance` more in the window.
        while self.blocks_in_window <= self.distance {
            let Some(ctid) = self.source.next() else {
                break;
            };
            let block = utils::u64_ctid_block_number(ctid);
            if self
                .window
                .back()
                .map(|&last| utils::u64_ctid_block_number(last))
                != Some(block)
            {
                self.blocks_in_window += 1;
                if self.distance > 0 {
                    unsafe {
                        pg_sys::PrefetchBuffer(
                            self.heaprel.as_ptr(),
                            pg_sys::ForkNumber::MAIN_FORKNUM,
                            block,
                        );
                    }
                }
            }
            self.window.push_back(ctid);
        }
        let ctid = self.window.pop_front()?;
        if self
            .window
            .front()
            .map(|&next| utils::u64_ctid_block_number(next))
            != Some(utils::u64_ctid_block_number(ctid))
        {
            self.blocks_in_window -= 1;
        }
        Some(ctid)
    }
}

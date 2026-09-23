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
use std::ops::Deref;
use std::sync::Arc;

use crate::api::version::Version;
use crate::api::{CTID_FIELD_NAME, TID_BLOCK_FIELD_NAME};
use crate::index::fast_fields_helper::{FFHelper, TidReader};
use crate::postgres::composite::CompositeSlotValues;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::buffer::{BorrowedBuffer, BufferManager, PinnedBuffer};
use crate::postgres::utils::{self, TidBlock, TidOffset};
use crate::schema::{CategorizedFieldData, FieldSource, SearchField};
use parking_lot::Mutex;
use pgrx::pg_sys;
use pgrx::{PgList, PgTupleDesc, check_for_interrupts};
use tantivy::SegmentReader;
use tantivy::columnar::Cardinality;
use tantivy::index::SegmentId;
use tantivy::{DocId, SegmentOrdinal, TantivyDocument};

use util::HEAPBLOCKS_PER_BYTE;
use util::HEAPBLOCKS_PER_PAGE as HEAPBLOCKS_PER_VM_PAGE;

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
    pub(crate) unsafe fn read(heaprel: &PgSearchRelation, blockno: pg_sys::BlockNumber) -> Self {
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

/// Target buffer for batch visibility checking.
enum VisibilityTarget<'a> {
    /// Boolean mask indicating visibility for each document.
    Mask(&'a mut [bool]),
    /// Visible CTIDs (or None if invisible) for each document.
    Ctids(&'a mut [Option<u64>]),
}

impl<'a> VisibilityTarget<'a> {
    #[inline(always)]
    fn len(&self) -> usize {
        match self {
            Self::Mask(m) => m.len(),
            Self::Ctids(c) => c.len(),
        }
    }

    #[inline(always)]
    fn reset(&mut self) {
        match self {
            Self::Mask(m) => m.fill(false),
            Self::Ctids(c) => c.fill(None),
        }
    }

    #[inline(always)]
    fn set(&mut self, idx: usize, resolved_ctid: Option<u64>) {
        match self {
            Self::Mask(m) => m[idx] = resolved_ctid.is_some(),
            Self::Ctids(c) => c[idx] = resolved_ctid,
        }
    }
}

/// Reusable scratch vectors for document tuple identifier decoding and index classification.
#[derive(Default)]
struct TidScratch {
    blocks: Vec<TidBlock>,
    legacy_ctids: Vec<Option<u64>>,
    offsets: Vec<Option<u64>>,
    missed_doc_ids: Vec<DocId>,
    missed_offsets: Vec<Option<u64>>,
    missed: Vec<usize>,
}

/// A batch accessor for document tuple identifiers, wrapping scratch buffers and binding
/// the segment reader and document IDs.
struct TidBatch<'a> {
    reader: &'a TidReader,
    docs: &'a [DocId],
    scratch: &'a mut TidScratch,
    is_split: bool,
    has_read_all_offsets: bool,
}

impl<'a> TidBatch<'a> {
    fn new(reader: &'a TidReader, docs: &'a [DocId], scratch: &'a mut TidScratch) -> Self {
        debug_assert!(
            docs.windows(2).all(|w| w[0] <= w[1]),
            "docs passed to TidBatch must be sorted"
        );
        scratch.offsets.clear();
        scratch.missed.clear();

        match reader {
            TidReader::Legacy(col) => {
                scratch.legacy_ctids.resize(docs.len(), None);
                col.first_vals(docs, scratch.legacy_ctids.as_mut_slice());
            }
            TidReader::Split { block, .. } => {
                scratch.blocks.resize(docs.len(), 0);
                block.u32_vals(docs, scratch.blocks.as_mut_slice());
            }
        }

        Self {
            reader,
            docs,
            scratch,
            is_split: matches!(reader, TidReader::Split { .. }),
            has_read_all_offsets: false,
        }
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.docs.len()
    }

    /// Returns the block number for the document at index `idx` within the batch.
    #[inline(always)]
    fn block(&self, idx: usize) -> Option<TidBlock> {
        if self.is_split {
            self.scratch.blocks.get(idx).copied()
        } else {
            let raw = self.scratch.legacy_ctids.get(idx).copied().flatten()?;
            Some((raw >> 16) as TidBlock)
        }
    }

    /// Finds the end index (exclusive) of the contiguous run of documents sharing the same
    /// block number as `start`.
    #[inline(always)]
    fn block_run_end(&self, start: usize) -> usize {
        let len = self.len();
        if self.is_split {
            let blocks = &self.scratch.blocks[..len];
            let target_block = blocks[start];
            let mut end = start + 1;
            while end < blocks.len() && blocks[end] == target_block {
                end += 1;
            }
            end
        } else {
            let target_block = self.block(start);
            let mut end = start + 1;
            while end < len && self.block(end) == target_block {
                end += 1;
            }
            end
        }
    }

    /// Returns the offset number for the document at index `idx` within the batch.
    #[inline(always)]
    fn offset(&self, idx: usize) -> Option<TidOffset> {
        if self.is_split {
            let raw = self.scratch.offsets.get(idx).copied().flatten()?;
            Some(raw as TidOffset)
        } else {
            let raw = self.scratch.legacy_ctids.get(idx).copied().flatten()?;
            Some((raw & 0xFFFF) as TidOffset)
        }
    }

    /// Returns the packed 64-bit CTID for the document at index `idx` within the batch.
    #[inline(always)]
    fn ctid(&self, idx: usize) -> Option<u64> {
        if self.is_split {
            let b = self.block(idx)?;
            let o = self.offset(idx)?;
            Some(crate::postgres::utils::tid_from_components(b, o))
        } else {
            self.scratch.legacy_ctids.get(idx).copied().flatten()
        }
    }

    /// Returns the block number and packed CTID for the document at index `idx`.
    #[inline(always)]
    fn block_and_ctid(&self, idx: usize) -> (TidBlock, u64) {
        if self.is_split {
            let blockno = self.scratch.blocks[idx];
            let offset = self.offset(idx).expect("Document must have offset");
            let ctid = crate::postgres::utils::tid_from_components(blockno, offset);
            (blockno, ctid)
        } else {
            let raw = self
                .scratch
                .legacy_ctids
                .get(idx)
                .copied()
                .flatten()
                .expect("Document must have ctid");
            ((raw >> 16) as TidBlock, raw)
        }
    }

    #[inline(always)]
    fn record_miss_range(&mut self, range: std::ops::Range<usize>) {
        self.scratch.missed.extend(range);
    }

    #[inline(always)]
    fn missed(&self) -> &[usize] {
        &self.scratch.missed
    }

    /// Reads offsets exclusively for documents recorded as missed.
    ///
    /// For split columns, decodes `tid_offset` for `missed` indices.
    /// For legacy columns, this is a no-op as offsets are already in `blocks`.
    fn read_missed_offsets(&mut self) {
        if !self.is_split || self.has_read_all_offsets || self.scratch.missed.is_empty() {
            return;
        }
        let TidReader::Split { offset, .. } = self.reader else {
            unreachable!()
        };
        if self.scratch.offsets.len() < self.docs.len() {
            self.scratch.offsets.resize(self.docs.len(), None);
        }
        self.scratch.missed_doc_ids.clear();
        self.scratch
            .missed_doc_ids
            .extend(self.scratch.missed.iter().map(|&i| self.docs[i]));

        self.scratch
            .missed_offsets
            .resize(self.scratch.missed.len(), None);
        offset.first_vals(
            &self.scratch.missed_doc_ids,
            self.scratch.missed_offsets.as_mut_slice(),
        );

        for (&idx, &opt_o) in self
            .scratch
            .missed
            .iter()
            .zip(self.scratch.missed_offsets.iter())
        {
            self.scratch.offsets[idx] = opt_o;
        }
    }

    /// Sorts missed document indices by block number to optimize buffer lock reuse.
    #[inline(always)]
    fn sort_missed_by_block(&mut self) {
        if self.is_split {
            let blocks = &self.scratch.blocks;
            self.scratch.missed.sort_unstable_by_key(|&i| blocks[i]);
        } else {
            let legacy_ctids = &self.scratch.legacy_ctids;
            self.scratch.missed.sort_unstable_by_key(|&i| {
                let raw = legacy_ctids.get(i).copied().flatten()?;
                Some((raw >> 16) as TidBlock)
            });
        }
    }

    /// Reads offsets for all documents in the batch.
    ///
    /// For split columns, this decodes `tid_offset` for the entire batch.
    /// For legacy columns, this is a no-op as offsets are already in `blocks`.
    fn read_all_offsets(&mut self) {
        if !self.is_split || self.has_read_all_offsets {
            return;
        }
        let TidReader::Split { offset, .. } = self.reader else {
            unreachable!()
        };
        self.scratch.offsets.resize(self.docs.len(), None);
        offset.first_vals(self.docs, self.scratch.offsets.as_mut_slice());
        self.has_read_all_offsets = true;
    }
}

/// Helper to validate that a "ctid" is currently visible to a snapshot.
///
/// When querying ParadeDB indexes, individual ctid entries may be stale. After an UPDATE,
/// the old tuple is marked dead and a new tuple is created at a new ctid, but the
/// index still has the old ctid until VACUUM runs.
///
/// The visibility checker supports three operational modes:
/// 1. Mask-only visibility checking ([`VisibilityChecker::check_segment_docs_mask`]):
///    Checks the PostgreSQL visibility map first and returns a boolean mask. For split columns,
///    never reads or decodes `tid_offset` for docs on all-visible blocks.
/// 2. Fast-path visibility confirmation ([`VisibilityChecker::check_segment_docs`]):
///    Checks the PostgreSQL visibility map first. On all-visible blocks, visibility is
///    guaranteed for all active snapshots, so heap page access is bypassed entirely.
///    The returned CTID for an all-visible block is the raw index CTID, which may be an index root pointing to
///    a HOT redirect (`LP_REDIRECT`). This is safe and optimal for execution plan nodes
///    like `VisibilityFilterExec` and `BatchScanner` whose downstream tuple fetcher
///    (e.g. `JoinScanState::build_result_tuple` or `BaseScan`) uses `table_index_fetch_tuple`
///    to resolve the HOT redirect to the physical tuple at final output time.
/// 3. Full physical HOT resolution ([`VisibilityChecker::resolve_segment_docs`]):
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

    vm_block_no: Option<pg_sys::BlockNumber>,
    vmbuff: pg_sys::Buffer,
    vm_page_ptr: *const u8,
    // tracks our previous block visibility so we can elide checking again
    blockvis: (pg_sys::BlockNumber, bool),

    /// Cached relation size (in blocks) at scan start. Used to cheaply skip
    /// stale ctids pointing to pages truncated by a previous VACUUM.
    nblocks: pg_sys::BlockNumber,

    /// Pin on the heap block last checked by `resolve_visible`, held across
    /// calls since consecutive checks tend to hit the same block.
    cached_heap_block: pg_sys::BlockNumber,
    cached_heap_pin: Option<PinnedBuffer>,

    pub heap_tuple_check_count: usize,
    pub invisible_tuple_count: usize,

    /// False for a `visibility => 'raw'` aggregate, which trades snapshot
    /// accuracy for skipping the heap: every ctid then passes as-is.
    check_visibility: bool,

    // TODO: Make this non-optional in the future once all call sites provide an FFHelper.
    ffhelper: Option<Arc<FFHelper>>,
    tid_scratch: TidScratch,
    segment_visibility: Option<(SegmentId, bool)>,
}

// TODO: Use of clone results in new metrics in the clone. Should put them in `Rc<RefCell<usize>>`.
impl Clone for VisibilityChecker {
    fn clone(&self) -> Self {
        let mut checker = Self::with_rel_and_snap(&self.heaprel, self.snapshot);
        checker.check_visibility = self.check_visibility;
        checker.ffhelper = self.ffhelper.clone();
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
                vm_page_ptr: std::ptr::null(),
                blockvis: (pg_sys::InvalidBlockNumber, false),
                nblocks,
                cached_heap_block: pg_sys::InvalidBlockNumber,
                cached_heap_pin: None,
                heap_tuple_check_count: 0,
                invisible_tuple_count: 0,
                check_visibility: true,
                ffhelper: None,
                tid_scratch: TidScratch::default(),
                segment_visibility: None,
            }
        }
    }

    /// Attaches an [`FFHelper`] for resolving segment `DocId`s to ctids directly.
    pub fn with_ffhelper(mut self, ffhelper: Arc<FFHelper>) -> Self {
        self.ffhelper = Some(ffhelper);
        self.segment_visibility = None;
        self
    }

    pub fn set_ffhelper(&mut self, ffhelper: Arc<FFHelper>) {
        self.ffhelper = Some(ffhelper);
        self.segment_visibility = None;
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
        let blockno = (ctid >> 16) as pg_sys::BlockNumber;
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
            let blockno = (ctid >> 16) as pg_sys::BlockNumber;
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
    #[inline]
    pub fn is_block_all_visible(&mut self, blockno: pg_sys::BlockNumber) -> bool {
        if blockno == self.blockvis.0 {
            return self.blockvis.1;
        }
        self.blockvis.0 = blockno;

        let vm_block_no = blockno / util::HEAPBLOCKS_PER_PAGE;
        unsafe {
            let is_all_visible = if Some(vm_block_no) == self.vm_block_no
                && !self.vm_page_ptr.is_null()
            {
                debug_assert_eq!(
                    pg_sys::BufferGetBlockNumber(self.vmbuff),
                    vm_block_no,
                    "pinned vmbuff does not cover the expected VM mapBlock"
                );
                // Fast path: bit test directly on the pinned VM page in Rust memory.
                // Avoids FFI function call overhead completely.
                let map_byte =
                    ((blockno % util::HEAPBLOCKS_PER_PAGE) / util::HEAPBLOCKS_PER_BYTE) as usize;
                let map_offset = (blockno % util::HEAPBLOCKS_PER_BYTE) * pg_sys::BITS_PER_HEAPBLOCK;
                let byte = *self.vm_page_ptr.add(map_byte);
                ((byte >> map_offset) & (pg_sys::VISIBILITYMAP_ALL_VISIBLE as u8)) != 0
            } else {
                // Slow path: either we have no pinned VM page yet, or `blockno` crossed a
                // VM-page boundary. The C function may release the old buffer and call
                // `vm_readbuf` (which can `ereport`), so we MUST go through the guarded
                // wrapper. This also (re)pins `vmbuff` to the correct mapBlock so the
                // fast path can be taken on subsequent calls.
                let status = pg_sys::visibilitymap_get_status(
                    self.heaprel.as_ptr(),
                    blockno,
                    &mut self.vmbuff,
                );
                if self.vmbuff != pg_sys::InvalidBuffer as pg_sys::Buffer {
                    self.vm_block_no = Some(vm_block_no);
                    let page = pg_sys::BufferGetPage(self.vmbuff);
                    self.vm_page_ptr = (page as *const u8).add(util::PAGE_HEADER_OFFSET);
                } else {
                    self.vm_block_no = None;
                    self.vm_page_ptr = std::ptr::null();
                }
                (status & (pg_sys::VISIBILITYMAP_ALL_VISIBLE as u8)) != 0
            };

            self.blockvis.1 = is_all_visible;
        }
        self.blockvis.1
    }

    pub(crate) fn for_segment(
        checker: &Arc<Mutex<Self>>,
        segment: &SegmentReader,
    ) -> tantivy::Result<Option<Arc<Mutex<Self>>>> {
        Ok((!checker.lock().is_segment_all_visible(segment)?).then(|| checker.clone()))
    }

    pub(crate) fn is_segment_all_visible(
        &mut self,
        segment: &SegmentReader,
    ) -> tantivy::Result<bool> {
        if let Some((id, visible)) = self.segment_visibility
            && id == segment.segment_id()
        {
            return Ok(visible);
        }
        // prove segment is all visible
        let visible = 'proof: {
            if self.snapshot.is_null()
                || unsafe { (*self.snapshot).snapshot_type != pg_sys::SnapshotType::SNAPSHOT_MVCC }
                || !self
                    .ffhelper
                    .as_ref()
                    .is_some_and(|helper| helper.is_immutable_segment(segment.segment_id()))
                || segment.num_docs() == 0
            {
                break 'proof false;
            }
            let (first, last) = if segment.schema().get_field(TID_BLOCK_FIELD_NAME).is_ok() {
                let blocks = segment.fast_fields().u64(TID_BLOCK_FIELD_NAME)?;
                if blocks.get_cardinality() != Cardinality::Full
                    || blocks.num_docs() != segment.max_doc()
                {
                    break 'proof false;
                }
                let (Ok(first), Ok(last)) = (
                    u32::try_from(blocks.min_value()),
                    u32::try_from(blocks.max_value()),
                ) else {
                    break 'proof false;
                };
                (first, last)
            } else {
                let ctids = segment.fast_fields().u64(CTID_FIELD_NAME)?;
                if ctids.get_cardinality() != Cardinality::Full
                    || ctids.num_docs() != segment.max_doc()
                {
                    break 'proof false;
                }
                let (Ok(first), Ok(last)) = (
                    u32::try_from(ctids.min_value() >> 16),
                    u32::try_from(ctids.max_value() >> 16),
                ) else {
                    break 'proof false;
                };
                (first, last)
            };
            let vm_pages = last / HEAPBLOCKS_PER_VM_PAGE - first / HEAPBLOCKS_PER_VM_PAGE + 1;
            if vm_pages > 64 {
                break 'proof false;
            }
            // Read CTID bounds before fresh VM bits; FFHelper retains the VACUUM cleanup pin.
            self.is_range_all_visible(first, last)
        };
        self.segment_visibility = Some((segment.segment_id(), visible));
        Ok(visible)
    }

    fn is_range_all_visible(
        &mut self,
        first: pg_sys::BlockNumber,
        last: pg_sys::BlockNumber,
    ) -> bool {
        if first > last || last >= self.nblocks {
            return false;
        }
        const VISIBLE_MASK: u8 = (u8::MAX as u32 / ((1 << pg_sys::BITS_PER_HEAPBLOCK) - 1)
            * pg_sys::VISIBILITYMAP_ALL_VISIBLE) as u8;
        self.blockvis = (pg_sys::InvalidBlockNumber, false);
        let mut block = first;
        while block <= last {
            check_for_interrupts!();
            if !self.is_block_all_visible(block) {
                return false;
            }
            if !block.is_multiple_of(HEAPBLOCKS_PER_BYTE) || last - block < HEAPBLOCKS_PER_BYTE - 1
            {
                block += 1;
                continue;
            }
            let local_block = block % HEAPBLOCKS_PER_VM_PAGE;
            let bytes =
                (last - block + 1).min(HEAPBLOCKS_PER_VM_PAGE - local_block) / HEAPBLOCKS_PER_BYTE;
            // is_block_all_visible pins the VM page; only scan complete bytes in our range.
            unsafe {
                let map = pg_sys::PageGetContents(pg_sys::BufferGetPage(self.vmbuff))
                    .cast::<u8>()
                    .add((local_block / HEAPBLOCKS_PER_BYTE) as usize);
                for byte in 0..bytes as usize {
                    if map.add(byte).read_volatile() & VISIBLE_MASK != VISIBLE_MASK {
                        return false;
                    }
                }
            }
            block += bytes * HEAPBLOCKS_PER_BYTE;
        }
        true
    }

    /// Single-ctid visibility check for callers probing one doc at a time
    /// (e.g. the cardinality fast path's visibility filter).
    pub fn check_one(&mut self, ctid: u64) -> bool {
        !self.check_visibility || self.resolve_visible(ctid, None, false).is_some()
    }

    /// Checks visibility of documents within a segment and populates a boolean visibility mask.
    ///
    /// # Preconditions
    ///
    /// `doc_ids` must be sorted in ascending order:
    /// `doc_ids.windows(2).all(|w| w[0] <= w[1])`.
    ///
    /// For all-visible blocks, visibility is confirmed via the visibility map fast-path without
    /// reading heap buffers. For split columns, this never reads or decodes `tid_offset` for
    /// docs on all-visible blocks.
    pub fn check_segment_docs_mask(
        &mut self,
        segment_ord: SegmentOrdinal,
        doc_ids: &[DocId],
        mask: &mut [bool],
    ) {
        self.check_segment_docs_inner(segment_ord, doc_ids, VisibilityTarget::Mask(mask), false);
    }

    /// Checks if a slice of `DocId`s within a segment are visible, fetching ctids directly from
    /// the configured [`FFHelper`].
    ///
    /// # Preconditions
    ///
    /// `doc_ids` must be sorted in ascending order:
    /// `doc_ids.windows(2).all(|w| w[0] <= w[1])`.
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
        self.check_segment_docs_inner(
            segment_ord,
            doc_ids,
            VisibilityTarget::Ctids(results),
            false,
        );
    }

    /// Checks if a slice of `DocId`s within a segment are visible and resolves each CTID to the
    /// physical HOT member visible under this checker's snapshot, fetching ctids directly from
    /// the configured [`FFHelper`].
    ///
    /// # Preconditions
    ///
    /// `doc_ids` must be sorted in ascending order:
    /// `doc_ids.windows(2).all(|w| w[0] <= w[1])`.
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
        self.check_segment_docs_inner(segment_ord, doc_ids, VisibilityTarget::Ctids(results), true);
    }

    fn check_segment_docs_inner(
        &mut self,
        segment_ord: SegmentOrdinal,
        doc_ids: &[DocId],
        mut target: VisibilityTarget<'_>,
        resolve_hot: bool,
    ) {
        assert_eq!(doc_ids.len(), target.len());
        if doc_ids.is_empty() {
            return;
        }
        debug_assert!(
            doc_ids.windows(2).all(|w| w[0] <= w[1]),
            "doc_ids must be sorted"
        );
        target.reset();

        let ffhelper = self
            .ffhelper
            .clone()
            .expect("FFHelper must be configured to check segment doc visibility");

        let reader = ffhelper.ctid(segment_ord);
        let mut scratch = std::mem::take(&mut self.tid_scratch);
        let mut batch = TidBatch::new(reader, doc_ids, &mut scratch);

        let len = batch.len();
        let mut start = 0;

        while start < len {
            let end = batch.block_run_end(start);
            let Some(blockno) = batch.block(start) else {
                start = end;
                continue;
            };

            if blockno >= self.nblocks {
                self.invisible_tuple_count += end - start;
                start = end;
                continue;
            }

            if !self.check_visibility || (!resolve_hot && self.is_block_all_visible(blockno)) {
                if let VisibilityTarget::Mask(mask) = &mut target {
                    mask[start..end].fill(true);
                }
            } else {
                batch.record_miss_range(start..end);
            }

            start = end;
        }

        // For CTID target: populate results for all valid documents initially.
        // Missed documents will be resolved and overwritten below.
        if let VisibilityTarget::Ctids(results) = &mut target {
            batch.read_all_offsets();
            for i in 0..len {
                if let Some(blockno) = batch.block(i)
                    && blockno < self.nblocks
                {
                    results[i] = batch.ctid(i);
                }
            }
        }

        // If there were any misses, resolve them against heap buffers.
        if !batch.missed().is_empty() {
            batch.read_missed_offsets();
            batch.sort_missed_by_block();

            let mut current_buffer: Option<crate::postgres::storage::buffer::Buffer> = None;
            let mut current_block = pg_sys::InvalidBlockNumber;

            for &i in batch.missed() {
                let (blockno, raw_ctid) = batch.block_and_ctid(i);

                let locked_buffer = if current_block != blockno {
                    drop(current_buffer.take());
                    current_buffer = Some(self.bman.get_buffer(blockno));
                    current_block = blockno;
                    *current_buffer.as_ref().unwrap().deref()
                } else {
                    *current_buffer.as_ref().unwrap().deref()
                };

                let resolved = self.resolve_visible(raw_ctid, Some(locked_buffer), resolve_hot);
                target.set(i, resolved);
            }
        }

        self.tid_scratch = scratch;
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
        let blockno = (ctid >> 16) as pg_sys::BlockNumber;
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
    nblocks: pg_sys::BlockNumber,
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
    use pgrx::pg_sys;

    /// Mirrors `#define HEAPBLOCKS_PER_BYTE` from `src/backend/access/heap/visibilitymap.c`:
    /// `BITS_PER_BYTE / BITS_PER_HEAPBLOCK`. Number of heap blocks represented in one byte.
    pub const HEAPBLOCKS_PER_BYTE: u32 = 8 / pg_sys::BITS_PER_HEAPBLOCK;

    /// Offset of usable bitmap data on a VM page, past the standard page header.
    /// SizeOfPageHeaderData == offsetof(PageHeaderData, pd_linp); see pgrx `SizeOfPageHeaderData`.
    pub const PAGE_HEADER_OFFSET: usize =
        unsafe { pg_sys::MAXALIGN(std::mem::offset_of!(pg_sys::PageHeaderData, pd_linp)) };

    /// Number of usable bitmap bytes on a VM page, mirroring `#define MAPSIZE` from
    /// `src/backend/access/heap/visibilitymap.c`: `BLCKSZ - MAXALIGN(SizeOfPageHeaderData)`.
    /// The page header is NOT available for the bitmap, so this is smaller than `BLCKSZ`.
    const MAPSIZE: u32 = pg_sys::BLCKSZ - PAGE_HEADER_OFFSET as u32;

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

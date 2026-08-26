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

use std::ops::Deref;

use crate::api::version::Version;
use crate::postgres::composite::CompositeSlotValues;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::buffer::{BorrowedBuffer, BufferManager, PinnedBuffer};
use crate::postgres::utils;
use crate::schema::{CategorizedFieldData, FieldSource, SearchField};
use pgrx::pg_sys;
use pgrx::{PgList, PgTupleDesc};
use tantivy::TantivyDocument;

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

/// Helper to validate that a "ctid" is currently visible to a snapshot.
///
/// When querying BM25 indexes, individual ctid entries may be stale. After an UPDATE,
/// the old tuple is marked dead and a new tuple is created at a new ctid, but the
/// index still has the old ctid until VACUUM runs.
///
/// The visibility checker uses `table_index_fetch_tuple` which:
/// - Verifies the tuple exists and is visible (following HOT chains if needed)
/// - Returns the tuple at its current heap location
pub struct VisibilityChecker {
    scan: *mut pg_sys::IndexFetchTableData,
    snapshot: pg_sys::Snapshot,
    tid: pg_sys::ItemPointerData,

    // we hold onto this b/c `scan` points to the relation this does
    heaprel: PgSearchRelation,
    bman: BufferManager,

    vm_block_no: Option<pg_sys::BlockNumber>,
    vmbuff: pg_sys::Buffer,
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
}

// TODO: Use of clone results in new metrics in the clone. Should put them in `Rc<RefCell<usize>>`.
impl Clone for VisibilityChecker {
    fn clone(&self) -> Self {
        Self::with_rel_and_snap(&self.heaprel, self.snapshot)
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
            }
        }
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
        unsafe {
            let blockno = (ctid >> 16) as pg_sys::BlockNumber;
            if blockno >= self.nblocks {
                self.invisible_tuple_count += 1;
                return None;
            }
            self.heap_tuple_check_count += 1;

            utils::u64_to_item_pointer(ctid, &mut self.tid);

            let mut call_again = false;
            let mut all_dead = false;
            let found = pg_sys::table_index_fetch_tuple(
                self.scan,
                &mut self.tid,
                self.snapshot,
                slot,
                &mut call_again,
                &mut all_dead,
            );

            if found {
                Some(func((*self.scan).rel))
            } else {
                self.invisible_tuple_count += 1;
                None
            }
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
    pub fn is_block_all_visible(&mut self, blockno: pg_sys::BlockNumber) -> bool {
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

    /// Single-ctid visibility check for callers probing one doc at a time
    /// (e.g. the cardinality fast path's visibility filter).
    pub fn check_one(&mut self, ctid: u64) -> bool {
        self.resolve_visible(ctid, None).is_some()
    }

    /// Checks if a batch of rows are visible, and computes their updated ctid (by following a HOT
    /// chain) if so. Panics if any ctids are absent.
    ///
    /// See `check` for details on visibility checking logic.
    pub fn check_batch(&mut self, ctids: &[Option<u64>], results: &mut [Option<u64>]) {
        if ctids.is_empty() {
            return;
        }
        assert_eq!(ctids.len(), results.len());

        let mut sorted_indices: Vec<(usize, u64)> = ctids
            .iter()
            .map(|maybe_ctid| maybe_ctid.expect("All rows must have ctids."))
            .enumerate()
            .collect();
        sorted_indices.sort_unstable_by_key(|(_, ctid)| *ctid);

        let mut current_buffer: Option<crate::postgres::storage::buffer::Buffer> = None;
        let mut current_block = pg_sys::InvalidBlockNumber;

        for (idx, ctid) in sorted_indices {
            let blockno = (ctid >> 16) as pg_sys::BlockNumber;
            // acquire the block's buffer once per run of same-block ctids and
            // hold its lock across the run; resolve_visible's own VM re-check
            // hits the blockvis cache
            let needs_heap_check = blockno < self.nblocks && !self.is_block_all_visible(blockno);
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
            results[idx] = self.resolve_visible(ctid, locked_buffer);
        }
    }

    /// Resolves a ctid to its visible ctid under the checker's snapshot,
    /// following any HOT chain; `None` if invisible or stale. A caller
    /// already holding a pin and share lock on the ctid's block passes the
    /// buffer; otherwise the tuple is checked under a short share lock,
    /// reusing a cached pin on the heap block across calls since consecutive
    /// checks tend to hit the same block.
    fn resolve_visible(&mut self, ctid: u64, locked_buffer: Option<pg_sys::Buffer>) -> Option<u64> {
        let blockno = (ctid >> 16) as pg_sys::BlockNumber;
        if blockno >= self.nblocks {
            self.invisible_tuple_count += 1;
            return None;
        }
        if self.is_block_all_visible(blockno) {
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
        let index_exprs = unsafe { pg_sys::RelationGetIndexExpressions(indexrel.as_ptr()) };
        let mut econtext: *mut pg_sys::ExprContext = std::ptr::null_mut();
        let expr_states = if !index_exprs.is_null() {
            econtext = unsafe {
                pgrx::PgMemoryContexts::TopTransactionContext
                    .switch_to(|_| pg_sys::CreateStandaloneExprContext())
            };
            let expr_list: PgList<pg_sys::Node> = unsafe { PgList::from_pg(index_exprs) };

            let old_context =
                unsafe { pg_sys::MemoryContextSwitchTo((*econtext).ecxt_per_query_memory) };

            let states = expr_list
                .iter_ptr()
                .map(|expr_node| unsafe {
                    pg_sys::ExecInitExpr(expr_node.cast(), std::ptr::null_mut())
                })
                .collect::<Vec<_>>();

            unsafe { pg_sys::MemoryContextSwitchTo(old_context) };
            states
        } else {
            vec![]
        };

        Self {
            econtext,
            expr_states,
        }
    }

    /// Evaluate expressions for the tuple in the given slot.
    pub fn evaluate(&self, slot: *mut pg_sys::TupleTableSlot) -> Vec<(pg_sys::Datum, bool)> {
        let mut expr_results = Vec::new();
        if !self.econtext.is_null() {
            unsafe {
                (*self.econtext).ecxt_scantuple = slot;
            }
            for expr_state in &self.expr_states {
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
            )
            .unwrap_or_else(|e| {
                panic!("Failed to create document from row: {e}");
            });

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

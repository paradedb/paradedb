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

//! Returns blocks that nothing in the index references to the free space map, once the
//! transaction that allocated them is known to be gone.
//!
//! A writer takes blocks from the FSM or by extending the relation, fills them, and links them
//! into a segment entry last. An ERROR or a crash before that link leaves the blocks full of data
//! with no owner. This module is VACUUM's answer, modelled on how heap and nbtree VACUUM treat
//! pages left behind by a crashed extension: the page says who allocated it
//! (`BM25PageSpecialData::alloc_xid`, the top-level transaction id written by
//! `BufferMut::init_page`), the allocator's transaction state decides, and a page is freed only
//! under a cleanup lock, so no writer is ever waited for.
//!
//! VACUUM first totals the blocks every structure owns from their list headers
//! (`LinkedListData::npages`) and the blocks the FSM holds. When the total equals the relation
//! size nothing is missing and the walk below is skipped, as it is while a merge is writing
//! output, since its unpublished blocks are the gap. Otherwise a block is a candidate when no
//! segment, list, or FSM page references it and the FSM does not hold it. The structures are
//! read before the FSM, so a block garbage collection moves from a dead segment into the FSM
//! during the walk is seen in one place or the other.
//!
//! A candidate is judged by its stamp against the oldest transaction that was running when the
//! sweep began. A stamp at or past that horizon belongs to a transaction that may publish the
//! block before the sweep ends, so it waits for the next sweep. An older stamp belongs to a
//! transaction that finished before the walk started: had it published, the walk would have
//! found the block. An older stamp that aborted or crashed is freed, and a never written page
//! is a crashed extension and is freed. An older stamp that committed is a block a write
//! outside the merge path left behind, an INSERT flush that errored inside a subtransaction
//! the application then committed; this module is about merges and leaves those blocks
//! alone.
//!
//! A freed block is claimed under a conditional cleanup lock, its stamp re-read, and stamped
//! `FrozenTransactionId` before it is returned. The process-local registry in
//! `storage::unpublished` returns a page only after stamping it with its own running id under
//! the same lock, and skips a Frozen one, so whichever side writes first owns the return and no
//! block reaches the FSM twice. The cleanup lock is what makes a block in transit safe:
//! `FreeSpaceManager::drain` and relation extension pin a block until its writer locks it, so a
//! pinned block is never claimed, however stale its page.
//!
//! Reachability is not optional. Segment entries carry no creator id, so a segment published by
//! a transaction that later rolled back stays linked and keeps serving. Its blocks carry an
//! aborted stamp and must not be freed on the stamp alone.

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{
    BM25PageSpecialData, LinkedList, LinkedListData, MVCCEntry, SegmentMetaEntry,
    SegmentMetaEntryContent,
};
use crate::postgres::storage::buffer::{BufferManager, Page, PinnedBuffer};
use crate::postgres::storage::fsm::FreeSpaceManager;
use crate::postgres::storage::fsm::v2::{FsmBlock, V2FSM};
use crate::postgres::storage::merge::MergeEntry;
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use pgrx::{check_for_interrupts, pg_sys};

/// What the allocation stamp says about an unreferenced block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The allocating transaction was running when the sweep began, or began since, and may
    /// still publish the block. Judged by a later sweep.
    Recent,
    /// The allocating transaction is still running.
    InProgress,
    /// The allocating transaction committed before the sweep began without publishing. Only a
    /// write outside the merge path does that, an INSERT flush that errored inside a
    /// subtransaction the application committed. Kept and reported; not this module's leak.
    Committed,
    /// The allocating transaction aborted or crashed. Freed.
    Abandoned,
    /// Written before pages carried a stamp. Kept.
    Unstamped,
    /// Never written: a relation extension whose writer is gone. Freed.
    Empty,
    /// Stamped by an earlier sweep and yet outside the FSM: that sweep crashed between the
    /// stamp and the FSM, or a writer drained it and then let the registry skip it. Returned
    /// again; only the sweep ever returns a Frozen block.
    Freed,
    /// Older than the oldest transaction the commit log still answers for. Nothing that old can
    /// still publish it. Freed.
    BeyondClog,
}

impl Verdict {
    fn frees(self) -> bool {
        matches!(
            self,
            Verdict::Abandoned | Verdict::Empty | Verdict::Freed | Verdict::BeyondClog
        )
    }

    #[cfg(feature = "pg_test")]
    fn as_str(self) -> &'static str {
        match self {
            Verdict::Recent => "recent",
            Verdict::InProgress => "in_progress",
            Verdict::Committed => "committed",
            Verdict::Abandoned => "abandoned",
            Verdict::Unstamped => "unstamped",
            Verdict::Empty => "empty",
            Verdict::Freed => "freed",
            Verdict::BeyondClog => "beyond_clog",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Candidate {
    pub blockno: pg_sys::BlockNumber,
    pub alloc_xid: pg_sys::TransactionId,
    pub verdict: Verdict,
}

/// Relation size minus the blocks accounted for by structures and the FSM. Zero means there is
/// nothing to sweep. Negative means something is counted twice, which the sweep tolerates by
/// running anyway.
#[cfg(any(test, feature = "pg_test"))]
pub unsafe fn unaccounted_blocks(indexrel: &PgSearchRelation) -> i64 {
    let (accounted, nblocks) = accounted_blocks(indexrel, false);
    nblocks as i64 - accounted as i64
}

/// Every block that is neither referenced nor held by the FSM, with its verdict. Read-only.
#[cfg(any(test, feature = "pg_test"))]
pub unsafe fn candidates(indexrel: &PgSearchRelation) -> Vec<Candidate> {
    candidates_as_of(indexrel, sweep_horizon())
}

/// Frees every candidate whose verdict allows it and returns how many. Skips the walk entirely
/// when the accounting says no block is missing, or when a merge is writing output.
pub unsafe fn reclaim(indexrel: &PgSearchRelation) -> usize {
    reclaim_as_of(indexrel, sweep_horizon())
}

/// [`candidates`] judged against `horizon` instead of the oldest running transaction. A test
/// runs inside one transaction and passes the next xid to stand for one that has finished.
#[cfg(any(test, feature = "pg_test"))]
pub(crate) unsafe fn candidates_as_of(
    indexrel: &PgSearchRelation,
    horizon: pg_sys::TransactionId,
) -> Vec<Candidate> {
    candidates_in(indexrel, relation_blocks(indexrel), horizon)
}

/// [`reclaim`] judged against `horizon` instead of the oldest running transaction. A test runs
/// inside one transaction and passes the next xid to stand for one that has finished.
pub(crate) unsafe fn reclaim_as_of(
    indexrel: &PgSearchRelation,
    horizon: pg_sys::TransactionId,
) -> usize {
    let (accounted, nblocks) = accounted_blocks(indexrel, true);
    if accounted == nblocks as u64 {
        return 0;
    }
    if accounted > nblocks as u64 {
        pgrx::debug1!(
            "pg_search: index {} accounts for {accounted} blocks but has {nblocks}; sweeping anyway",
            indexrel.name()
        );
    }
    let meta = MetaPage::open(indexrel);
    if merge_in_flight(indexrel, &meta) {
        pgrx::debug1!(
            "pg_search: index {} has a merge in flight; its output is the unaccounted space",
            indexrel.name()
        );
        return 0;
    }

    let freeable = candidates_in(indexrel, nblocks, horizon)
        .into_iter()
        .filter(|candidate| candidate.verdict.frees())
        .collect::<Vec<_>>();
    if freeable.is_empty() {
        return 0;
    }

    let mut bman = BufferManager::new(indexrel);
    let mut claimed = Vec::new();
    for candidate in freeable {
        check_for_interrupts!();
        if claim(&mut bman, &candidate) {
            claimed.push(candidate.blockno);
        }
    }
    // Stamped before extended. A crash between the two leaves a Frozen page outside the FSM,
    // which the next sweep returns; the other order could leave an aborted stamp on a page the
    // FSM already hands out.
    bman.fsm().extend(&mut bman, claimed.iter().copied());
    claimed.len()
}

/// Stamps the candidate's page `FrozenTransactionId` if it still says what the verdict was
/// based on, and reports whether it did. The cleanup lock is conditional: a page anyone else
/// has pinned is in a writer's hands, whatever its stamp says. A registry return that got
/// there first has left its own running id, which fails the comparison.
unsafe fn claim(bman: &mut BufferManager, candidate: &Candidate) -> bool {
    let Some(mut buffer) = bman.get_buffer_for_cleanup_conditional(candidate.blockno) else {
        return false;
    };
    let (empty, alloc_xid) = {
        let page = buffer.page();
        page_stamp(&page)
    };
    if empty != (candidate.verdict == Verdict::Empty) || alloc_xid != candidate.alloc_xid {
        return false;
    }
    if candidate.verdict == Verdict::Freed {
        return true;
    }
    if empty {
        buffer.init_page();
    }
    buffer
        .page_mut()
        .special_mut::<BM25PageSpecialData>()
        .alloc_xid = pg_sys::FrozenTransactionId;
    true
}

/// The oldest transaction running as the sweep begins. Anything at or past it may still
/// publish during the walk.
unsafe fn sweep_horizon() -> pg_sys::TransactionId {
    pg_sys::GetOldestActiveTransactionId()
}

/// Whether a merge is writing output. Its blocks are unaccounted until it publishes, so the
/// count gate cannot close and a walk would read every block for nothing.
unsafe fn merge_in_flight(indexrel: &PgSearchRelation, meta: &MetaPage) -> bool {
    let (entries, _) = merge_list_snapshot(indexrel, meta);
    let mut bman = BufferManager::new(indexrel);
    entries.iter().any(|entry| !entry.recyclable(&mut bman))
}

unsafe fn relation_blocks(indexrel: &PgSearchRelation) -> pg_sys::BlockNumber {
    pg_sys::RelationGetNumberOfBlocksInFork(indexrel.as_ptr(), pg_sys::ForkNumber::MAIN_FORKNUM)
}

unsafe fn candidates_in(
    indexrel: &PgSearchRelation,
    nblocks: pg_sys::BlockNumber,
    horizon: pg_sys::TransactionId,
) -> Vec<Candidate> {
    let keep = referenced_or_free(indexrel, nblocks);
    let bman = BufferManager::new(indexrel);
    let oldest_clog_xid = oldest_clog_xid();

    (0..nblocks)
        .filter(|blockno| !keep.contains(*blockno))
        .map(|blockno| {
            check_for_interrupts!();
            let buffer = bman.get_buffer(blockno);
            let (empty, alloc_xid) = page_stamp(&buffer.page());
            Candidate {
                blockno,
                alloc_xid,
                verdict: verdict(alloc_xid, empty, horizon, oldest_clog_xid),
            }
        })
        .collect()
}

/// Whether the page was never written, and its stamp if it was.
fn page_stamp(page: &Page<'_>) -> (bool, pg_sys::TransactionId) {
    if page.is_new() {
        (true, pg_sys::InvalidTransactionId)
    } else {
        (false, page.special::<BM25PageSpecialData>().alloc_xid)
    }
}

/// PostgreSQL's rule from `HeapTupleSatisfiesVacuum`, behind the sweep's horizon: ask the proc
/// array before the commit log, and treat "neither running nor committed" as aborted, because
/// a crashed transaction never wrote its abort record.
unsafe fn verdict(
    xid: pg_sys::TransactionId,
    empty: bool,
    horizon: pg_sys::TransactionId,
    oldest_clog_xid: pg_sys::TransactionId,
) -> Verdict {
    if empty {
        return Verdict::Empty;
    }
    if xid == pg_sys::FrozenTransactionId {
        return Verdict::Freed;
    }
    if xid.into_inner() < pg_sys::FirstNormalTransactionId.into_inner() {
        return Verdict::Unstamped;
    }
    if !pg_sys::TransactionIdPrecedes(xid, horizon) {
        return Verdict::Recent;
    }
    if pg_sys::TransactionIdIsInProgress(xid) {
        return Verdict::InProgress;
    }
    if pg_sys::TransactionIdPrecedes(xid, oldest_clog_xid) {
        return Verdict::BeyondClog;
    }
    if pg_sys::TransactionIdDidCommit(xid) {
        return Verdict::Committed;
    }
    Verdict::Abandoned
}

/// The commit log is truncated up to this xid; asking it about an older one raises an ERROR.
unsafe fn oldest_clog_xid() -> pg_sys::TransactionId {
    #[cfg(any(feature = "pg15", feature = "pg16"))]
    {
        let cache = std::ptr::addr_of!(pg_sys::ShmemVariableCache).read();
        (*cache).oldestXid
    }
    #[cfg(not(any(feature = "pg15", feature = "pg16")))]
    {
        let vars = std::ptr::addr_of!(pg_sys::TransamVariables).read();
        (*vars).oldestXid
    }
}

struct BlockSet {
    words: Vec<u64>,
    nblocks: pg_sys::BlockNumber,
}

impl BlockSet {
    fn new(nblocks: pg_sys::BlockNumber) -> Self {
        Self {
            words: vec![0; nblocks.div_ceil(64) as usize],
            nblocks,
        }
    }

    /// Returns whether the block was newly added. A block past the end of the relation is not
    /// added: it was read off a page that no longer belongs to the structure being walked.
    fn insert(&mut self, blockno: pg_sys::BlockNumber) -> bool {
        if blockno >= self.nblocks || self.contains(blockno) {
            return false;
        }
        self.words[(blockno / 64) as usize] |= 1 << (blockno % 64);
        true
    }

    fn contains(&self, blockno: pg_sys::BlockNumber) -> bool {
        blockno < self.nblocks && self.words[(blockno / 64) as usize] & (1 << (blockno % 64)) != 0
    }

    fn extend_from(&mut self, other: &BlockSet) {
        for (word, bits) in self.words.iter_mut().zip(&other.words) {
            *word |= bits;
        }
    }
}

// ---------------------------------------------------------
// Counting: headers only
// ---------------------------------------------------------

/// Blocks every structure owns plus blocks the FSM holds or occupies, from list headers and
/// the FSM pages alone, with the relation size they are compared against. No mapping page or
/// data page is read for a list whose header carries its count; a list without one is walked,
/// and with `backfill` its header is updated so the next count is a single read.
pub unsafe fn accounted_blocks(
    indexrel: &PgSearchRelation,
    backfill: bool,
) -> (u64, pg_sys::BlockNumber) {
    let meta = MetaPage::open(indexrel);
    // Opening the metadata page and taking the merge lock create their lists on first use, so
    // the relation size is read only once both have run.
    let (segment_id_lists, merge_list_pages) = merge_list_blocks(indexrel, &meta);
    let nblocks = relation_blocks(indexrel);
    let bman = BufferManager::new(indexrel);
    let mut walked = BlockSet::new(nblocks);
    let mut total = 0u64;

    for blockno in meta.fixed_blocks() {
        total += chain_len(&bman, &mut walked, blockno);
    }
    V2FSM::open(meta.fsm()).visit(&bman, |block| match block {
        FsmBlock::Page(blockno) => {
            let new = walked.insert(blockno);
            total += new as u64;
            new
        }
        FsmBlock::Free(_) => {
            total += 1;
            true
        }
    });

    total += bytes_list_pages(indexrel, meta.schema_bytes().header_blockno, backfill);
    total += bytes_list_pages(indexrel, meta.settings_bytes().header_blockno, backfill);

    total += segment_entries_pages(indexrel, meta.segment_metas(), backfill);
    if let Some(garbage) = meta.segment_metas_garbage() {
        total += segment_entries_pages(indexrel, garbage, backfill);
    }

    for header in segment_id_lists {
        total += bytes_list_pages(indexrel, header, backfill);
    }
    total += merge_list_pages.len() as u64;

    (total, nblocks)
}

/// The segment-id list of every merge entry, and the pages of the merge list itself, read
/// under the merge lock. The lock is an exclusive content lock on the merge-lock block, so it
/// is released before the fixed chains, which include that block, are read.
unsafe fn merge_list_blocks(
    indexrel: &PgSearchRelation,
    meta: &MetaPage,
) -> (Vec<pg_sys::BlockNumber>, Vec<pg_sys::BlockNumber>) {
    let (entries, pages) = merge_list_snapshot(indexrel, meta);
    let segment_id_lists = entries
        .iter()
        .map(|entry| entry.segment_ids_start_blockno)
        .collect();
    (segment_id_lists, pages)
}

/// The merge list's entries and pages, read under the merge lock and returned without it.
unsafe fn merge_list_snapshot(
    indexrel: &PgSearchRelation,
    meta: &MetaPage,
) -> (Vec<MergeEntry>, Vec<pg_sys::BlockNumber>) {
    let merge_lock = meta.acquire_merge_lock();
    let merge_list =
        LinkedItemList::<MergeEntry>::open(indexrel, merge_lock.merge_list_header_blockno());
    (merge_list.list(None), merge_list.pages())
}

unsafe fn chain_len(
    bman: &BufferManager,
    walked: &mut BlockSet,
    start: pg_sys::BlockNumber,
) -> u64 {
    let mut len = 0;
    let mut blockno = start;
    while blockno != pg_sys::InvalidBlockNumber && walked.insert(blockno) {
        len += 1;
        blockno = bman.get_buffer(blockno).page().next_blockno();
    }
    len
}

/// Every block of a bytes list. `freeable_blocks` reaches data pages through the mapping
/// pages, which a list written without a writer (schema, settings) does not have; those are
/// walked through `next_blockno` from the start page instead.
unsafe fn bytes_list_blocks(
    indexrel: &PgSearchRelation,
    header_blockno: pg_sys::BlockNumber,
) -> Vec<pg_sys::BlockNumber> {
    let list = LinkedBytesList::open(indexrel, header_blockno);
    let data = list.get_linked_list_data();
    if data.blocklist_start != pg_sys::InvalidBlockNumber {
        return list.freeable_blocks().collect();
    }
    let bman = BufferManager::new(indexrel);
    let mut blocks = Vec::new();
    for start in [header_blockno, data.start_blockno] {
        let mut blockno = start;
        while blockno != pg_sys::InvalidBlockNumber && !blocks.contains(&blockno) {
            blocks.push(blockno);
            blockno = bman.get_buffer(blockno).page().next_blockno();
        }
    }
    blocks
}

unsafe fn bytes_list_pages(
    indexrel: &PgSearchRelation,
    header_blockno: pg_sys::BlockNumber,
    backfill: bool,
) -> u64 {
    let list = LinkedBytesList::open(indexrel, header_blockno);
    let data = list.get_linked_list_data();
    // Only `finalize_and_write` keeps the count current, and only it writes a mapping. A list
    // written without one (schema, settings) is short and is walked every time.
    let finalized = data.blocklist_start != pg_sys::InvalidBlockNumber;
    if finalized && data.npages != 0 {
        return data.npages as u64;
    }
    let npages = bytes_list_blocks(indexrel, header_blockno).len() as u32;
    if backfill && finalized {
        let mut bman = BufferManager::new(indexrel);
        let mut header = bman.get_buffer_mut(header_blockno);
        header
            .page_mut()
            .contents_mut_in_place::<LinkedListData>()
            .npages = npages;
    }
    npages as u64
}

/// Pins each entry's pintest block while the header lock is held, the same pin readers hold:
/// garbage collection skips a pinned entry, so its files stay put while they are counted or
/// walked.
unsafe fn pinned_entries(
    list: &mut LinkedItemList<SegmentMetaEntry>,
) -> (Vec<SegmentMetaEntry>, Vec<PinnedBuffer>) {
    let mut entries = Vec::new();
    let mut pins = Vec::new();
    list.for_each(|bman, entry| {
        let pintest = entry.pintest_blockno();
        if pintest != pg_sys::InvalidBlockNumber {
            pins.push(bman.pinned_buffer(pintest));
        }
        entries.push(entry);
    });
    (entries, pins)
}

/// The same ownership `SegmentMetaEntry::freeable_blocks` frees: an orphaned-delete clone owns
/// only its delete file, the rest of its files belong to the live entry.
unsafe fn segment_entries_pages(
    indexrel: &PgSearchRelation,
    mut list: LinkedItemList<SegmentMetaEntry>,
    backfill: bool,
) -> u64 {
    let (entries, pins) = pinned_entries(&mut list);
    let mut total = 0u64;
    for entry in &entries {
        match &entry.content {
            SegmentMetaEntryContent::Immutable(content) if entry.is_orphaned_delete() => {
                let delete = content.delete.as_ref().unwrap().file_entry.starting_block;
                total += bytes_list_pages(indexrel, delete, backfill);
            }
            SegmentMetaEntryContent::Immutable(content) => {
                for (file_entry, _) in content.file_entries() {
                    total += bytes_list_pages(indexrel, file_entry.starting_block, backfill);
                }
            }
            SegmentMetaEntryContent::Mutable(content) => {
                total += content.open(indexrel).pages().len() as u64;
            }
        }
    }
    drop(pins);
    total + list.pages().len() as u64
}

// ---------------------------------------------------------
// Marking: the full walk, only when the count says blocks are missing
// ---------------------------------------------------------

/// Every block some structure references, plus every block the FSM holds or occupies.
///
/// Each structure is read under its own locks, so the result is a union of per-structure
/// snapshots. That only ever over-approximates what to keep: a block that moves from a segment
/// into the FSM during the walk is seen by at least one of the two, and a block the FSM hands
/// out during the walk is kept because of what its page still says, see `verdict`.
unsafe fn referenced_or_free(
    indexrel: &PgSearchRelation,
    nblocks: pg_sys::BlockNumber,
) -> BlockSet {
    let meta = MetaPage::open(indexrel);
    let bman = BufferManager::new(indexrel);
    let mut keep = BlockSet::new(nblocks);
    // Chains are followed through `next_blockno` on pages read without a lock on the whole
    // chain, so a page recycled mid-walk can point anywhere. This set stops a chain at the first
    // page seen twice; `keep` cannot serve, since the legacy marks below pre-fill it.
    let mut walked = BlockSet::new(nblocks);

    for blockno in meta.fixed_blocks() {
        mark_chain(&bman, &mut keep, &mut walked, blockno);
    }
    // Legacy layout: the first six blocks had fixed roles before the metadata page named them.
    for blockno in 0..=6 {
        keep.insert(blockno);
    }

    for header in [
        meta.schema_bytes().header_blockno,
        meta.settings_bytes().header_blockno,
    ] {
        for blockno in bytes_list_blocks(indexrel, header) {
            keep.insert(blockno);
        }
    }

    mark_segment_entries(indexrel, &mut keep, meta.segment_metas());
    if let Some(garbage) = meta.segment_metas_garbage() {
        mark_segment_entries(indexrel, &mut keep, garbage);
    }

    let (segment_id_lists, merge_list_pages) = merge_list_blocks(indexrel, &meta);
    for header in segment_id_lists {
        mark_bytes_list(&mut keep, LinkedBytesList::open(indexrel, header));
    }
    for blockno in merge_list_pages {
        keep.insert(blockno);
    }

    // Last, after every list. A block garbage collection frees between two of these reads moves
    // from a dead segment's entry into the FSM, and the entry was read first; read the FSM first
    // and a block freed between the two reads would be in neither.
    keep.extend_from(&fsm_blocks(indexrel, &meta, nblocks));

    keep
}

/// Blocks the FSM holds as free and the pages it occupies.
unsafe fn fsm_blocks(
    indexrel: &PgSearchRelation,
    meta: &MetaPage,
    nblocks: pg_sys::BlockNumber,
) -> BlockSet {
    let bman = BufferManager::new(indexrel);
    let mut blocks = BlockSet::new(nblocks);
    V2FSM::open(meta.fsm()).visit(&bman, |block| match block {
        FsmBlock::Page(blockno) => blocks.insert(blockno),
        FsmBlock::Free(blockno) => {
            blocks.insert(blockno);
            true
        }
    });
    blocks
}

unsafe fn mark_chain(
    bman: &BufferManager,
    keep: &mut BlockSet,
    walked: &mut BlockSet,
    start: pg_sys::BlockNumber,
) {
    let mut blockno = start;
    while blockno != pg_sys::InvalidBlockNumber && walked.insert(blockno) {
        keep.insert(blockno);
        blockno = bman.get_buffer(blockno).page().next_blockno();
    }
}

fn mark_bytes_list(keep: &mut BlockSet, list: LinkedBytesList) {
    for blockno in list.freeable_blocks() {
        keep.insert(blockno);
    }
}

unsafe fn mark_segment_entries(
    indexrel: &PgSearchRelation,
    keep: &mut BlockSet,
    mut list: LinkedItemList<SegmentMetaEntry>,
) {
    let (entries, pins) = pinned_entries(&mut list);
    for entry in &entries {
        for blockno in entry.freeable_blocks(indexrel) {
            keep.insert(blockno);
        }
    }
    drop(pins);
    for blockno in list.pages() {
        keep.insert(blockno);
    }
}

// ---------------------------------------------------------
// SQL, for the multi-session tests only
// ---------------------------------------------------------

#[cfg(feature = "pg_test")]
#[pgrx::pg_extern(name = "unaccounted_blocks")]
unsafe fn unaccounted_blocks_sql(index: pgrx::PgRelation) -> i64 {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    unaccounted_blocks(&index)
}

#[cfg(feature = "pg_test")]
#[pgrx::pg_extern]
unsafe fn unreferenced_blocks(
    index: pgrx::PgRelation,
) -> pgrx::iter::TableIterator<
    'static,
    (
        pgrx::name!(blockno, i64),
        pgrx::name!(alloc_xid, pg_sys::TransactionId),
        pgrx::name!(state, String),
    ),
> {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    pgrx::iter::TableIterator::new(candidates(&index).into_iter().map(|candidate| {
        (
            candidate.blockno as i64,
            candidate.alloc_xid,
            candidate.verdict.as_str().to_string(),
        )
    }))
}

/// Per-structure counts next to a full walk of the same structure, to find where the header
/// counts and reality disagree.
#[cfg(any(test, feature = "pg_test"))]
pub(crate) unsafe fn accounting_report(indexrel: &PgSearchRelation) -> String {
    use std::fmt::Write;
    let nblocks = relation_blocks(indexrel);
    let meta = MetaPage::open(indexrel);
    let bman = BufferManager::new(indexrel);
    let mut out = String::new();
    writeln!(out, "nblocks={nblocks}").unwrap();
    let mut walked = BlockSet::new(nblocks);
    for blockno in meta.fixed_blocks() {
        let n = chain_len(&bman, &mut walked, blockno);
        writeln!(out, "fixed {blockno}: chain {n}").unwrap();
    }
    let (mut pages, mut free, mut dup) = (0u64, 0u64, 0u64);
    let mut seen = BlockSet::new(nblocks);
    V2FSM::open(meta.fsm()).visit(&bman, |block| match block {
        FsmBlock::Page(b) => {
            let new = walked.insert(b);
            pages += new as u64;
            new
        }
        FsmBlock::Free(b) => {
            free += 1;
            if !seen.insert(b) {
                dup += 1;
                writeln!(out, "fsm free duplicate or out of range: {b}").unwrap();
            }
            true
        }
    });
    writeln!(out, "fsm pages={pages} free={free} dup={dup}").unwrap();
    let bytes = |name: &str, header: pg_sys::BlockNumber| -> String {
        let list = LinkedBytesList::open(indexrel, header);
        let npages = list.get_linked_list_data().npages;
        let walked = bytes_list_blocks(indexrel, header).len();
        format!("{name} header={header} npages={npages} walked={walked}\n")
    };
    out.push_str(&bytes("schema", meta.schema_bytes().header_blockno));
    out.push_str(&bytes("settings", meta.settings_bytes().header_blockno));
    let mut metas = meta.segment_metas();
    let (entries, _pins) = pinned_entries(&mut metas);
    for entry in &entries {
        match &entry.content {
            SegmentMetaEntryContent::Immutable(content) => {
                for (file_entry, component) in content.file_entries() {
                    out.push_str(&bytes(
                        &format!("{} {component:?}", entry.segment_id()),
                        file_entry.starting_block,
                    ));
                }
            }
            SegmentMetaEntryContent::Mutable(content) => {
                let list = content.open(indexrel);
                writeln!(
                    out,
                    "{} mutable header={} pages={}",
                    entry.segment_id(),
                    content.header_block,
                    list.pages().len()
                )
                .unwrap();
            }
        }
    }
    writeln!(out, "metas pages={:?}", metas.pages()).unwrap();
    if let Some(garbage) = meta.segment_metas_garbage() {
        writeln!(out, "garbage pages={:?}", garbage.pages()).unwrap();
    }
    let (segment_id_lists, merge_list_pages) = merge_list_blocks(indexrel, &meta);
    writeln!(
        out,
        "merge entries={} pages={:?}",
        segment_id_lists.len(),
        merge_list_pages
    )
    .unwrap();
    for candidate in candidates(indexrel) {
        let buffer = bman.get_buffer(candidate.blockno);
        let page = buffer.page();
        let data = page.contents::<LinkedListData>();
        writeln!(
            out,
            "candidate {candidate:?} next={} pd_lower={} as_list={data:?}",
            page.next_blockno(),
            page.header().pd_lower
        )
        .unwrap();
    }
    out
}

/// Fixtures for the stamp. A test runs inside one transaction, so pages it allocates carry its
/// own running id; these write the id of a transaction that aborted, or any other, instead.
#[cfg(any(test, feature = "pg_test"))]
pub(crate) mod testing {
    use super::*;

    /// A transaction id that has aborted: a subtransaction given an id and rolled back.
    pub(crate) unsafe fn aborted_xid() -> pg_sys::TransactionId {
        let context = pg_sys::CurrentMemoryContext;
        let resource_owner = pg_sys::CurrentResourceOwner;
        pg_sys::BeginInternalSubTransaction(std::ptr::null());
        let xid = pg_sys::GetCurrentTransactionId();
        pg_sys::RollbackAndReleaseCurrentSubTransaction();
        pg_sys::MemoryContextSwitchTo(context);
        pg_sys::CurrentResourceOwner = resource_owner;
        xid
    }

    /// The next transaction id: as a horizon, every transaction so far counts as finished.
    pub(crate) unsafe fn next_xid() -> pg_sys::TransactionId {
        pg_sys::TransactionId::from(pg_sys::ReadNextFullTransactionId().value as u32)
    }

    pub(crate) unsafe fn stamp(
        indexrel: &PgSearchRelation,
        blocks: &[pg_sys::BlockNumber],
        xid: pg_sys::TransactionId,
    ) {
        let mut bman = BufferManager::new(indexrel);
        for blockno in blocks {
            bman.get_buffer_mut(*blockno)
                .page_mut()
                .special_mut::<BM25PageSpecialData>()
                .alloc_xid = xid;
        }
    }

    pub(crate) unsafe fn stamp_of(
        indexrel: &PgSearchRelation,
        blockno: pg_sys::BlockNumber,
    ) -> pg_sys::TransactionId {
        BufferManager::new(indexrel)
            .get_buffer(blockno)
            .page()
            .special::<BM25PageSpecialData>()
            .alloc_xid
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::testing::*;
    use super::*;
    use pgrx::prelude::*;

    unsafe fn test_index(table: &str) -> PgSearchRelation {
        Spi::run(&format!(
            "CREATE TABLE {table} (id SERIAL PRIMARY KEY, data TEXT)"
        ))
        .unwrap();
        Spi::run(&format!(
            "INSERT INTO {table} (data) SELECT 'row ' || g FROM generate_series(1, 2000) g"
        ))
        .unwrap();
        // Every segment is immutable, so finalized list headers with page counts exist.
        Spi::run(&format!(
            "CREATE INDEX {table}_idx ON {table} USING bm25 (id, data) \
             WITH (key_field = 'id', mutable_segment_rows = 0)"
        ))
        .unwrap();
        Spi::run(&format!(
            "INSERT INTO {table} (data) SELECT 'late ' || g FROM generate_series(1, 200) g"
        ))
        .unwrap();
        let oid = Spi::get_one::<pg_sys::Oid>(&format!("SELECT '{table}_idx'::regclass::oid"))
            .unwrap()
            .unwrap();
        PgSearchRelation::open(oid)
    }

    /// Allocates and fills a list that nothing references, returning its blocks.
    unsafe fn allocate_unpublished(indexrel: &PgSearchRelation) -> Vec<pg_sys::BlockNumber> {
        let list = LinkedBytesList::create_with_fsm(indexrel);
        let mut writer = list.writer();
        writer.write(&vec![7u8; 64 * 1024]).unwrap();
        let list = writer.finalize_and_write().unwrap();
        let mut blocks = list.freeable_blocks().collect::<Vec<_>>();
        blocks.sort_unstable();
        blocks
    }

    fn sorted_blocks(candidates: &[Candidate], verdict: Verdict) -> Vec<pg_sys::BlockNumber> {
        let mut blocks = candidates
            .iter()
            .filter(|candidate| candidate.verdict == verdict)
            .map(|candidate| candidate.blockno)
            .collect::<Vec<_>>();
        blocks.sort_unstable();
        blocks
    }

    #[pg_test]
    unsafe fn fresh_index_accounts_for_every_block() {
        let indexrel = test_index("sweep_fresh");
        let found = candidates(&indexrel);
        assert!(
            found.is_empty(),
            "unreferenced blocks on a fresh index: {found:?}\n{}",
            accounting_report(&indexrel)
        );
        assert_eq!(
            unaccounted_blocks(&indexrel),
            0,
            "{}",
            accounting_report(&indexrel)
        );
    }

    #[pg_test]
    unsafe fn finalized_list_header_counts_its_pages() {
        let indexrel = test_index("sweep_npages");
        let list = LinkedBytesList::create_with_fsm(&indexrel);
        let header = list.header_blockno;
        let mut writer = list.writer();
        writer.write(&vec![7u8; 64 * 1024]).unwrap();
        writer.finalize_and_write().unwrap();

        let list = LinkedBytesList::open(&indexrel, header);
        let npages = list.get_linked_list_data().npages as usize;
        assert_eq!(npages, list.freeable_blocks().count());
    }

    #[pg_test]
    unsafe fn abandoned_allocation_is_reclaimed() {
        let indexrel = test_index("sweep_abandoned");
        let blocks = allocate_unpublished(&indexrel);
        stamp(&indexrel, &blocks, aborted_xid());
        assert_eq!(unaccounted_blocks(&indexrel), blocks.len() as i64);

        // The aborted transaction began after this one, so a sweep that starts now keeps its
        // blocks: it cannot tell them from a merge still running.
        let found = candidates(&indexrel);
        assert_eq!(sorted_blocks(&found, Verdict::Recent), blocks);
        assert_eq!(reclaim(&indexrel), 0);

        // Once it is behind the horizon, the blocks are abandoned.
        let found = candidates_as_of(&indexrel, next_xid());
        assert_eq!(sorted_blocks(&found, Verdict::Abandoned), blocks);
        assert_eq!(found.len(), blocks.len(), "{found:?}");

        assert_eq!(reclaim_as_of(&indexrel, next_xid()), blocks.len());
        assert!(candidates(&indexrel).is_empty());
        assert_eq!(unaccounted_blocks(&indexrel), 0);
        for blockno in blocks {
            assert_eq!(stamp_of(&indexrel, blockno), pg_sys::FrozenTransactionId);
        }
    }

    #[pg_test]
    unsafe fn in_progress_allocation_is_kept() {
        let indexrel = test_index("sweep_in_progress");
        let blocks = allocate_unpublished(&indexrel);
        assert_eq!(unaccounted_blocks(&indexrel), blocks.len() as i64);

        let found = candidates(&indexrel);
        assert_eq!(sorted_blocks(&found, Verdict::Recent), blocks);
        assert_eq!(found.len(), blocks.len(), "{found:?}");
        assert_eq!(reclaim(&indexrel), 0);

        // Behind the horizon the stamp is still in progress, which also keeps the blocks.
        let found = candidates_as_of(&indexrel, next_xid());
        assert_eq!(sorted_blocks(&found, Verdict::InProgress), blocks);
        assert_eq!(reclaim_as_of(&indexrel, next_xid()), 0);
    }

    /// A transaction that committed before the sweep began and published nothing left blocks
    /// behind outside the merge path. They are reported and kept.
    #[pg_test]
    unsafe fn committed_unpublished_allocation_is_kept() {
        let indexrel = test_index("sweep_committed");
        let blocks = allocate_unpublished(&indexrel);
        let committed = Spi::get_one::<i64>(
            "SELECT xmin::text::bigint FROM pg_extension WHERE extname = 'pg_search'",
        )
        .unwrap()
        .unwrap() as u32;
        if committed < pg_sys::FirstNormalTransactionId.into_inner() {
            // Frozen by a VACUUM of the catalog; no committed id to borrow.
            return;
        }
        stamp(&indexrel, &blocks, pg_sys::TransactionId::from(committed));

        let found = candidates(&indexrel);
        assert_eq!(sorted_blocks(&found, Verdict::Committed), blocks);
        assert_eq!(reclaim(&indexrel), 0);
        assert_eq!(unaccounted_blocks(&indexrel), blocks.len() as i64);
    }

    /// A relation extension whose writer is gone leaves never-written pages. They are freed.
    #[pg_test]
    unsafe fn empty_extension_is_reclaimed() {
        let indexrel = test_index("sweep_empty");
        let bman = BufferManager::new(&indexrel);
        let blocks = bman
            .buffer_access()
            .new_buffers(3)
            .map(|pg_buffer| {
                let blockno = pg_sys::BufferGetBlockNumber(pg_buffer);
                pg_sys::UnlockReleaseBuffer(pg_buffer);
                blockno
            })
            .collect::<Vec<_>>();
        assert_eq!(unaccounted_blocks(&indexrel), 3);

        let found = candidates(&indexrel);
        assert_eq!(sorted_blocks(&found, Verdict::Empty), blocks);
        assert_eq!(reclaim(&indexrel), 3);
        assert_eq!(unaccounted_blocks(&indexrel), 0);
        for blockno in blocks {
            assert_eq!(stamp_of(&indexrel, blockno), pg_sys::FrozenTransactionId);
        }
    }

    /// A pinned block is in some writer's hands, whatever its page says.
    #[pg_test]
    unsafe fn pinned_block_is_not_claimed() {
        let indexrel = test_index("sweep_pinned");
        let blocks = allocate_unpublished(&indexrel);
        stamp(&indexrel, &blocks, aborted_xid());
        let bman = BufferManager::new(&indexrel);
        let pin = bman.pinned_buffer(blocks[0]);

        assert_eq!(reclaim_as_of(&indexrel, next_xid()), blocks.len() - 1);
        assert_eq!(unaccounted_blocks(&indexrel), 1);
        assert_ne!(stamp_of(&indexrel, blocks[0]), pg_sys::FrozenTransactionId);
        drop(pin);
        assert_eq!(reclaim_as_of(&indexrel, next_xid()), 1);
        assert_eq!(unaccounted_blocks(&indexrel), 0);
    }

    /// A block an earlier sweep stamped and lost before the FSM is returned again.
    #[pg_test]
    unsafe fn frozen_block_outside_the_fsm_is_returned_again() {
        let indexrel = test_index("sweep_frozen");
        let blocks = allocate_unpublished(&indexrel);
        stamp(&indexrel, &blocks, pg_sys::FrozenTransactionId);
        let found = candidates(&indexrel);
        assert_eq!(sorted_blocks(&found, Verdict::Freed), blocks);
        assert_eq!(reclaim(&indexrel), blocks.len());
        assert!(candidates(&indexrel).is_empty());
        assert_eq!(reclaim(&indexrel), 0);
    }

    /// Pages written before stamping and headers written before counting: nothing is freed, the
    /// counts are backfilled, and the next VACUUM reads headers only.
    #[pg_test]
    unsafe fn old_format_index_is_kept_and_backfilled() {
        let indexrel = test_index("sweep_old");
        let leaked = allocate_unpublished(&indexrel);
        // Every component file of every immutable segment is a finalized list with a header.
        let headers = {
            let mut metas = MetaPage::open(&indexrel).segment_metas();
            let (entries, _pins) = pinned_entries(&mut metas);
            entries
                .iter()
                .flat_map(|entry| match &entry.content {
                    SegmentMetaEntryContent::Immutable(content) => content
                        .file_entries()
                        .map(|(file_entry, _)| file_entry.starting_block)
                        .collect::<Vec<_>>(),
                    SegmentMetaEntryContent::Mutable(_) => Vec::new(),
                })
                .collect::<Vec<_>>()
        };
        assert!(!headers.is_empty(), "the build wrote an immutable segment");
        let npages = |header| {
            LinkedBytesList::open(&indexrel, header)
                .get_linked_list_data()
                .npages
        };
        assert!(headers.iter().all(|header| npages(*header) != 0));

        // An index written before this change: no page stamped, no header counted.
        let nblocks = relation_blocks(&indexrel);
        let mut bman = BufferManager::new(&indexrel);
        for blockno in 0..nblocks {
            let mut buffer = bman.get_buffer_mut(blockno);
            if buffer.page().is_new() {
                continue;
            }
            buffer
                .page_mut()
                .special_mut::<BM25PageSpecialData>()
                .alloc_xid = pg_sys::InvalidTransactionId;
        }
        for header in &headers {
            bman.get_buffer_mut(*header)
                .page_mut()
                .contents_mut_in_place::<LinkedListData>()
                .npages = 0;
        }

        let found = candidates_as_of(&indexrel, next_xid());
        assert_eq!(sorted_blocks(&found, Verdict::Unstamped), leaked);
        assert_eq!(reclaim_as_of(&indexrel, next_xid()), 0);
        assert!(headers.iter().all(|header| npages(*header) != 0));
        assert_eq!(unaccounted_blocks(&indexrel), leaked.len() as i64);
    }

    /// A segment published by a transaction that then rolled back stays linked, so its blocks
    /// carry an aborted stamp but must never be freed.
    #[pg_test]
    unsafe fn rolled_back_insert_keeps_its_segment() {
        let indexrel = test_index("sweep_rollback");
        let segment_blocks = |indexrel: &PgSearchRelation| -> Vec<pg_sys::BlockNumber> {
            let mut metas = MetaPage::open(indexrel).segment_metas();
            let (entries, _pins) = pinned_entries(&mut metas);
            entries
                .iter()
                .flat_map(|entry| entry.freeable_blocks(indexrel))
                .collect()
        };
        let before = segment_blocks(&indexrel);
        let context = pg_sys::CurrentMemoryContext;
        let resource_owner = pg_sys::CurrentResourceOwner;
        pg_sys::BeginInternalSubTransaction(std::ptr::null());
        pg_sys::MemoryContextSwitchTo(context);
        Spi::run(
            "INSERT INTO sweep_rollback (data) SELECT 'undone ' || g FROM generate_series(1, 300) g",
        )
        .unwrap();
        let aborted = pg_sys::GetCurrentTransactionId();
        pg_sys::RollbackAndReleaseCurrentSubTransaction();
        pg_sys::MemoryContextSwitchTo(context);
        pg_sys::CurrentResourceOwner = resource_owner;
        // In production the rolled-back transaction is the top-level one and the stamp is its
        // id; here the insert ran under this test's id, so give its blocks the aborted one.
        let published = segment_blocks(&indexrel)
            .into_iter()
            .filter(|blockno| !before.contains(blockno))
            .collect::<Vec<_>>();
        assert!(!published.is_empty());
        stamp(&indexrel, &published, aborted);

        assert_eq!(unaccounted_blocks(&indexrel), 0);
        let found = candidates_as_of(&indexrel, next_xid());
        assert!(found.is_empty(), "{found:?}");
        assert_eq!(reclaim_as_of(&indexrel, next_xid()), 0);

        let visible = Spi::get_one::<i64>(
            "SELECT count(*) FROM sweep_rollback WHERE sweep_rollback @@@ 'data:undone'",
        )
        .unwrap()
        .unwrap();
        assert_eq!(visible, 0);
    }
}

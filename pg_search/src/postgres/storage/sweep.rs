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
//! pages left behind by a crashed extension: the page says who allocated it (`alloc_xid` in
//! [`crate::postgres::storage::block::BM25PageSpecialData`]), the allocator's transaction state
//! decides, and a page is freed only under a cleanup lock, so no writer is ever waited for.
//!
//! VACUUM first totals the blocks every [`Owner`] holds, from list headers that store their page
//! count, plus the FSM's. When the total equals the relation size nothing is missing and the walk
//! is skipped, as it is while a merge is writing output, since its unpublished blocks are the
//! gap. Otherwise every referenced block is marked and each unmarked block is judged by its stamp
//! against the oldest transaction running when the sweep began; see [`Verdict`].
//!
//! Reachability is not optional. Segment entries carry no creator id, so a segment published by
//! a transaction that later rolled back stays linked and keeps serving. Its blocks carry an
//! aborted stamp and must not be freed on the stamp alone.

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{
    LinkedList, LinkedListData, MVCCEntry, MutableSegmentEntry, OwnedList, SegmentMetaEntry,
};
use crate::postgres::storage::buffer::BufferManager;
use crate::postgres::storage::fsm::FreeSpaceManager;
use crate::postgres::storage::fsm::v2::{FsmBlock, V2FSM};
use crate::postgres::storage::linked_bytes::LinkedBytesList;
use crate::postgres::storage::linked_items::LinkedItemList;
use crate::postgres::storage::merge::MergeEntry;
use crate::postgres::storage::metadata::MetaPage;
use pgrx::{check_for_interrupts, pg_sys};

/// What the allocation stamp says about an unreferenced block. A stamp from a transaction that
/// was running when the sweep began may still publish during the walk, so it waits; an older
/// stamp belongs to a transaction that finished before the walk and would have been seen
/// publishing.
/// Each variant carries the stamp it was read from, so a verdict and the page state it
/// rests on cannot disagree; [`Verdict::stamp`] gives it back for the claim's re-read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    /// Running when the sweep began, or begun since. Judged by a later sweep.
    Recent(pg_sys::TransactionId),
    /// Still running.
    InProgress(pg_sys::TransactionId),
    /// Committed before the sweep began without publishing. Only a write outside the merge
    /// path does that, an INSERT flush that errored inside a subtransaction the application
    /// committed. Kept; not this module's leak.
    Committed(pg_sys::TransactionId),
    /// Aborted or crashed. Freed.
    Abandoned(pg_sys::TransactionId),
    /// Written before pages carried a stamp. Kept.
    Unstamped(pg_sys::TransactionId),
    /// Never written: a relation extension whose writer is gone. Freed.
    Empty,
    /// Stamped by an earlier sweep and yet outside the FSM: that sweep crashed between the
    /// stamp and the FSM, or a writer drained it and then let the registry skip it. Returned
    /// again; only the sweep ever returns a Frozen block.
    Freed,
    /// Older than the oldest transaction the commit log still answers for. Nothing that old can
    /// still publish it. Freed.
    BeyondClog(pg_sys::TransactionId),
}

impl Verdict {
    fn frees(self) -> bool {
        matches!(
            self,
            Verdict::Abandoned(_) | Verdict::Empty | Verdict::Freed | Verdict::BeyondClog(_)
        )
    }

    /// What the page said when it was judged, as `Page::alloc_xid` reads it.
    fn stamp(self) -> Option<pg_sys::TransactionId> {
        match self {
            Verdict::Empty => None,
            Verdict::Freed => Some(pg_sys::FrozenTransactionId),
            Verdict::Recent(xid)
            | Verdict::InProgress(xid)
            | Verdict::Committed(xid)
            | Verdict::Abandoned(xid)
            | Verdict::Unstamped(xid)
            | Verdict::BeyondClog(xid) => Some(xid),
        }
    }

    #[cfg(feature = "pg_test")]
    fn as_str(self) -> &'static str {
        match self {
            Verdict::Recent(_) => "recent",
            Verdict::InProgress(_) => "in_progress",
            Verdict::Committed(_) => "committed",
            Verdict::Abandoned(_) => "abandoned",
            Verdict::Unstamped(_) => "unstamped",
            Verdict::Empty => "empty",
            Verdict::Freed => "freed",
            Verdict::BeyondClog(_) => "beyond_clog",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Candidate {
    blockno: pg_sys::BlockNumber,
    verdict: Verdict,
}

// ---------------------------------------------------------
// Entry points
// ---------------------------------------------------------

/// Frees every candidate whose verdict allows it and returns how many. Skips the walk entirely
/// when the accounting says no block is missing, or when a merge is writing output.
pub unsafe fn reclaim(indexrel: &PgSearchRelation) -> usize {
    reclaim_as_of(indexrel, sweep_horizon())
}

/// [`reclaim`] judged against `horizon` instead of the oldest running transaction. A test runs
/// inside one transaction and passes the next xid to stand for one that has finished.
pub(crate) unsafe fn reclaim_as_of(
    indexrel: &PgSearchRelation,
    horizon: pg_sys::TransactionId,
) -> usize {
    let meta = MetaPage::open(indexrel);
    let merge = MergeListSnapshot::take(indexrel, &meta);
    let nblocks = relation_blocks(indexrel);
    let accounted = accounted_blocks(indexrel, &meta, &merge, nblocks);
    if accounted == nblocks as u64 {
        return 0;
    }
    if accounted > nblocks as u64 {
        pgrx::debug1!(
            "pg_search: index {} accounts for {accounted} blocks but has {nblocks}; sweeping anyway",
            indexrel.name()
        );
    }
    if merge.in_flight(indexrel) {
        pgrx::debug1!(
            "pg_search: index {} has a merge in flight; its output is the unaccounted space",
            indexrel.name()
        );
        return 0;
    }

    let mut bman = BufferManager::new(indexrel);
    let claimed = candidates_in(indexrel, &meta, &merge, nblocks, horizon)
        .filter(|candidate| candidate.verdict.frees() && claim(&mut bman, candidate))
        .map(|candidate| candidate.blockno)
        .collect::<Vec<_>>();
    if claimed.is_empty() {
        return 0;
    }
    // Stamped before extended. A crash between the two leaves a Frozen page outside the FSM,
    // which the next sweep returns; the other order could leave an aborted stamp on a page the
    // FSM already hands out.
    bman.fsm().extend(&mut bman, claimed.iter().copied());
    claimed.len()
}

/// Relation size minus the blocks accounted for. Zero means there is nothing to sweep.
#[cfg(any(test, feature = "pg_test"))]
pub(crate) unsafe fn unaccounted_blocks(indexrel: &PgSearchRelation) -> i64 {
    let meta = MetaPage::open(indexrel);
    let merge = MergeListSnapshot::take(indexrel, &meta);
    let nblocks = relation_blocks(indexrel);
    nblocks as i64 - accounted_blocks(indexrel, &meta, &merge, nblocks) as i64
}

/// Every block that is neither referenced nor held by the FSM, with its verdict. Read-only.
#[cfg(any(test, feature = "pg_test"))]
unsafe fn candidates(indexrel: &PgSearchRelation) -> Vec<Candidate> {
    candidates_as_of(indexrel, sweep_horizon())
}

#[cfg(any(test, feature = "pg_test"))]
unsafe fn candidates_as_of(
    indexrel: &PgSearchRelation,
    horizon: pg_sys::TransactionId,
) -> Vec<Candidate> {
    let meta = MetaPage::open(indexrel);
    let merge = MergeListSnapshot::take(indexrel, &meta);
    let nblocks = relation_blocks(indexrel);
    candidates_in(indexrel, &meta, &merge, nblocks, horizon).collect()
}

/// Stamps the candidate's page `FrozenTransactionId` if it still says what the verdict was
/// based on, and reports whether it did. The cleanup lock is conditional: a page anyone else
/// has pinned is in a writer's hands, whatever its stamp says. A registry return that got
/// there first has left its own running id, which fails the comparison.
unsafe fn claim(bman: &mut BufferManager, candidate: &Candidate) -> bool {
    let Some(mut buffer) = bman.get_buffer_for_cleanup_conditional(candidate.blockno) else {
        return false;
    };
    if buffer.page().alloc_xid() != candidate.verdict.stamp() {
        return false;
    }
    if candidate.verdict != Verdict::Freed {
        buffer.set_alloc_xid(pg_sys::FrozenTransactionId);
    }
    true
}

/// The oldest transaction running as the sweep begins. Anything at or past it may still
/// publish during the walk.
unsafe fn sweep_horizon() -> pg_sys::TransactionId {
    pg_sys::GetOldestActiveTransactionId()
}

unsafe fn relation_blocks(indexrel: &PgSearchRelation) -> pg_sys::BlockNumber {
    pg_sys::RelationGetNumberOfBlocksInFork(indexrel.as_ptr(), pg_sys::ForkNumber::MAIN_FORKNUM)
}

unsafe fn candidates_in(
    indexrel: &PgSearchRelation,
    meta: &MetaPage,
    merge: &MergeListSnapshot,
    nblocks: pg_sys::BlockNumber,
    horizon: pg_sys::TransactionId,
) -> impl Iterator<Item = Candidate> + use<> {
    let keep = referenced_or_free(indexrel, meta, merge, nblocks);
    let bman = BufferManager::new(indexrel);
    let oldest_clog_xid = oldest_clog_xid();
    (0..nblocks)
        .filter(move |blockno| !keep.contains(*blockno))
        .map(move |blockno| {
            check_for_interrupts!();
            let alloc_xid = bman.get_buffer(blockno).page().alloc_xid();
            Candidate {
                blockno,
                verdict: verdict(alloc_xid, horizon, oldest_clog_xid),
            }
        })
}

/// PostgreSQL's rule from `HeapTupleSatisfiesVacuum`, behind the sweep's horizon: ask the proc
/// array before the commit log, and treat "neither running nor committed" as aborted, because
/// a crashed transaction never wrote its abort record.
unsafe fn verdict(
    alloc_xid: Option<pg_sys::TransactionId>,
    horizon: pg_sys::TransactionId,
    oldest_clog_xid: pg_sys::TransactionId,
) -> Verdict {
    let Some(xid) = alloc_xid else {
        return Verdict::Empty;
    };
    if xid == pg_sys::FrozenTransactionId {
        return Verdict::Freed;
    }
    if xid.into_inner() < pg_sys::FirstNormalTransactionId.into_inner() {
        return Verdict::Unstamped(xid);
    }
    if !pg_sys::TransactionIdPrecedes(xid, horizon) {
        return Verdict::Recent(xid);
    }
    if pg_sys::TransactionIdIsInProgress(xid) {
        return Verdict::InProgress(xid);
    }
    if pg_sys::TransactionIdPrecedes(xid, oldest_clog_xid) {
        return Verdict::BeyondClog(xid);
    }
    if pg_sys::TransactionIdDidCommit(xid) {
        return Verdict::Committed(xid);
    }
    Verdict::Abandoned(xid)
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
}

// ---------------------------------------------------------
// Ownership: who holds blocks, counted from headers or walked
// ---------------------------------------------------------

/// Something that owns blocks. [`owners`] lists them in the order the mark pass must read them.
enum Owner {
    /// A chain of pages from a metadata-page root, through `next_blockno`.
    Chain(pg_sys::BlockNumber),
    /// A bytes list, by its header.
    Bytes(pg_sys::BlockNumber),
    /// A list of segment entries, each owning lists of its own.
    Segments(LinkedItemList<SegmentMetaEntry>),
    /// Pages already read.
    Pages(Vec<pg_sys::BlockNumber>),
}

/// Every owner, lists before the FSM, which [`visit_fsm`] reads separately and last. A block
/// garbage collection moves from a dead segment's entry into the FSM mid-walk is then seen in
/// one place or the other; read the FSM first and a block freed between the two reads would be
/// in neither.
unsafe fn owners(
    indexrel: &PgSearchRelation,
    meta: &MetaPage,
    merge: &MergeListSnapshot,
) -> Vec<Owner> {
    let mut owners = meta.fixed_blocks().map(Owner::Chain).collect::<Vec<_>>();
    owners.push(Owner::Bytes(meta.schema_bytes().header_blockno));
    owners.push(Owner::Bytes(meta.settings_bytes().header_blockno));
    owners.push(Owner::Segments(meta.segment_metas()));
    if let Some(garbage) = meta.segment_metas_garbage() {
        owners.push(Owner::Segments(garbage));
    }
    owners.extend(
        merge
            .entries
            .iter()
            .map(|entry| Owner::Bytes(entry.segment_ids_start_blockno)),
    );
    owners.push(Owner::Pages(merge.pages.clone()));
    let _ = indexrel;
    owners
}

/// The merge list's entries and pages, read under the merge lock and kept after it. The lock is
/// an exclusive content lock on the merge-lock block, one of the metadata page's fixed chains,
/// so it must be released before those are read.
struct MergeListSnapshot {
    entries: Vec<MergeEntry>,
    pages: Vec<pg_sys::BlockNumber>,
}

impl MergeListSnapshot {
    unsafe fn take(indexrel: &PgSearchRelation, meta: &MetaPage) -> Self {
        let merge_lock = meta.acquire_merge_lock();
        let list =
            LinkedItemList::<MergeEntry>::open(indexrel, merge_lock.merge_list_header_blockno());
        Self {
            entries: list.list(None),
            pages: list.pages(),
        }
    }

    /// Whether a merge is writing output. Its blocks are unaccounted until it publishes, so the
    /// count gate cannot close and a walk would read every block for nothing.
    unsafe fn in_flight(&self, indexrel: &PgSearchRelation) -> bool {
        let mut bman = BufferManager::new(indexrel);
        self.entries
            .iter()
            .any(|entry| !entry.recyclable(&mut bman))
    }
}

/// Blocks every owner holds plus blocks the FSM holds or occupies, from list headers and the
/// FSM pages alone, for the count gate. No data page is read for a list whose header carries its
/// count; a list without one is walked and its header updated so the next count is one read.
///
/// The relation size is read by the caller after `MetaPage::open` and the merge-list snapshot,
/// since both create their lists lazily on first use.
unsafe fn accounted_blocks(
    indexrel: &PgSearchRelation,
    meta: &MetaPage,
    merge: &MergeListSnapshot,
    nblocks: pg_sys::BlockNumber,
) -> u64 {
    let bman = BufferManager::new(indexrel);
    let mut seen = BlockSet::new(nblocks);
    let mut total = 0;
    for owner in owners(indexrel, meta, merge) {
        total += count_owner(indexrel, &bman, &mut seen, owner);
    }
    let fsm = visit_fsm(indexrel, meta, &mut seen);
    total + fsm.pages + fsm.free
}

/// Every block some owner references plus every block the FSM holds or occupies.
unsafe fn referenced_or_free(
    indexrel: &PgSearchRelation,
    meta: &MetaPage,
    merge: &MergeListSnapshot,
    nblocks: pg_sys::BlockNumber,
) -> BlockSet {
    let bman = BufferManager::new(indexrel);
    let mut keep = BlockSet::new(nblocks);
    for owner in owners(indexrel, meta, merge) {
        mark_owner(indexrel, &bman, &mut keep, owner);
    }
    visit_fsm(indexrel, meta, &mut keep);
    keep
}

unsafe fn count_owner(
    indexrel: &PgSearchRelation,
    bman: &BufferManager,
    seen: &mut BlockSet,
    owner: Owner,
) -> u64 {
    match owner {
        Owner::Chain(start) => bman.chain(start).take_while(|b| seen.insert(*b)).count() as u64,
        Owner::Bytes(header) => bytes_list_pages(indexrel, header),
        Owner::Segments(mut list) => {
            let entries = list.list_pinned();
            let files: u64 = entries
                .iter()
                .flat_map(SegmentMetaEntry::owned_lists)
                .map(|owned| match owned {
                    OwnedList::Bytes(header) => bytes_list_pages(indexrel, header),
                    OwnedList::Items(header) => {
                        LinkedItemList::<MutableSegmentEntry>::open(indexrel, header)
                            .pages()
                            .len() as u64
                    }
                })
                .sum();
            files + list.pages().len() as u64
        }
        Owner::Pages(pages) => pages.len() as u64,
    }
}

unsafe fn mark_owner(
    indexrel: &PgSearchRelation,
    bman: &BufferManager,
    keep: &mut BlockSet,
    owner: Owner,
) {
    match owner {
        Owner::Chain(start) => bman
            .chain(start)
            .take_while(|b| keep.insert(*b))
            .for_each(drop),
        Owner::Bytes(header) => {
            for blockno in LinkedBytesList::open(indexrel, header).freeable_blocks() {
                keep.insert(blockno);
            }
        }
        Owner::Segments(mut list) => {
            let entries = list.list_pinned();
            for entry in entries.iter() {
                for blockno in entry.freeable_blocks(indexrel) {
                    keep.insert(blockno);
                }
            }
            drop(entries);
            for blockno in list.pages() {
                keep.insert(blockno);
            }
        }
        Owner::Pages(pages) => {
            for blockno in pages {
                keep.insert(blockno);
            }
        }
    }
}

/// A bytes list's page count from its header, or from a walk when the header carries none,
/// which then backfills the header. Only `finalize_and_write` keeps the count current, and
/// only it writes a mapping; a list written without one (schema, settings) is short and is
/// walked every time.
unsafe fn bytes_list_pages(
    indexrel: &PgSearchRelation,
    header_blockno: pg_sys::BlockNumber,
) -> u64 {
    let list = LinkedBytesList::open(indexrel, header_blockno);
    let data = list.get_linked_list_data();
    let finalized = data.blocklist_start != pg_sys::InvalidBlockNumber;
    if finalized && data.npages != 0 {
        return data.npages as u64;
    }
    let npages = list.freeable_blocks().count() as u32;
    if finalized {
        let mut bman = BufferManager::new(indexrel);
        // The header carries the block-list directory after its struct; `pd_lower` must stay.
        bman.get_buffer_mut(header_blockno)
            .page_mut()
            .contents_mut_in_place::<LinkedListData>()
            .npages = npages;
    }
    npages as u64
}

struct FsmVisit {
    /// Pages the FSM occupies, not seen before.
    pages: u64,
    /// Blocks the FSM holds as free, not seen before.
    free: u64,
}

/// Adds the FSM's pages and the blocks it holds to `set`.
unsafe fn visit_fsm(indexrel: &PgSearchRelation, meta: &MetaPage, set: &mut BlockSet) -> FsmVisit {
    let bman = BufferManager::new(indexrel);
    let mut visit = FsmVisit { pages: 0, free: 0 };
    V2FSM::open(meta.fsm()).visit(&bman, |block| match block {
        FsmBlock::Page(blockno) => {
            let new = set.insert(blockno);
            visit.pages += new as u64;
            new
        }
        FsmBlock::Free(blockno) => {
            visit.free += set.insert(blockno) as u64;
            true
        }
    });
    visit
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
            candidate
                .verdict
                .stamp()
                .unwrap_or(pg_sys::InvalidTransactionId),
            candidate.verdict.as_str().to_string(),
        )
    }))
}

/// Each owner's counted pages next to a walk of the same owner, to find where the header
/// counts and reality disagree.
#[cfg(any(test, feature = "pg_test"))]
pub(crate) unsafe fn accounting_report(indexrel: &PgSearchRelation) -> String {
    use std::fmt::Write;
    let meta = MetaPage::open(indexrel);
    let merge = MergeListSnapshot::take(indexrel, &meta);
    let nblocks = relation_blocks(indexrel);
    let bman = BufferManager::new(indexrel);
    let mut out = format!("nblocks={nblocks}\n");
    let mut seen = BlockSet::new(nblocks);
    let mut marked = BlockSet::new(nblocks);
    let mut marked_so_far = 0;
    for (counted, walked) in owners(indexrel, &meta, &merge)
        .into_iter()
        .zip(owners(indexrel, &meta, &merge))
    {
        let name = match &counted {
            Owner::Chain(start) => format!("chain {start}"),
            Owner::Bytes(header) => format!("bytes list {header}"),
            Owner::Segments(list) => format!("segment entries {}", list.header_blockno),
            Owner::Pages(pages) => format!("pages {pages:?}"),
        };
        let count = count_owner(indexrel, &bman, &mut seen, counted);
        mark_owner(indexrel, &bman, &mut marked, walked);
        let now = marked
            .words
            .iter()
            .map(|w| w.count_ones() as u64)
            .sum::<u64>();
        writeln!(
            out,
            "{name}: counted {count} walked {}",
            now - marked_so_far
        )
        .unwrap();
        marked_so_far = now;
    }
    let fsm = visit_fsm(indexrel, &meta, &mut seen);
    writeln!(out, "fsm pages={} free={}", fsm.pages, fsm.free).unwrap();
    for candidate in candidates(indexrel) {
        writeln!(out, "candidate {candidate:?}").unwrap();
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
            bman.get_buffer_mut(*blockno).set_alloc_xid(xid);
        }
    }

    pub(crate) unsafe fn stamp_of(
        indexrel: &PgSearchRelation,
        blockno: pg_sys::BlockNumber,
    ) -> pg_sys::TransactionId {
        BufferManager::new(indexrel)
            .get_buffer(blockno)
            .page()
            .alloc_xid()
            .expect("stamp of a written page")
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

    fn sorted_blocks(
        candidates: &[Candidate],
        verdict: impl Fn(Verdict) -> bool,
    ) -> Vec<pg_sys::BlockNumber> {
        let mut blocks = candidates
            .iter()
            .filter(|candidate| verdict(candidate.verdict))
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
        assert_eq!(
            sorted_blocks(&found, |v| matches!(v, Verdict::Recent(_))),
            blocks
        );
        assert_eq!(reclaim(&indexrel), 0);

        // Once it is behind the horizon, the blocks are abandoned.
        let found = candidates_as_of(&indexrel, next_xid());
        assert_eq!(
            sorted_blocks(&found, |v| matches!(v, Verdict::Abandoned(_))),
            blocks
        );
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
        assert_eq!(
            sorted_blocks(&found, |v| matches!(v, Verdict::Recent(_))),
            blocks
        );
        assert_eq!(found.len(), blocks.len(), "{found:?}");
        assert_eq!(reclaim(&indexrel), 0);

        // Behind the horizon the stamp is still in progress, which also keeps the blocks.
        let found = candidates_as_of(&indexrel, next_xid());
        assert_eq!(
            sorted_blocks(&found, |v| matches!(v, Verdict::InProgress(_))),
            blocks
        );
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
        assert!(
            committed >= pg_sys::FirstNormalTransactionId.into_inner(),
            "pg_extension row is frozen; borrow a committed id elsewhere"
        );
        stamp(&indexrel, &blocks, pg_sys::TransactionId::from(committed));

        let found = candidates(&indexrel);
        assert_eq!(
            sorted_blocks(&found, |v| matches!(v, Verdict::Committed(_))),
            blocks
        );
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
        assert_eq!(
            sorted_blocks(&found, |v| matches!(v, Verdict::Empty)),
            blocks
        );
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
        assert_eq!(
            sorted_blocks(&found, |v| matches!(v, Verdict::Freed)),
            blocks
        );
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
            let entries = metas.list_pinned();
            entries
                .iter()
                .flat_map(SegmentMetaEntry::owned_lists)
                .filter_map(|owned| match owned {
                    OwnedList::Bytes(header) => Some(header),
                    OwnedList::Items(_) => None,
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
            buffer.set_alloc_xid(pg_sys::InvalidTransactionId);
        }
        for header in &headers {
            bman.get_buffer_mut(*header)
                .page_mut()
                .contents_mut_in_place::<LinkedListData>()
                .npages = 0;
        }

        let found = candidates_as_of(&indexrel, next_xid());
        assert_eq!(
            sorted_blocks(&found, |v| matches!(v, Verdict::Unstamped(_))),
            leaked
        );
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
            let entries = metas.list_pinned();
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

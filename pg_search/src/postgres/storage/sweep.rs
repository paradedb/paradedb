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
//! transaction that allocated them is known to have aborted or crashed.
//!
//! A writer takes blocks from the FSM or by extending the relation, fills them, and links them
//! into a segment entry last. An ERROR or a crash before that link leaves the blocks full of data
//! with no owner. This module is VACUUM's answer, modelled on how heap and nbtree VACUUM treat
//! pages left behind by a crashed extension: the page says who allocated it
//! (`BM25PageSpecialData::alloc_xid`, written by `BufferMut::init_page`), and the allocator's
//! transaction state decides, so no writer is ever waited for.
//!
//! VACUUM first totals the blocks every structure owns from their list headers
//! (`LinkedListData::npages`) and the blocks the FSM holds. When the total equals the relation
//! size nothing is missing and the walk below is skipped. Otherwise a block is a candidate when
//! no segment, list, or FSM page references it and the FSM does not hold it. A candidate whose
//! allocator is in progress is still being written. One whose allocator committed was abandoned
//! by a caller that swallowed an ERROR, which is a bug to fix at that caller, so it is reported
//! and left alone. One whose allocator neither committed nor runs any more aborted or crashed
//! and is freed.
//!
//! Reachability is not optional. Segment entries carry no creator id, so a segment published by
//! a transaction that later rolled back stays linked and keeps serving. Its blocks carry an
//! aborted stamp and must not be freed.
//!
//! A block reserved by `new_buffers` but never initialised still shows its previous content, so
//! the sweep keeps it. While the process lives, `storage::unpublished` returns such blocks.

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{
    BM25PageSpecialData, LinkedList, LinkedListData, MVCCEntry, SegmentMetaEntry,
    SegmentMetaEntryContent,
};
use crate::postgres::storage::buffer::{BufferManager, PinnedBuffer};
use crate::postgres::storage::fsm::FreeSpaceManager;
use crate::postgres::storage::fsm::v2::{FsmBlock, V2FSM};
use crate::postgres::storage::merge::MergeEntry;
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use pgrx::iter::TableIterator;
use pgrx::{PgRelation, check_for_interrupts, name, pg_extern, pg_sys};

/// What the allocation stamp says about an unreferenced block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The allocating transaction is still running and may publish the block.
    InProgress,
    /// The allocating transaction committed without publishing. Never freed: the caller that
    /// swallowed its ERROR must be fixed.
    Committed,
    /// The allocating transaction aborted or crashed. Safe to free.
    Abandoned,
    /// Written before pages carried a stamp, or by a parallel worker without an xid.
    Unstamped,
    /// Freed by an earlier sweep, so the FSM holds it or a writer just took it.
    Freed,
    /// Older than the oldest transaction the commit log still answers for.
    BeyondClog,
}

impl Verdict {
    fn as_str(self) -> &'static str {
        match self {
            Verdict::InProgress => "in_progress",
            Verdict::Committed => "committed",
            Verdict::Abandoned => "abandoned",
            Verdict::Unstamped => "unstamped",
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
pub unsafe fn unaccounted_blocks(indexrel: &PgSearchRelation) -> i64 {
    let (accounted, nblocks) = accounted_blocks(indexrel, false);
    nblocks as i64 - accounted as i64
}

/// Every block that is neither referenced nor held by the FSM, with its verdict. Read-only.
pub unsafe fn candidates(indexrel: &PgSearchRelation) -> Vec<Candidate> {
    candidates_in(indexrel, relation_blocks(indexrel))
}

/// Frees every `Abandoned` candidate and returns how many. Skips the walk entirely when the
/// accounting says no block is missing.
pub unsafe fn reclaim(indexrel: &PgSearchRelation) -> usize {
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

    let abandoned = candidates_in(indexrel, nblocks)
        .into_iter()
        .filter(|candidate| candidate.verdict == Verdict::Abandoned)
        .map(|candidate| candidate.blockno)
        .collect::<Vec<_>>();
    if abandoned.is_empty() {
        return 0;
    }

    // Stamp first, then extend. A crash between the two leaves a Frozen page outside the FSM,
    // which every later sweep keeps. The other order could leave an aborted stamp on a page the
    // FSM already hands out, and a later sweep reading that stale stamp would free it twice.
    let mut bman = BufferManager::new(indexrel);
    for blockno in &abandoned {
        let mut buffer = bman.get_buffer_mut(*blockno);
        buffer
            .page_mut()
            .special_mut::<BM25PageSpecialData>()
            .alloc_xid = pg_sys::FrozenTransactionId;
    }
    bman.fsm().extend(&mut bman, abandoned.iter().copied());
    abandoned.len()
}

unsafe fn relation_blocks(indexrel: &PgSearchRelation) -> pg_sys::BlockNumber {
    pg_sys::RelationGetNumberOfBlocksInFork(indexrel.as_ptr(), pg_sys::ForkNumber::MAIN_FORKNUM)
}

unsafe fn candidates_in(
    indexrel: &PgSearchRelation,
    nblocks: pg_sys::BlockNumber,
) -> Vec<Candidate> {
    let keep = referenced_or_free(indexrel, nblocks);
    let bman = BufferManager::new(indexrel);
    let oldest_clog_xid = oldest_clog_xid();

    (0..nblocks)
        .filter(|blockno| !keep.contains(*blockno))
        .map(|blockno| {
            check_for_interrupts!();
            let alloc_xid = alloc_xid(&bman, blockno);
            Candidate {
                blockno,
                alloc_xid,
                verdict: verdict(alloc_xid, oldest_clog_xid),
            }
        })
        .collect()
}

unsafe fn alloc_xid(bman: &BufferManager, blockno: pg_sys::BlockNumber) -> pg_sys::TransactionId {
    let buffer = bman.get_buffer(blockno);
    let page = buffer.page();
    // An all-zero page is a relation extension whose first write never happened.
    if page.header().pd_upper == 0 {
        return pg_sys::InvalidTransactionId;
    }
    page.special::<BM25PageSpecialData>().alloc_xid
}

/// PostgreSQL's rule from `HeapTupleSatisfiesVacuum`: ask the proc array before the commit log,
/// and treat "neither running nor committed" as aborted, because a crashed transaction never
/// wrote its abort record.
unsafe fn verdict(xid: pg_sys::TransactionId, oldest_clog_xid: pg_sys::TransactionId) -> Verdict {
    if xid == pg_sys::FrozenTransactionId {
        return Verdict::Freed;
    }
    if xid.into_inner() < pg_sys::FirstNormalTransactionId.into_inner() {
        return Verdict::Unstamped;
    }
    if pg_sys::TransactionIdPrecedes(xid, oldest_clog_xid) {
        return Verdict::BeyondClog;
    }
    if pg_sys::TransactionIdIsInProgress(xid) {
        return Verdict::InProgress;
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
    let merge_lock = meta.acquire_merge_lock();
    let merge_list =
        LinkedItemList::<MergeEntry>::open(indexrel, merge_lock.merge_list_header_blockno());
    let segment_id_lists = merge_list
        .list(None)
        .iter()
        .map(|entry| entry.segment_ids_start_blockno)
        .collect();
    (segment_id_lists, merge_list.pages())
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
        header.page_mut().contents_mut::<LinkedListData>().npages = npages;
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

    V2FSM::open(meta.fsm()).visit(&bman, |block| match block {
        FsmBlock::Page(blockno) => {
            keep.insert(blockno);
            walked.insert(blockno)
        }
        FsmBlock::Free(blockno) => keep.insert(blockno),
    });

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

    keep
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
// SQL
// ---------------------------------------------------------

#[pg_extern(name = "unaccounted_blocks")]
unsafe fn unaccounted_blocks_sql(index: PgRelation) -> i64 {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    unaccounted_blocks(&index)
}

#[pg_extern]
unsafe fn unreferenced_blocks(
    index: PgRelation,
) -> TableIterator<
    'static,
    (
        name!(blockno, i64),
        name!(alloc_xid, pg_sys::TransactionId),
        name!(state, String),
    ),
> {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    TableIterator::new(candidates(&index).into_iter().map(|candidate| {
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

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
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
        Spi::run(&format!(
            "CREATE INDEX {table}_idx ON {table} USING bm25 (id, data) WITH (key_field = 'id')"
        ))
        .unwrap();
        // Rows inserted after the build land in a mutable segment.
        Spi::run(&format!(
            "INSERT INTO {table} (data) SELECT 'late ' || g FROM generate_series(1, 200) g"
        ))
        .unwrap();
        let oid = Spi::get_one::<pg_sys::Oid>(&format!("SELECT '{table}_idx'::regclass::oid"))
            .unwrap()
            .unwrap();
        PgSearchRelation::open(oid)
    }

    /// Runs `f` in a subtransaction that is then rolled back, so whatever it allocated carries
    /// an aborted stamp.
    unsafe fn rolled_back<T>(f: impl FnOnce() -> T) -> T {
        let context = pg_sys::CurrentMemoryContext;
        // Ending the subtransaction leaves the top transaction's owner current, not the
        // portal's that opened the test relation.
        let resource_owner = pg_sys::CurrentResourceOwner;
        pg_sys::BeginInternalSubTransaction(std::ptr::null());
        pg_sys::MemoryContextSwitchTo(context);
        let result = f();
        pg_sys::RollbackAndReleaseCurrentSubTransaction();
        pg_sys::MemoryContextSwitchTo(context);
        pg_sys::CurrentResourceOwner = resource_owner;
        result
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
        let blocks = rolled_back(|| allocate_unpublished(&indexrel));
        assert_eq!(unaccounted_blocks(&indexrel), blocks.len() as i64);

        let found = candidates(&indexrel);
        assert_eq!(sorted_blocks(&found, Verdict::Abandoned), blocks);
        assert_eq!(found.len(), blocks.len(), "{found:?}");

        assert_eq!(reclaim(&indexrel), blocks.len());
        assert!(candidates(&indexrel).is_empty());
        assert_eq!(unaccounted_blocks(&indexrel), 0);

        let bman = BufferManager::new(&indexrel);
        for blockno in blocks {
            assert_eq!(
                bman.get_buffer(blockno)
                    .page()
                    .special::<BM25PageSpecialData>()
                    .alloc_xid,
                pg_sys::FrozenTransactionId
            );
        }
    }

    #[pg_test]
    unsafe fn in_progress_allocation_is_kept() {
        let indexrel = test_index("sweep_in_progress");
        let blocks = allocate_unpublished(&indexrel);
        assert_eq!(unaccounted_blocks(&indexrel), blocks.len() as i64);

        let found = candidates(&indexrel);
        assert_eq!(sorted_blocks(&found, Verdict::InProgress), blocks);
        assert_eq!(found.len(), blocks.len(), "{found:?}");
        assert_eq!(reclaim(&indexrel), 0);
    }

    /// A segment published inside a rolled-back subtransaction stays linked, so its blocks carry
    /// an aborted stamp but must never be freed.
    #[pg_test]
    unsafe fn rolled_back_insert_keeps_its_segment() {
        let indexrel = test_index("sweep_rollback");
        rolled_back(|| {
            Spi::run(
                "INSERT INTO sweep_rollback (data) SELECT 'undone ' || g FROM generate_series(1, 300) g",
            )
            .unwrap();
        });

        assert_eq!(unaccounted_blocks(&indexrel), 0);
        let found = candidates(&indexrel);
        assert!(found.is_empty(), "{found:?}");
        assert_eq!(reclaim(&indexrel), 0);

        let visible = Spi::get_one::<i64>(
            "SELECT count(*) FROM sweep_rollback WHERE sweep_rollback @@@ 'data:undone'",
        )
        .unwrap()
        .unwrap();
        assert_eq!(visible, 0);
    }
}

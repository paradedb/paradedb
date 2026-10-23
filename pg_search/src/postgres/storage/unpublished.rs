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

//! Output files a merge has started writing but not yet published.
//!
//! Track all private component allocations, including unconsumed buffers and unattached mapping
//! pages. Publication transfers ownership at the atomic segment-list switch. Abandoned blocks
//! return to the FSM in a clean transaction after a cooperative stop, from the parent transaction
//! when the merge payload fails inside [`reclaiming_on_error`], or during `before_shmem_exit`
//! after cancellation/shutdown.
//!
//! This process-local registry does not survive crashes; `storage::sweep` finds what it missed.
//! Exit reclamation also skips an index locked by DDL rather than delaying shutdown.

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::buffer::BufferManager;
use crate::postgres::storage::fsm::FreeSpaceManager;
use crate::postgres::utils::ExitWorkScope;
use parking_lot::Mutex;
use pgrx::{PgTryBuilder, pg_guard, pg_sys};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
struct UnpublishedFile {
    index_oid: pg_sys::Oid,

    /// The index's physical storage identifier when the file was written. `REINDEX` and
    /// `TRUNCATE` give the index new storage, and a chain recorded against the old storage must
    /// never be returned to the new one.
    relfilenumber: pg_sys::Oid,

    /// The first allocation is the component header, followed by all reserved output pages.
    blocks: Mutex<Vec<pg_sys::BlockNumber>>,
}

#[derive(Clone, Debug)]
pub(crate) struct Allocation(Arc<UnpublishedFile>);

impl Allocation {
    pub(crate) fn reserve(&self, count: usize) {
        self.0.blocks.lock().reserve(count);
    }

    pub(crate) fn record(&self, blockno: pg_sys::BlockNumber) {
        let mut blocks = self.0.blocks.lock();
        assert!(
            blocks.len() < blocks.capacity(),
            "allocation capacity must be reserved before allocating blocks"
        );
        blocks.push(blockno);
    }
}

static UNPUBLISHED: Mutex<Vec<Allocation>> = Mutex::new(Vec::new());

fn relfilenumber(rel: &PgSearchRelation) -> pg_sys::Oid {
    #[cfg(feature = "pg15")]
    {
        rel.rd_node.relNode
    }
    #[cfg(not(feature = "pg15"))]
    {
        rel.rd_locator.relNumber
    }
}

static EXIT_CALLBACK_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Establish ownership before allocating the first block of a private output file.
pub(crate) fn track(rel: &PgSearchRelation) -> Allocation {
    // Once per process, and before anything is registered, so the callback can never miss a
    // chain. Registered lazily rather than in `_PG_init` so only processes that merge pay for it.
    if !EXIT_CALLBACK_REGISTERED.load(Ordering::Relaxed) {
        unsafe { pg_sys::before_shmem_exit(Some(reclaim_at_exit), pg_sys::Datum::from(0)) };
        EXIT_CALLBACK_REGISTERED.store(true, Ordering::Relaxed);
    }
    let allocation = Allocation(Arc::new(UnpublishedFile {
        index_oid: rel.oid(),
        relfilenumber: relfilenumber(rel),
        blocks: Mutex::new(Vec::new()),
    }));
    UNPUBLISHED.lock().push(allocation.clone());
    allocation
}

pub(crate) struct Publication(Vec<Allocation>);

impl Publication {
    pub(crate) fn publish(self) {
        UNPUBLISHED.lock().retain(|file| {
            !self
                .0
                .iter()
                .any(|published| Arc::ptr_eq(&file.0, &published.0))
        });
    }
}

/// Prepare an acknowledgment that performs no allocations at the atomic publication point.
pub(crate) fn prepare_publication(
    rel: &PgSearchRelation,
    header_blocknos: impl IntoIterator<Item = pg_sys::BlockNumber>,
) -> Publication {
    let index_oid = rel.oid();
    let registry = UNPUBLISHED.lock();
    let mut files = Vec::new();
    for header_blockno in header_blocknos {
        files.extend(
            registry
                .iter()
                .filter(|file| {
                    file.0.index_oid == index_oid
                        && file.0.relfilenumber == relfilenumber(rel)
                        && file.0.blocks.lock().first() == Some(&header_blockno)
                })
                .cloned(),
        );
    }
    Publication(files)
}

/// Number of unpublished files recorded for `index_oid`.
#[cfg(any(test, feature = "pg_test"))]
fn pending(index_oid: pg_sys::Oid) -> usize {
    UNPUBLISHED
        .lock()
        .iter()
        .filter(|file| file.0.index_oid == index_oid)
        .count()
}

/// Runs `f` in a subtransaction and returns `rel`'s unpublished blocks to the FSM if it fails.
///
/// On success the subtransaction is released and its locks pass to the parent. On an ERROR or
/// an `Err`, the subtransaction is rolled back, which releases the pins and locks the failed code
/// still held, the blocks recorded for `rel` are returned from the now healthy parent, and the
/// error continues to the caller. This is plpgsql's EXCEPTION block.
///
/// A subtransaction's resource owner tracks only what was acquired under it, so every buffer
/// pin, relation reference and snapshot `f` takes must be released before it returns, and `f`
/// must not release one taken before the scope: either is an ERROR. In parallel mode, where
/// subtransactions are not allowed, `f` runs directly and a failure leaves its blocks to the exit
/// callback or `storage::sweep`.
pub(crate) unsafe fn reclaiming_on_error<T>(
    rel: &PgSearchRelation,
    f: impl FnOnce() -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    if pg_sys::IsInParallelMode() || !pg_sys::IsTransactionState() {
        return f();
    }

    let outer = OuterState {
        interrupt_holdoff: pg_sys::InterruptHoldoffCount,
        memory_context: pg_sys::CurrentMemoryContext,
        resource_owner: pg_sys::CurrentResourceOwner,
    };
    pg_sys::BeginInternalSubTransaction(std::ptr::null());
    pg_sys::MemoryContextSwitchTo(outer.memory_context);

    // A caught ERROR is handled inside the handler, as a PG_CATCH block would: `PgTryBuilder`
    // flushes the error state when a handler returns, and the rethrow needs it intact.
    let rel = AssertUnwindSafe(rel);
    let result = PgTryBuilder::new(AssertUnwindSafe(f))
        .catch_others(|caught| {
            outer.leave_subtransaction(true);
            reclaim(*rel);
            caught.rethrow()
        })
        .execute();

    outer.leave_subtransaction(result.is_err());
    if result.is_err() {
        reclaim(*rel);
    }
    result
}

/// What the subtransaction replaces and `leave_subtransaction` puts back.
#[derive(Clone, Copy)]
struct OuterState {
    interrupt_holdoff: u32,
    memory_context: pg_sys::MemoryContext,
    resource_owner: pg_sys::ResourceOwner,
}

impl OuterState {
    unsafe fn leave_subtransaction(&self, rollback: bool) {
        pg_sys::MemoryContextSwitchTo(self.memory_context);
        if rollback {
            pg_sys::RollbackAndReleaseCurrentSubTransaction();
        } else {
            pg_sys::ReleaseCurrentSubTransaction();
        }
        pg_sys::MemoryContextSwitchTo(self.memory_context);
        pg_sys::CurrentResourceOwner = self.resource_owner;
        pg_sys::InterruptHoldoffCount = self.interrupt_holdoff;
    }
}

/// Return this process's private output allocations for `rel` to its FSM, in one batch.
/// Returns the number of files reclaimed.
///
/// Must be called in a transaction with no ERROR in flight: an ERROR leaves buffer locks and
/// pins behind until the transaction aborts. Reclamation modifies the FSM in a clean transaction.
///
/// # Safety
///
/// Every recorded allocation must be unreachable from the index. Publication must acknowledge
/// the ownership transfer at the atomic metadata switch.
pub unsafe fn reclaim(rel: &PgSearchRelation) -> usize {
    let index_oid = rel.oid();
    let current = relfilenumber(rel);

    // Out of the registry before the FSM is touched: an ERROR in the FSM must not let a later
    // reclaim or the exit callback return the same block twice. Blocks a failed extend leaves
    // outside the FSM are found by `storage::sweep`.
    let mine = {
        let mut registry = UNPUBLISHED.lock();
        let (mine, others) = registry
            .drain(..)
            .partition::<Vec<_>, _>(|file| file.0.index_oid == index_oid);
        *registry = others;
        mine
    };

    let mut blocks = Vec::new();
    let mut freed = 0;
    for file in mine {
        // The storage this chain lived in is gone, and so is the chain.
        if file.0.relfilenumber != current {
            continue;
        }
        blocks.append(&mut file.0.blocks.lock());
        freed += 1;
    }
    if !blocks.is_empty() {
        #[cfg(feature = "pg_test")]
        crate::postgres::storage::coordination_test::failpoint(c"pg_search.test_reclaim_fail");
        let mut bman = BufferManager::new(rel);
        bman.fsm().extend(&mut bman, blocks.into_iter());
    }
    if freed > 0 {
        pgrx::debug1!(
            "returned {freed} unpublished segment files of index {} to the FSM",
            rel.oid().to_u32()
        );
    }
    freed
}

/// Reclaim by OID, forgetting replaced/dropped storage and skipping an index locked by DDL.
/// Exit cleanup must not wait behind a `REINDEX` whose holder may be terminating this backend.
unsafe fn reclaim_by_oid(index_oid: pg_sys::Oid) -> usize {
    let lockmode = pg_sys::AccessShareLock as pg_sys::LOCKMODE;
    if !pg_sys::ConditionalLockRelationOid(index_oid, lockmode) {
        return 0;
    }
    match PgSearchRelation::try_open(index_oid, pg_sys::NoLock as pg_sys::LOCKMODE) {
        Some(rel) => reclaim(&rel),
        None => {
            UNPUBLISHED
                .lock()
                .retain(|file| file.0.index_oid != index_oid);
            0
        }
    }
}

/// Reclaim before shared-memory detachment. FATAL does not unwind, so first abort the
/// interrupted transaction to release its pins and locks, then start a clean transaction,
/// as PostgreSQL's `RemoveTempRelationsCallback` does. Only fresh buffer cleanup runs under
/// [`ExitWorkScope`], after that abort.
///
/// ERROR during exit becomes FATAL and continues with the remaining exit callbacks; this
/// callback is not retried, so reclamation is best-effort.
#[pg_guard]
unsafe extern "C-unwind" fn reclaim_at_exit(_code: core::ffi::c_int, _arg: pg_sys::Datum) {
    let index_oids = {
        // A FATAL cannot land while the registry is locked, since nothing under the lock checks
        // for interrupts, but there is no reason to risk an exit on it either.
        let Some(registry) = UNPUBLISHED.try_lock() else {
            return;
        };
        let mut index_oids = registry
            .iter()
            .map(|file| file.0.index_oid)
            .collect::<Vec<_>>();
        index_oids.sort_unstable_by_key(|oid| oid.to_u32());
        index_oids.dedup();
        index_oids
    };
    if index_oids.is_empty() {
        return;
    }

    pg_sys::AbortOutOfAnyTransaction();
    pg_sys::StartTransactionCommand();
    pg_sys::PushActiveSnapshot(pg_sys::GetTransactionSnapshot());
    {
        let _scope = ExitWorkScope::enter();
        for index_oid in index_oids {
            reclaim_by_oid(index_oid);
        }
    }
    pg_sys::PopActiveSnapshot();
    pg_sys::CommitTransactionCommand();
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::index::mvcc::MvccSatisfies;
    use crate::index::writer::index::{Mergeable, SearchIndexMerger};
    use pgrx::pg_sys::panic::CaughtError;
    use pgrx::prelude::*;
    use std::io::Write;
    use tantivy::Directory;
    use tantivy::index::SegmentId;

    unsafe fn abort_subtransaction(action: impl FnOnce()) {
        let interrupt_holdoff = pg_sys::InterruptHoldoffCount;
        assert_eq!(
            interrupt_holdoff, 0,
            "subtransaction fixture must not hold LWLocks"
        );
        let memory_context = pg_sys::CurrentMemoryContext;
        let resource_owner = pg_sys::CurrentResourceOwner;
        pg_sys::BeginInternalSubTransaction(std::ptr::null());
        let error = pgrx::PgTryBuilder::new(std::panic::AssertUnwindSafe(|| {
            action();
            None
        }))
        .catch_others(|error| match error {
            pg_sys::panic::CaughtError::ErrorReport(report)
            | pg_sys::panic::CaughtError::PostgresError(report) => {
                Some(report.message().to_string())
            }
            other => other.rethrow(),
        })
        .execute();
        pg_sys::MemoryContextSwitchTo(memory_context);
        pg_sys::RollbackAndReleaseCurrentSubTransaction();
        pg_sys::MemoryContextSwitchTo(memory_context);
        pg_sys::CurrentResourceOwner = resource_owner;
        pg_sys::InterruptHoldoffCount = interrupt_holdoff;
        let error = error.expect("the injected error must occur");
        assert!(
            error == "abandon allocated batch"
                || error.starts_with("injected output ownership failure:"),
            "unexpected error: {error}"
        );
    }

    fn allocated_blocks(index_oid: pg_sys::Oid) -> usize {
        UNPUBLISHED
            .lock()
            .iter()
            .filter(|file| file.0.index_oid == index_oid)
            .map(|file| file.0.blocks.lock().len())
            .sum()
    }

    unsafe fn abandon_batch(rel: &PgSearchRelation, count: usize) {
        abort_subtransaction(|| {
            let mut bman = BufferManager::new_tracked(rel);
            let mut buffers = bman.new_buffers(count);
            drop(buffers.next().unwrap());
            assert_eq!(allocated_blocks(rel.oid()), count);
            pgrx::error!("abandon allocated batch");
        });
    }

    #[pg_test]
    unsafe fn test_unconsumed_allocations_are_reclaimed() {
        let rel = setup("unpublished_batch", "1gb");
        let blocks_before = relation_blocks(&rel);
        let free_before = free_blocks(&rel);
        abandon_batch(&rel, 64);
        assert_eq!(relation_blocks(&rel) - blocks_before, 64);
        assert_eq!(allocated_blocks(rel.oid()), 64);
        assert_eq!(reclaim(&rel), 1);
        assert_eq!(free_blocks(&rel) - free_before, 64);
        assert_eq!(pending(rel.oid()), 0);

        let blocks_before_reuse = relation_blocks(&rel);
        abandon_batch(&rel, 64);
        assert_eq!(relation_blocks(&rel), blocks_before_reuse);
        assert_eq!(free_blocks(&rel), free_before);
        assert_eq!(reclaim(&rel), 1);
        assert_eq!(free_blocks(&rel) - free_before, 64);
        assert_eq!(reclaim(&rel), 0);
    }

    #[pg_test]
    unsafe fn test_reclamation_error_leaves_blocks_to_the_sweep() {
        use crate::postgres::storage::linked_bytes::LinkedBytesList;
        use crate::postgres::storage::sweep;
        let rel = setup("unpublished_retry", "1gb");
        let free_before = free_blocks(&rel);
        abort_subtransaction(|| {
            let mut writer = LinkedBytesList::create_tracked(&rel).writer();
            writer.write_all(&vec![7; 64 * 8192]).unwrap();
            writer.finalize_and_write().unwrap();
            pgrx::error!("abandon allocated batch");
        });
        let allocated = allocated_blocks(rel.oid()) as i64;
        assert!(allocated > 64);
        abort_subtransaction(|| {
            Spi::run("SET LOCAL pg_search.test_reclaim_fail = 'on'").unwrap();
            reclaim(&rel);
        });
        // Taken out of the registry before the FSM was touched: nothing returns them twice.
        assert_eq!(allocated_blocks(rel.oid()), 0);
        assert_eq!(reclaim(&rel), 0);
        assert_eq!(free_blocks(&rel), free_before);
        // Allocated by an aborted subtransaction and referenced by nothing: the sweep's case.
        assert_eq!(sweep::unaccounted_blocks(&rel), allocated);
        assert_eq!(sweep::reclaim(&rel) as i64, allocated);
        assert_eq!(free_blocks(&rel) - free_before, allocated);
        assert_eq!(sweep::unaccounted_blocks(&rel), 0);
    }

    #[pg_test]
    unsafe fn test_failed_scope_returns_output_before_the_error_leaves() {
        use crate::postgres::storage::linked_bytes::LinkedBytesList;
        let rel = setup("unpublished_scope", "1gb");
        let written = std::cell::Cell::new(0);
        // The FSM count once the file is written: the write may have drained blocks a previous
        // case returned, so the count before the write is not a baseline.
        let free_after_write = std::cell::Cell::new(0);
        let write = || {
            let mut writer = LinkedBytesList::create_tracked(&rel).writer();
            writer.write_all(&vec![7; 64 * 8192]).unwrap();
            writer.finalize_and_write().unwrap();
            written.set(allocated_blocks(rel.oid()) as i64);
            free_after_write.set(free_blocks(&rel));
        };

        // A Rust error: the scope returns `Err` after returning the blocks.
        let result = reclaiming_on_error(&rel, || -> anyhow::Result<()> {
            write();
            anyhow::bail!("merge failed")
        });
        assert_eq!(result.unwrap_err().to_string(), "merge failed");
        assert_eq!(allocated_blocks(rel.oid()), 0);
        assert_eq!(free_blocks(&rel), free_after_write.get() + written.get());

        // A PostgreSQL ERROR: the scope rolls back, returns the blocks, and rethrows.
        let message = PgTryBuilder::new(AssertUnwindSafe(|| {
            reclaiming_on_error(&rel, || -> anyhow::Result<()> {
                write();
                pgrx::error!("abandon allocated batch");
            })
            .unwrap();
            None
        }))
        .catch_others(|error| match error {
            CaughtError::ErrorReport(report) | CaughtError::PostgresError(report) => {
                Some(report.message().to_string())
            }
            other => other.rethrow(),
        })
        .execute();
        assert_eq!(message.as_deref(), Some("abandon allocated batch"));
        assert_eq!(allocated_blocks(rel.oid()), 0);
        assert_eq!(free_blocks(&rel), free_after_write.get() + written.get());
        assert!(pg_sys::IsTransactionState());
        let interrupt_holdoff = pg_sys::InterruptHoldoffCount;
        assert_eq!(interrupt_holdoff, 0);

        // Success: the subtransaction is released and nothing is returned.
        reclaiming_on_error(&rel, || -> anyhow::Result<()> {
            write();
            Ok(())
        })
        .unwrap();
        assert_eq!(free_blocks(&rel), free_after_write.get());
        assert_eq!(allocated_blocks(rel.oid()) as i64, written.get());
        assert_eq!(reclaim(&rel), 1);
    }

    #[pg_test]
    unsafe fn test_unattached_mapping_allocation_is_reclaimed() {
        use crate::postgres::storage::linked_bytes::LinkedBytesList;
        let rel = setup("unpublished_mapping", "1gb");
        let blocks_before = relation_blocks(&rel);
        let free_before = free_blocks(&rel);
        let data_blocks = std::cell::Cell::new(0);
        abort_subtransaction(|| {
            let mut writer = LinkedBytesList::create_tracked(&rel).writer();
            writer.write_all(&vec![7; 300 * 8192]).unwrap();
            data_blocks.set(allocated_blocks(rel.oid()));
            Spi::run("SET LOCAL pg_search.test_mapping_fail = 'on'").unwrap();
            writer.finalize_and_write().unwrap();
        });
        let allocated = allocated_blocks(rel.oid());
        assert_eq!(allocated, data_blocks.get() + 1);
        assert_eq!(relation_blocks(&rel) - blocks_before, allocated as i64);
        assert_eq!(reclaim(&rel), 1);
        assert_eq!(free_blocks(&rel) - free_before, allocated as i64);
        assert_eq!(pending(rel.oid()), 0);
    }

    #[pg_test]
    unsafe fn test_publication_error_transfers_only_published_output() {
        let rel = setup("unpublished_handoff", "1gb");
        Spi::run(
            "SET paradedb.global_mutable_segment_rows = 0;
             INSERT INTO unpublished_handoff SELECT g, 'silver dragon ' || g FROM generate_series(1, 200) g;
             INSERT INTO unpublished_handoff SELECT g, 'golden dragon ' || g FROM generate_series(201, 400) g;
             RESET paradedb.global_mutable_segment_rows;",
        ).unwrap();
        pg_sys::CommandCounterIncrement();

        for (setting, published) in [
            ("pg_search.test_publication_before_fail", false),
            ("pg_search.test_publication_after_fail", true),
        ] {
            abort_subtransaction(|| {
                Spi::run(&format!("SET LOCAL {setting} = 'on'")).unwrap();
                let mut merger = SearchIndexMerger::open(&rel, MvccSatisfies::Mergeable).unwrap();
                let segment_ids = merger.all_entries().keys().copied().collect::<Vec<_>>();
                assert!(segment_ids.len() >= 2);
                merger.merge_segments(&segment_ids).unwrap();
            });
            let tracked = pending(rel.oid());
            if published {
                assert_eq!(tracked, 0);
                assert_eq!(reclaim(&rel), 0);
            } else {
                assert!(tracked > 0);
                assert_eq!(reclaim(&rel), tracked);
            }
            let count: i64 =
                Spi::get_one("SELECT count(*) FROM unpublished_handoff WHERE title @@@ 'dragon'")
                    .unwrap()
                    .unwrap();
            assert_eq!(count, 400);
            let entries = crate::postgres::storage::metadata::MetaPage::open(&rel)
                .segment_metas()
                .list(None)
                .into_iter()
                .filter(|entry| entry.xmax() == pg_sys::InvalidTransactionId)
                .count();
            assert_eq!(entries, if published { 1 } else { 2 });
        }
    }

    /// `layer_sizes` controls whether the inserts below merge on their own: a tiny layer lets a
    /// foreground merge run, a huge one keeps every insert's segment separate so a test can merge
    /// them itself. No background layers keeps the background merger out of it.
    unsafe fn setup(table: &str, layer_sizes: &str) -> PgSearchRelation {
        Spi::run(&format!(
            "CREATE TABLE {table} (id BIGINT PRIMARY KEY, title TEXT NOT NULL);
             CREATE INDEX {table}_idx ON {table} USING paradedb (id, title)
             WITH (target_segment_count = 1, layer_sizes = '{layer_sizes}', background_layer_sizes = '0');",
        ))
        .expect("fixture setup");
        pg_sys::CommandCounterIncrement();

        let oid: pg_sys::Oid = Spi::get_one(&format!("SELECT '{table}_idx'::regclass::oid"))
            .expect("spi should succeed")
            .unwrap();
        PgSearchRelation::open(oid)
    }

    unsafe fn free_blocks(rel: &PgSearchRelation) -> i64 {
        Spi::get_one(&format!(
            "SELECT count(*) FROM paradedb.fsm_info({}::regclass)",
            rel.oid().to_u32()
        ))
        .expect("spi should succeed")
        .unwrap()
    }

    unsafe fn relation_blocks(rel: &PgSearchRelation) -> i64 {
        Spi::get_one(&format!(
            "SELECT pg_relation_size({}::regclass) / current_setting('block_size')::bigint",
            rel.oid().to_u32()
        ))
        .expect("spi should succeed")
        .unwrap()
    }

    /// Dropping an unfinished writer leaves every allocation registered for reclamation.
    #[pg_test]
    unsafe fn test_unpublished_file_is_returned_to_fsm() {
        let rel = setup("unpublished_drop", "1kb");
        let mut directory = MvccSatisfies::Mergeable.directory(&rel);
        directory.track_unpublished_files();

        let free_before = free_blocks(&rel);
        let blocks_before = relation_blocks(&rel);

        let path = format!("{}.pos", SegmentId::generate_random().uuid_string());
        let mut writer = directory
            .open_write(std::path::Path::new(&path))
            .expect("open_write");
        writer
            .write_all(&vec![7u8; 64 * 1024])
            .expect("write should succeed");
        writer.flush().expect("flush should succeed");
        drop(writer);

        assert_eq!(pending(rel.oid()), 1, "the open file is tracked");
        let blocks_after_write = relation_blocks(&rel);
        let allocated = blocks_after_write - blocks_before;
        assert!(
            allocated >= 9,
            "a 64kb file takes a header plus data pages: {allocated}"
        );

        assert_eq!(reclaim(&rel), 1, "one chain reclaimed");
        assert_eq!(pending(rel.oid()), 0, "nothing left tracked");
        assert_eq!(
            free_blocks(&rel) - free_before,
            allocated,
            "every block the file took is back in the FSM"
        );
        assert_eq!(reclaim(&rel), 0, "a second reclaim finds nothing");
    }

    /// A merge that is cancelled cooperatively, through tantivy's cancel check rather than a
    /// Postgres ERROR, leaves its partial output tracked, and reclaiming returns it to the FSM.
    #[pg_test]
    unsafe fn test_cancelled_merge_output_is_reclaimed() {
        let rel = setup("unpublished_cancel", "1gb");
        Spi::run(
            "SET paradedb.global_mutable_segment_rows = 0;
             INSERT INTO unpublished_cancel SELECT g, 'silver dragon ' || g FROM generate_series(1, 2000) g;
             INSERT INTO unpublished_cancel SELECT g, 'golden dragon ' || g FROM generate_series(2001, 4000) g;
             RESET paradedb.global_mutable_segment_rows;",
        )
        .expect("fixture rows");
        pg_sys::CommandCounterIncrement();
        assert_eq!(
            pending(rel.oid()),
            0,
            "a merge that finishes leaves nothing tracked"
        );

        let mut merger =
            SearchIndexMerger::open(&rel, MvccSatisfies::Mergeable).expect("open merger");
        let segment_ids = merger.all_entries().keys().copied().collect::<Vec<_>>();
        assert!(
            segment_ids.len() >= 2,
            "need at least two segments to merge"
        );

        let free_before = free_blocks(&rel);

        pg_sys::QueryCancelPending = 1;
        let result = merger.merge_segments(&segment_ids);
        pg_sys::QueryCancelPending = 0;
        assert!(result.is_err(), "the merge must stop on the pending cancel");

        let tracked = pending(rel.oid());
        assert!(tracked > 0, "the cancelled merge left output files tracked");
        assert_eq!(reclaim(&rel), tracked);
        assert_eq!(pending(rel.oid()), 0);
        assert!(
            free_blocks(&rel) > free_before,
            "the cancelled merge's output blocks are back in the FSM"
        );
    }

    /// A successful merge publishes its files before the registry is drained, so nothing it wrote
    /// is ever reclaimed.
    #[pg_test]
    unsafe fn test_published_merge_output_is_not_reclaimed() {
        let rel = setup("unpublished_publish", "1gb");
        Spi::run(
            "SET paradedb.global_mutable_segment_rows = 0;
             INSERT INTO unpublished_publish SELECT g, 'silver dragon ' || g FROM generate_series(1, 200) g;
             INSERT INTO unpublished_publish SELECT g, 'golden dragon ' || g FROM generate_series(201, 400) g;
             RESET paradedb.global_mutable_segment_rows;",
        )
        .expect("fixture rows");
        pg_sys::CommandCounterIncrement();

        let mut merger =
            SearchIndexMerger::open(&rel, MvccSatisfies::Mergeable).expect("open merger");
        let segment_ids = merger.all_entries().keys().copied().collect::<Vec<_>>();
        assert!(
            segment_ids.len() >= 2,
            "need at least two segments to merge"
        );

        let merged = merger
            .merge_segments(&segment_ids)
            .expect("merge should succeed");
        assert!(merged.is_some(), "the merge produced a segment");
        assert_eq!(pending(rel.oid()), 0, "published files are untracked");
        assert_eq!(
            reclaim(&rel),
            0,
            "nothing to reclaim after a published merge"
        );

        let count: i64 =
            Spi::get_one("SELECT count(*) FROM unpublished_publish WHERE title @@@ 'dragon'")
                .expect("spi should succeed")
                .unwrap();
        assert_eq!(count, 400, "the merged segment is intact");
    }
}

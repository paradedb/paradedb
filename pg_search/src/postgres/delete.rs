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

use crate::api::HashMap;
use crate::index::fast_fields_helper::FFType;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::index::writer::index::{Mergeable, SearchIndexMerger};
use crate::postgres::locks::AdvisoryLock;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::SegmentMetaEntry;
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::utils::vacuum_delay_point;

use anyhow::Result;
use pgrx::pg_sys;
use pgrx::{IntoDatum, PgTryBuilder, direct_function_call, pg_sys::ItemPointerData, *};
use std::cell::RefCell;
use std::panic::AssertUnwindSafe;
use std::rc::Rc;
use tantivy::SegmentMeta;
use tantivy::indexer::delete_queue::DeleteQueue;
use tantivy::indexer::{DeleteOperation, SegmentEntry, advance_deletes};
use tantivy::{Directory, DocId, Index, IndexMeta, Opstamp};

// random key, ensures that this advisory lock key doesn't conflict
// with other Postgres advisory locks
const VACUUM_SIGNAL_KEY_BASE: u32 = 0x50475641; // "PGVA"

/// Vacuum signal lock for cooperative cancellation of background merge workers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VacuumSignal {
    index_oid: pg_sys::Oid,
}

impl VacuumSignal {
    pub(crate) fn new(index_oid: pg_sys::Oid) -> Self {
        Self { index_oid }
    }

    fn key(&self) -> i64 {
        ((VACUUM_SIGNAL_KEY_BASE as i64) << 32) | (u32::from(self.index_oid) as i64)
    }

    fn lock(&self) -> AdvisoryLock {
        AdvisoryLock::lock_session(self.key())
    }

    /// Check if vacuum wants background merge workers to cancel.
    ///
    /// Returns `true` if VACUUM holds the exclusive signal lock.
    pub(crate) fn wants_cancel(&self) -> bool {
        let key = self.key();
        let acquired = unsafe {
            direct_function_call::<bool>(
                pg_sys::pg_try_advisory_lock_shared_int8,
                &[key.into_datum()],
            )
        }
        .unwrap_or(false);

        if acquired {
            unsafe {
                direct_function_call::<bool>(
                    pg_sys::pg_advisory_unlock_shared_int8,
                    &[key.into_datum()],
                );
            }
        }
        !acquired
    }
}

#[pg_guard]
pub unsafe extern "C-unwind" fn ambulkdelete(
    info: *mut pg_sys::IndexVacuumInfo,
    stats: *mut pg_sys::IndexBulkDeleteResult,
    callback: pg_sys::IndexBulkDeleteCallback,
    callback_state: *mut ::std::os::raw::c_void,
) -> *mut pg_sys::IndexBulkDeleteResult {
    let info = PgBox::from_pg(info);
    let mut stats = PgBox::<pg_sys::IndexBulkDeleteResult>::from_pg(stats.cast());
    let index_relation = PgSearchRelation::from_pg(info.index);
    let callback =
        callback.expect("the ambulkdelete() callback should be a valid function pointer");
    let callback = move |ctid_val: u64| {
        let mut ctid = ItemPointerData::default();
        crate::postgres::utils::u64_to_item_pointer(ctid_val, &mut ctid);
        callback(&mut ctid, callback_state)
    };

    // Signal background merge workers via an advisory lock so they yield between candidates.
    // Open metadata before taking the signal to keep the lock window small.
    let mut metadata = MetaPage::open(&index_relation);
    let signal = Rc::new(RefCell::new(Some(
        VacuumSignal::new(index_relation.oid()).lock(),
    )));
    let signal_finally = signal.clone();

    // AdvisoryLock's Drop skips the unlock while an ERROR unwinds, so release the signal here.
    let merge_lock = PgTryBuilder::new(AssertUnwindSafe(|| {
        let merge_lock = metadata.acquire_merge_lock_for_cleanup();

        // Workers have exited - release signal
        if let Some(lock) = signal.borrow_mut().take() {
            drop(lock);
        }

        merge_lock
    }))
    .finally(move || {
        // Release signal if error occurred before we released it
        if let Some(lock) = signal_finally.borrow_mut().take() {
            drop(lock);
        }
    })
    .execute();

    // garbage_collect reclaims stale entries (owning backend exited or transaction ended) left
    // behind by a cancelled or crashed merge.
    //
    // We don't assert the list is empty afterwards. A merge that errored between writing its
    // MergeEntry and removing it leaves a not-yet-recyclable entry (backend alive, xmin in
    // progress) that survives garbage_collect. Such an entry is a harmless leftover, not a
    // concurrent merge: the cleanup acquisition above waited out admitted mergers and MergeLock
    // blocks new selection. A later garbage_collect reclaims it.
    merge_lock
        .merge_list()
        .garbage_collect(pg_sys::ReadNextFullTransactionId());

    let mutable_segments = metadata.freeze_mutable_segments();

    #[cfg(feature = "pg_test")]
    crate::postgres::storage::coordination_test::vacuum_setup_pause();

    let reader = SearchIndexReader::empty(&index_relation, MvccSatisfies::Vacuum)
        .expect("ambulkdelete: should be able to open a SearchIndexReader");
    let writer_segment_ids = reader.segment_ids();

    // Write out the list of segment ids we're about to operate on
    // Then acquire a `vacuum_sentinel` to notify concurrent backends that a vacuum is happening
    // The segment ids written here will be excluded from possible future concurrent merges, until `vacuum_sentinel` is dropped
    metadata.vacuum_list().write_list(writer_segment_ids.iter());
    let vacuum_sentinel = metadata.pin_ambulkdelete_sentinel();
    // It's important to drop the merge lock after the `vacuum_sentinel` is pinned
    drop(merge_lock);

    #[cfg(feature = "pg_test")]
    crate::postgres::storage::coordination_test::vacuum_pause();

    let mut old_metas = Vec::new();
    let mut new_metas = Vec::new();

    let directory = MvccSatisfies::Vacuum.directory(&index_relation);
    let index = crate::index::open_index(directory.clone()).unwrap();
    let searchable_segment_metas = index.searchable_segment_metas().unwrap();
    let mut did_delete = !mutable_segments.is_empty();

    for segment_reader in reader.segment_readers() {
        let segment_id = segment_reader.segment_id();
        if !writer_segment_ids.contains(&segment_id) || directory.is_mutable(&segment_id) {
            // the writer doesn't have this segment reader, and that's fine
            // we open the writer and reader in separate calls so it's possible
            // for the reader, which is opened second and outside of the MergeLock,
            // to see a different view of the segment entries on disk, but we only
            // need to concern ourselves with the ones the writer is aware of
            continue;
        }

        let segment_meta = searchable_segment_metas
            .iter()
            .find(|meta| meta.id() == segment_id)
            .unwrap_or_else(|| panic!("segment meta not found for segment_id: {segment_id:?}"));
        let mut deleter = SegmentDeleter::open(segment_meta);
        let ctid_ff = FFType::new_ctid(segment_reader.fast_fields());
        let mut num_deleted = 0;
        for (i, doc_id) in segment_reader.doc_ids_alive().enumerate() {
            if i % 100 == 0 {
                vacuum_delay_point();
            }
            if callback(ctid_ff.as_u64(doc_id).expect("ctid should be present")) {
                did_delete = true;
                num_deleted += 1;
                deleter.delete(doc_id);
            }
        }

        if num_deleted == segment_reader.num_docs() {
            did_delete = true;
            old_metas.push(segment_meta.clone());
        } else if num_deleted > 0 {
            let meta_change = deleter
                .commit(&index)
                .expect("ambulkdelete: segment deleter commit should succeed");
            if let Some((old_meta, new_meta)) = meta_change {
                old_metas.push(old_meta);
                new_metas.push(new_meta);
            }
        }
    }
    // no need to keep the reader around.  Also, it holds a pin on the CLEANUP_LOCK, which
    // will get in the way of our CLEANUP_LOCK barrier below
    drop(reader);

    if stats.is_null() {
        stats = unsafe {
            PgBox::from_pg(
                pg_sys::palloc0(std::mem::size_of::<pg_sys::IndexBulkDeleteResult>()).cast(),
            )
        };
        stats.pages_deleted = 0;
    }

    if !old_metas.is_empty() {
        // Save the new delete metas entries in one atomic operation
        save_delete_metas(&index, old_metas, new_metas)
            .expect("ambulkdelete: should be able to save delete metas entries");
    }

    drop(index);
    drop(directory);

    flush_mutable_segments(&index_relation, &mutable_segments, &callback)
        .expect("ambulkdelete: should be able to flush mutable segments");

    if did_delete {
        // As soon as ambulkdelete returns, Postgres will update the visibility map
        // This can cause concurrent scans that have just read ctids, which are dead but
        // are about to be marked visible, to return wrong results. To guard against this,
        // we acquire a cleanup lock that guarantees that there are no pins on the index,
        // which means that all concurrent scans have completed.
        //
        // Effectively, we're blocking ambulkdelete from finishing until we know that concurrent
        // scans have finished too
        drop(metadata.cleanup_lock_for_cleanup());
        metadata.increment_ambulkdelete_epoch();
    }

    // we're done, no need to hold onto the sentinel any longer
    drop(vacuum_sentinel);

    stats.into_pg()
}

fn flush_mutable_segments(
    indexrel: &PgSearchRelation,
    segments: &[SegmentMetaEntry],
    callback: &impl Fn(u64) -> bool,
) -> Result<()> {
    for batch in segments.chunks(2) {
        let mut live_ctids = HashMap::default();
        let mut segment_ids = Vec::with_capacity(batch.len());
        for segment in batch {
            let ctids = segment
                .mutable_snapshot(indexrel)
                .map_err(|error| anyhow::anyhow!("{error}"))?;
            let ctids = ctids
                .into_iter()
                .enumerate()
                .filter_map(|(i, ctid)| {
                    if i % 100 == 0 {
                        vacuum_delay_point();
                    }
                    // Dead tuples may already have lost their TOAST values. Never fetch them.
                    (!callback(ctid)).then_some(ctid)
                })
                .collect();
            let segment_id = segment.segment_id();
            segment_ids.push(segment_id);
            live_ctids.insert(segment_id, ctids);
        }
        let mut merger =
            SearchIndexMerger::open(indexrel, MvccSatisfies::VacuumMerge { live_ctids })?;
        if let Err(error) = merger.merge_segments(&segment_ids) {
            // `merge_segments` returns `Err` on cancellation so `merge_index` can clean up its
            // `MergeEntry` before unwinding (see https://github.com/paradedb/paradedb/issues/6962).
            // Check for interrupts here before `ambulkdelete`'s `.expect()` turns a cancellation
            // into an internal error panic.
            pgrx::check_for_interrupts!();
            return Err(error);
        }
    }
    Ok(())
}

struct SegmentDeleter {
    delete_queue: DeleteQueue,
    segment_entry: SegmentEntry,
    opstamp: Opstamp,
}

impl SegmentDeleter {
    fn open(segment_meta: &SegmentMeta) -> Self {
        let delete_queue = DeleteQueue::default();
        let delete_cursor = delete_queue.cursor();
        let opstamp = segment_meta.delete_opstamp().unwrap_or_default();
        let segment_entry = SegmentEntry::new(segment_meta.clone(), delete_cursor, None);
        Self {
            delete_queue,
            segment_entry,
            opstamp,
        }
    }

    fn delete(&mut self, doc_id: DocId) {
        self.opstamp += 1;
        self.delete_queue.push(DeleteOperation::ByAddress {
            opstamp: self.opstamp,
            segment_id: self.segment_entry.meta().id(),
            doc_id,
        });
    }

    fn commit(mut self, index: &Index) -> Result<Option<(SegmentMeta, SegmentMeta)>> {
        let old_meta = self.segment_entry.meta().clone();
        let segment = index.segment(self.segment_entry.meta().clone());
        advance_deletes(segment, &mut self.segment_entry, self.opstamp + 1)?;
        let new_meta = self.segment_entry.meta().clone();
        // [dst correctness] VACUUM delete is monotonic and same-segment: applying this
        // VACUUM's tombstones to an immutable segment must (a) keep the same segment id and
        // physical max_doc (advance_deletes never rewrites the segment's docs, only its .del
        // file), and (b) never resurrect a deleted doc -- num_deleted only grows and live docs
        // only shrink. A violation means the atomic swap would pair mismatched segments or
        // un-delete already-dead rows (wrong results / resurrected tuples).
        dst::observe!(|| {
            let old_segment_id = old_meta.id();
            let new_segment_id = new_meta.id();
            let old_max_doc = old_meta.max_doc();
            let new_max_doc = new_meta.max_doc();
            let old_num_deleted_docs = old_meta.num_deleted_docs();
            let new_num_deleted_docs = new_meta.num_deleted_docs();
            let old_num_docs = old_meta.num_docs();
            let new_num_docs = new_meta.num_docs();
            dst::assert_always!(
                new_segment_id == old_segment_id
                    && new_max_doc == old_max_doc
                    && new_num_deleted_docs >= old_num_deleted_docs
                    && new_num_docs <= old_num_docs,
                "pg_search: ambulkdelete immutable delete counts are monotonic and same-segment",
                &::serde_json::json!({
                    "old_segment_id": old_segment_id.to_string(),
                    "new_segment_id": new_segment_id.to_string(),
                    "old_max_doc": old_max_doc,
                    "new_max_doc": new_max_doc,
                    "old_num_deleted_docs": old_num_deleted_docs,
                    "new_num_deleted_docs": new_num_deleted_docs,
                    "old_num_docs": old_num_docs,
                    "new_num_docs": new_num_docs,
                })
            );
        });
        Ok(Some((old_meta, new_meta)))
    }
}

fn save_delete_metas(
    index: &Index,
    old_metas: Vec<SegmentMeta>,
    new_metas: Vec<SegmentMeta>,
) -> Result<()> {
    let current_metas = index.load_metas()?;
    let old_index_meta = IndexMeta {
        segments: old_metas,
        ..current_metas.clone()
    };
    let new_index_meta = IndexMeta {
        segments: new_metas.clone(),
        ..current_metas.clone()
    };
    index
        .directory()
        .save_metas(&new_index_meta, &old_index_meta, &mut ())?;
    Ok(())
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::postgres::storage::block::{MutableSegmentEntry, SegmentMetaEntryMutable};
    use tantivy::index::SegmentId;

    #[pg_test(error = "canceling statement due to user request")]
    unsafe fn test_vacuum_flush_preserves_query_cancellation() {
        Spi::run(
            "CREATE TABLE vacuum_cancel (id INT, message TEXT);
             CREATE INDEX vacuum_cancel_idx ON vacuum_cancel USING paradedb (id, message)
             WITH (mutable_segment_rows = 100, layer_sizes = '0', background_layer_sizes = '0');
             INSERT INTO vacuum_cancel VALUES (1, 'hello');",
        )
        .unwrap();
        let indexrel = PgSearchRelation::open(
            Spi::get_one::<pg_sys::Oid>("SELECT 'vacuum_cancel_idx'::regclass::oid")
                .unwrap()
                .unwrap(),
        );
        let metadata = MetaPage::open(&indexrel);
        let segments = metadata.freeze_mutable_segments();
        assert_eq!(segments.len(), 1);

        flush_mutable_segments(&indexrel, &segments, &|_ctid| {
            // Arm cancellation after `vacuum_delay_point()` so Tantivy's foreground merge
            // observes `QueryCancelPending` via `MVCCDirectory::wants_cancel()`.
            unsafe {
                pg_sys::QueryCancelPending = 1;
                pg_sys::InterruptPending = 1;
            }
            false
        })
        .unwrap();
    }

    #[pg_test]
    unsafe fn test_vacuum_flushes_mutable_batches() {
        Spi::run(
            "CREATE TABLE vacuum_batches (id INT, message TEXT);
             CREATE INDEX vacuum_batches_idx ON vacuum_batches USING paradedb (id, message)
             WITH (mutable_segment_rows = 100, layer_sizes = '0', background_layer_sizes = '0');
             INSERT INTO vacuum_batches SELECT i, 'hello' FROM generate_series(1, 10) i;",
        )
        .unwrap();
        let indexrel = PgSearchRelation::open(
            Spi::get_one::<pg_sys::Oid>("SELECT 'vacuum_batches_idx'::regclass::oid")
                .unwrap()
                .unwrap(),
        );
        let metadata = MetaPage::open(&indexrel);
        let mut metas = metadata.segment_metas();
        let original = metas.list(None)[0];
        let mut ctids = original.mutable_snapshot(&indexrel).unwrap();
        ctids.sort_unstable();
        assert_eq!(ctids.len(), 10);
        metas
            .update_item(
                |entry| entry.segment_id() == original.segment_id(),
                |entry| entry.set_xmax(pg_sys::FrozenTransactionId),
            )
            .unwrap();
        for ctids in ctids.chunks(2) {
            let (content, mut items) = SegmentMetaEntryMutable::create(&indexrel);
            items.add_items(
                &ctids
                    .iter()
                    .copied()
                    .map(MutableSegmentEntry::Add)
                    .collect::<Vec<_>>(),
                None,
            );
            metas.add_items(
                &[SegmentMetaEntry::new_mutable(
                    SegmentId::generate_random(),
                    ctids.len() as u32,
                    pg_sys::InvalidTransactionId,
                    content,
                )],
                None,
            );
        }
        let segments = metadata.freeze_mutable_segments();
        assert_eq!(segments.len(), 5);
        flush_mutable_segments(&indexrel, &segments, &|ctid| ctids[4..8].contains(&ctid)).unwrap();
        let entries = metas
            .list(None)
            .into_iter()
            .filter(|entry| !entry.is_deleted())
            .collect::<Vec<_>>();
        assert!(entries.iter().all(|entry| !entry.is_mutable()));
        let mut counts = entries
            .iter()
            .map(|entry| entry.max_doc())
            .collect::<Vec<_>>();
        counts.sort_unstable();
        assert_eq!(counts, [2, 4]);
        assert_eq!(
            Spi::get_one::<i64>("SELECT count(*) FROM vacuum_batches WHERE message @@@ 'hello'")
                .unwrap(),
            Some(6),
        );
    }
}

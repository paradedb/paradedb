// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::index::fast_fields_helper::FFType;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::metadata::MetaPage;

use anyhow::Result;
use pgrx::{pg_sys::ItemPointerData, *};
use tantivy::indexer::delete_queue::DeleteQueue;
use tantivy::indexer::{advance_deletes, DeleteOperation, SegmentEntry};
use tantivy::SegmentMeta;
use tantivy::{Directory, DocId, Index, IndexMeta, Opstamp};

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

    // first, we need an exclusive lock on the CLEANUP_LOCK.  Once we get it, we know that there
    // are no concurrent merges happening
    let mut metadata = MetaPage::open(&index_relation);
    let cleanup_lock = metadata.cleanup_lock_exclusive();

    // take the MergeLock
    let merge_lock = metadata.acquire_merge_lock();

    // garbage collecting the MergeList is necessary to remove any stale entries that may have
    // been leftover from a cancelled merge or crash during merge
    merge_lock.merge_list().garbage_collect();

    // and now we should not have any merges happening, and cannot
    assert!(
        merge_lock.merge_list().is_empty(),
        "ambulkdelete cannot run concurrently with an active merge operation"
    );
    drop(cleanup_lock);

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

    let mut old_metas = Vec::new();
    let mut new_metas = Vec::new();

    let directory = MvccSatisfies::Vacuum.directory(&index_relation);
    let index = Index::open(directory).unwrap();
    let searchable_segment_metas = index.searchable_segment_metas().unwrap();

    for segment_reader in reader.segment_readers() {
        let segment_id = segment_reader.segment_id();
        if !writer_segment_ids.contains(&segment_id) {
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
        let mut deleter = SegmentDeleter::open(segment_meta)
            .expect("ambulkdelete: should be able to open a SegmentDeleter");
        let ctid_ff = FFType::new_ctid(segment_reader.fast_fields());
        let mut needs_commit = false;

        for doc_id in 0..segment_reader.max_doc() {
            if doc_id % 100 == 0 {
                // we think there's a pending interrupt, so this should raise a cancel query ERROR
                pg_sys::vacuum_delay_point();
            }

            let ctid = ctid_ff.as_u64(doc_id).expect("ctid should be present");
            if callback(ctid) {
                needs_commit = true;
                deleter.delete_document(doc_id);
            }
        }

        if needs_commit {
            let (old_meta, new_meta) = deleter
                .commit(&index)
                .expect("ambulkdelete: segment deletercommit should succeed");
            old_metas.push(old_meta);
            new_metas.push(new_meta);
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
        assert_eq!(old_metas.len(), new_metas.len());
        save_delete_metas(&index, old_metas, new_metas)
            .expect("ambulkdelete: should be able to save delete metas entries");

        // As soon as ambulkdelete returns, Postgres will update the visibility map
        // This can cause concurrent scans that have just read ctids, which are dead but
        // are about to be marked visible, to return wrong results. To guard against this,
        // we acquire a cleanup lock that guarantees that there are no pins on the index,
        // which means that all concurrent scans have completed.
        //
        // Effectively, we're blocking ambulkdelete from finishing until we know that concurrent
        // scans have finished too
        drop(metadata.cleanup_lock_for_cleanup());
    }

    // we're done, no need to hold onto the sentinel any longer
    drop(vacuum_sentinel);
    drop(metadata);

    stats.into_pg()
}

struct SegmentDeleter {
    delete_queue: DeleteQueue,
    segment_entry: SegmentEntry,
    opstamp: Opstamp,
}

impl SegmentDeleter {
    pub fn open(segment_meta: &SegmentMeta) -> Result<Self> {
        let delete_queue = DeleteQueue::new();
        let delete_cursor = delete_queue.cursor();
        let opstamp = segment_meta.delete_opstamp().unwrap_or_default();

        // It's important to set the entry/cursor at the beginning vs. when commit() is called,
        // because the delete cursor can only look forward
        let segment_entry = SegmentEntry::new(segment_meta.clone(), delete_cursor, None);

        Ok(Self {
            delete_queue,
            segment_entry,
            opstamp,
        })
    }

    pub fn delete_document(&mut self, doc_id: DocId) {
        self.opstamp += 1;
        self.delete_queue.push(DeleteOperation::ByAddress {
            opstamp: self.opstamp,
            segment_id: self.segment_entry.meta().id(),
            doc_id,
        });
    }

    pub fn commit(mut self, index: &Index) -> Result<(SegmentMeta, SegmentMeta)> {
        let old_meta = self.segment_entry.meta().clone();
        let segment = index.segment(self.segment_entry.meta().clone());
        advance_deletes(segment, &mut self.segment_entry, self.opstamp + 1)?;
        Ok((old_meta, self.segment_entry.meta().clone()))
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

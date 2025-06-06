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

use pgrx::{pg_sys::ItemPointerData, *};

use super::storage::block::CLEANUP_LOCK;
use crate::index::fast_fields_helper::FFType;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::index::writer::index::SearchIndexDeleter;
use crate::index::WriterResources;
use crate::postgres::storage::buffer::BufferManager;
use crate::postgres::storage::metadata::MetaPage;

#[pg_guard]
pub unsafe extern "C-unwind" fn ambulkdelete(
    info: *mut pg_sys::IndexVacuumInfo,
    stats: *mut pg_sys::IndexBulkDeleteResult,
    callback: pg_sys::IndexBulkDeleteCallback,
    callback_state: *mut ::std::os::raw::c_void,
) -> *mut pg_sys::IndexBulkDeleteResult {
    let info = PgBox::from_pg(info);
    let mut stats = PgBox::<pg_sys::IndexBulkDeleteResult>::from_pg(stats.cast());
    let index_relation = PgRelation::from_pg(info.index);
    let callback =
        callback.expect("the ambulkdelete() callback should be a valid function pointer");
    let callback = move |ctid_val: u64| {
        let mut ctid = ItemPointerData::default();
        crate::postgres::utils::u64_to_item_pointer(ctid_val, &mut ctid);
        callback(&mut ctid, callback_state)
    };

    // first, we need an exclusive lock on the CLEANUP_LOCK.  Once we get it, we know that there
    // are no concurrent merges happening
    let cleanup_lock = BufferManager::new(index_relation.oid()).get_buffer_mut(CLEANUP_LOCK);
    let mut metadata = MetaPage::open(index_relation.oid());

    // take the MergeLock
    let merge_lock = metadata.acquire_merge_lock();

    // garbage collecting the MergeList is necessary to remove any stale entries that may have
    // been leftover from a cancelled merge or crash during merge
    merge_lock.merge_list().garbage_collect();
    pg_sys::IndexFreeSpaceMapVacuum(index_relation.as_ptr());

    // and now we should not have any merges happening, and cannot
    assert!(
        merge_lock.merge_list().is_empty(),
        "ambulkdelete cannot run concurrently with an active merge operation"
    );
    drop(cleanup_lock);

    let mut index_writer = SearchIndexDeleter::open(
        &index_relation,
        MvccSatisfies::Vacuum,
        WriterResources::Vacuum,
    )
    .expect("ambulkdelete: should be able to open a SearchIndexWriter");
    let reader = SearchIndexReader::open(&index_relation, MvccSatisfies::Vacuum)
        .expect("ambulkdelete: should be able to open a SearchIndexReader");

    let writer_segment_ids = index_writer.segment_ids();

    // Write out the list of segment ids we're about to operate on
    // Then acquire a `vacuum_sentinel` to notify concurrent backends that a vacuum is happening
    // The segment ids written here will be excluded from possible future concurrent merges, until `vacuum_sentinel` is dropped
    metadata.vacuum_list().write_list(writer_segment_ids.iter());
    let vacuum_sentinel = metadata.pin_ambulkdelete_sentinel();
    // It's important to drop the merge lock after the `vacuum_sentinel` is pinned
    drop(merge_lock);

    let mut did_delete = false;
    for segment_reader in reader.segment_readers() {
        if !writer_segment_ids.contains(&segment_reader.segment_id()) {
            // the writer doesn't have this segment reader, and that's fine
            // we open the writer and reader in separate calls so it's possible
            // for the reader, which is opened second and outside of the MergeLock,
            // to see a different view of the segment entries on disk, but we only
            // need to concern ourselves with the ones the writer is aware of
            continue;
        }
        let ctid_ff = FFType::new_ctid(segment_reader.fast_fields());

        for doc_id in 0..segment_reader.max_doc() {
            if doc_id % 100 == 0 {
                // we think there's a pending interrupt, so this should raise a cancel query ERROR
                pg_sys::vacuum_delay_point();
            }

            let ctid = ctid_ff.as_u64(doc_id).expect("ctid should be present");
            if callback(ctid) {
                did_delete = true;
                index_writer
                    .delete_document(segment_reader.segment_id(), doc_id)
                    .expect("ambulkdelete: deleting document by segment and id should succeed");
            }
        }
    }
    // no need to keep the reader around.  Also, it holds a pin on the CLEANUP_LOCK, which
    // will get in the way of our CLEANUP_LOCK barrier below
    drop(reader);

    // this won't merge as the `WriterResources::Vacuum` uses `AllowedMergePolicy::None`
    index_writer
        .commit()
        .expect("ambulkdelete: commit should succeed");

    if stats.is_null() {
        stats = unsafe {
            PgBox::from_pg(
                pg_sys::palloc0(std::mem::size_of::<pg_sys::IndexBulkDeleteResult>()).cast(),
            )
        };
        stats.pages_deleted = 0;
    }

    // As soon as ambulkdelete returns, Postgres will update the visibility map
    // This can cause concurrent scans that have just read ctids, which are dead but
    // are about to be marked visible, to return wrong results. To guard against this,
    // we acquire a cleanup lock that guarantees that there are no pins on the index,
    // which means that all concurrent scans have completed.
    //
    // Effectively, we're blocking ambulkdelete from finishing until we know that concurrent
    // scans have finished too
    if did_delete {
        drop(BufferManager::new(index_relation.oid()).get_buffer_for_cleanup(CLEANUP_LOCK));
    }

    // we're done, no need to hold onto the sentinel any longer
    drop(vacuum_sentinel);
    stats.into_pg()
}

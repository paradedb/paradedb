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

use crate::index::merge_policy::{LayeredMergePolicy, NumCandidates, NumMerged};
use crate::index::mvcc::MvccSatisfies;
use crate::index::writer::index::SearchIndexWriter;
use crate::index::WriterResources;
use crate::postgres::options::SearchIndexCreateOptions;
use crate::postgres::storage::block::{SegmentMetaEntry, CLEANUP_LOCK, SEGMENT_METAS_START};
use crate::postgres::storage::buffer::BufferManager;
use crate::postgres::storage::merge::MergeLock;
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use crate::postgres::utils::{
    categorize_fields, item_pointer_to_u64, row_to_search_document, CategorizedFieldData,
};
use crate::schema::SearchField;
use pgrx::{pg_guard, pg_sys, PgMemoryContexts, PgRelation, PgTupleDesc};
use std::collections::HashMap;
use std::ffi::CStr;
use std::panic::{catch_unwind, resume_unwind};

extern "C" {
    fn IsLogicalWorker() -> bool;
}

pub struct InsertState {
    #[allow(dead_code)] // field is used by pg<16 for the fakeaminsertcleanup stuff
    pub indexrelid: pg_sys::Oid,
    pub writer: Option<SearchIndexWriter>,
    categorized_fields: Vec<(SearchField, CategorizedFieldData)>,
    key_field_name: String,
    per_row_context: PgMemoryContexts,
}

impl InsertState {
    unsafe fn new(
        indexrel: &PgRelation,
        writer_resources: WriterResources,
    ) -> anyhow::Result<Self> {
        let writer = SearchIndexWriter::open(indexrel, MvccSatisfies::Mergeable, writer_resources)?;
        let tupdesc = unsafe { PgTupleDesc::from_pg_unchecked(indexrel.rd_att) };
        let categorized_fields = categorize_fields(&tupdesc, &writer.schema);
        let key_field_name = writer.schema.key_field().name.0;

        let per_row_context = pg_sys::AllocSetContextCreateExtended(
            PgMemoryContexts::CurrentMemoryContext.value(),
            c"pg_search aminsert context".as_ptr(),
            pg_sys::ALLOCSET_DEFAULT_MINSIZE as usize,
            pg_sys::ALLOCSET_DEFAULT_INITSIZE as usize,
            pg_sys::ALLOCSET_DEFAULT_MAXSIZE as usize,
        );

        Ok(Self {
            indexrelid: indexrel.oid(),
            writer: Some(writer),
            categorized_fields,
            key_field_name,
            per_row_context: PgMemoryContexts::For(per_row_context),
        })
    }
}

#[cfg(not(feature = "pg17"))]
unsafe fn init_insert_state(
    index_relation: pg_sys::Relation,
    index_info: &mut pg_sys::IndexInfo,
    writer_resources: WriterResources,
) -> &'static mut InsertState {
    use crate::postgres::fake_aminsertcleanup::{get_insert_state, push_insert_state};

    if index_info.ii_AmCache.is_null() {
        let index_relation = PgRelation::from_pg(index_relation);
        let state = InsertState::new(&index_relation, writer_resources)
            .expect("should be able to open new SearchIndex for writing");

        push_insert_state(state);
        index_info.ii_AmCache = &true as *const _ as *mut _; // a pointer to `true` to indicate that we've set up the InsertState
    }

    get_insert_state((*index_relation).rd_id).expect("should have a pending insert state")
}

#[cfg(feature = "pg17")]
pub unsafe fn init_insert_state(
    index_relation: pg_sys::Relation,
    index_info: &mut pg_sys::IndexInfo,
    writer_resources: WriterResources,
) -> &mut InsertState {
    if index_info.ii_AmCache.is_null() {
        // we don't have any cached state yet, so create it now
        let index_relation = PgRelation::from_pg(index_relation);
        let state = InsertState::new(&index_relation, writer_resources)
            .expect("should be able to open new SearchIndex for writing");

        // leak it into the MemoryContext for this scan (as specified by the IndexInfo argument)
        //
        // When that memory context is freed by Postgres is when we'll do our tantivy commit/abort
        // of the changes made during `aminsert`
        //
        // SAFETY: `leak_and_drop_on_delete` palloc's memory in CurrentMemoryContext, but in this
        // case we want the thing it allocates to be palloc'd in the `ii_Context`
        pgrx::PgMemoryContexts::For(index_info.ii_Context)
            .switch_to(|mcxt| index_info.ii_AmCache = mcxt.leak_and_drop_on_delete(state).cast())
    };

    &mut *index_info.ii_AmCache.cast()
}

#[allow(clippy::too_many_arguments)]
#[pg_guard]
pub unsafe extern "C" fn aminsert(
    index_relation: pg_sys::Relation,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    heap_tid: pg_sys::ItemPointer,
    _heap_relation: pg_sys::Relation,
    _check_unique: pg_sys::IndexUniqueCheck::Type,
    _index_unchanged: bool,
    index_info: *mut pg_sys::IndexInfo,
) -> bool {
    aminsert_internal(index_relation, values, isnull, heap_tid, index_info)
}

#[inline(always)]
unsafe fn aminsert_internal(
    index_relation: pg_sys::Relation,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    ctid: pg_sys::ItemPointer,
    index_info: *mut pg_sys::IndexInfo,
) -> bool {
    if IsLogicalWorker() {
        panic!("pg_search logical replication is an enterprise feature");
    }

    let result = catch_unwind(|| {
        let state = init_insert_state(
            index_relation,
            index_info
                .as_mut()
                .expect("index_info argument must not be null"),
            WriterResources::Statement,
        );

        state.per_row_context.switch_to(|cxt| {
            let categorized_fields = &state.categorized_fields;
            let key_field_name = &state.key_field_name;
            let writer = state.writer.as_mut().expect("writer should not be null");

            let mut search_document = writer.schema.new_document();

            row_to_search_document(
                values,
                isnull,
                key_field_name,
                categorized_fields,
                &mut search_document,
            )
            .unwrap_or_else(|err| {
                panic!(
                    "error creating index entries for index '{}': {err}",
                    CStr::from_ptr((*(*index_relation).rd_rel).relname.data.as_ptr())
                        .to_string_lossy()
                );
            });
            writer
                .insert(search_document, item_pointer_to_u64(*ctid))
                .expect("insertion into index should succeed");

            cxt.reset();
            true
        })
    });

    match result {
        Ok(result) => result,
        Err(e) => resume_unwind(e),
    }
}

#[cfg(feature = "pg17")]
#[pg_guard]
pub unsafe extern "C" fn aminsertcleanup(
    _index_relation: pg_sys::Relation,
    index_info: *mut pg_sys::IndexInfo,
) {
    let state = (*index_info).ii_AmCache.cast::<InsertState>();
    if state.is_null() {
        return;
    }

    paradedb_aminsertcleanup(state.as_mut().and_then(|state| state.writer.take()));
}

pub fn paradedb_aminsertcleanup(mut writer: Option<SearchIndexWriter>) {
    if let Some(writer) = writer.take() {
        let indexrelid = writer.indexrelid;

        writer
            .commit()
            .expect("must be able to commit inserts in paradedb_aminsertcleanup");

        unsafe {
            do_merge(indexrelid);
        }
    }
}

#[allow(clippy::identity_op)]
pub(crate) const DEFAULT_LAYER_SIZES: &[u64] = &[
    100 * 1024,        // 100KB
    1 * 1024 * 1024,   // 1MB
    100 * 1024 * 1024, // 100MB
];

unsafe fn do_merge(indexrelid: pg_sys::Oid) -> (NumCandidates, NumMerged) {
    let indexrel = {
        let indexrel = PgRelation::open(indexrelid);
        let heaprel = indexrel
            .heap_relation()
            .expect("index should belong to a heap relation");

        /*
         * Recompute VACUUM XID boundaries.
         *
         * We don't actually care about the oldest non-removable XID.  Computing
         * the oldest such XID has a useful side-effect that we rely on: it
         * forcibly updates the XID horizon state for this backend.  This step is
         * essential; GlobalVisCheckRemovableFullXid() will not reliably recognize
         * that it is now safe to recycle newly deleted pages without this step.
         */
        pg_sys::GetOldestNonRemovableTransactionId(heaprel.as_ptr());
        indexrel
    };

    let index_options = SearchIndexCreateOptions::from_relation(&indexrel);
    let merge_policy = LayeredMergePolicy::new(index_options.layer_sizes(DEFAULT_LAYER_SIZES));

    merge_index_with_policy(indexrel, merge_policy)
}

pub unsafe fn merge_index_with_policy(
    indexrel: PgRelation,
    mut merge_policy: LayeredMergePolicy,
) -> (NumCandidates, NumMerged) {
    let indexrelid = indexrel.oid();
    let all_segments =
        LinkedItemList::<SegmentMetaEntry>::open(indexrelid, SEGMENT_METAS_START).list();

    // take a shared lock on the CLEANUP_LOCK and hold it until this function is done.  We keep it
    // locked here so we can cause `ambulkdelete()` to block, waiting for all merging to finish
    // before it decides to find the segments it should vacuum.  The reason is that it needs to see
    // the final merged segment, not the original segments that will be deleted
    let cleanup_lock = BufferManager::new(indexrelid).get_buffer(CLEANUP_LOCK);
    let mut merge_lock = MergeLock::acquire(indexrelid);
    let mut writer = SearchIndexWriter::open(
        &PgRelation::open(indexrelid),
        MvccSatisfies::Mergeable,
        WriterResources::PostStatementMerge,
    )
    .expect("should be able to open a SearchIndexWriter for PostStatementMerge");
    let writer_segment_ids = &writer.segment_ids();

    // the non_mergeable_segments are those that are concurrently being vacuumed *and* merged
    let mut non_mergeable_segments = merge_lock.list_vacuuming_segments();
    non_mergeable_segments.extend(merge_lock.in_progress_segment_ids());

    if pg_sys::message_level_is_interesting(pg_sys::DEBUG1 as _) {
        pgrx::debug1!("do_merge: non_mergeable_segments={non_mergeable_segments:?}");
        pgrx::debug1!("do_merge: writer_segment_ids={writer_segment_ids:?}");
    }

    // tell the MergePolicy which segments it's allowed to consider for merging
    merge_policy.mergeable_segments = all_segments
        .into_iter()
        .filter_map(|entry| {
            let segment_id = &entry.segment_id;

            // every segment that isn't known to currently be merging
            (!non_mergeable_segments.contains(segment_id)
                // and is also seen by the writer
                && writer_segment_ids.contains(segment_id))
            .then_some((*segment_id, entry))
        })
        .collect::<HashMap<_, _>>();

    // reduce the set of segments that the LayeredMergePolicy will operate on by internally
    // simulating the process, allowing concurrent merges to consider segments we're not
    let (ncandidates, nmerged) = merge_policy.simulate();

    if pg_sys::message_level_is_interesting(pg_sys::DEBUG1 as _) {
        pgrx::debug1!(
            "do_merge: merge_policy.mergeable_segments={:?}",
            merge_policy.mergeable_segments
        );
    }

    if ncandidates > 0 {
        // record all the segments the IndexWriter can see, as those are the ones that
        // could be merged.  This also drops the MergeLock
        let merge_entry = merge_lock
            .record_in_progress_segment_ids(merge_policy.mergeable_segments.keys())
            .expect("should be able to write current merge segment_id list");

        // and concurrently do the merge -- we are NOT under the MergeLock at this point
        //
        // we defer raising a panic in the face of a merge error as we need to remove the created
        // `merge_entry` whether the merge worked or not
        let merge_result = {
            writer.set_merge_policy(merge_policy);
            writer.merge()
        };

        // re-acquire the MergeLock to remove the entry we made above
        // we also can't concurrently garbage collect our segment meta entries list
        let mut merge_lock = MergeLock::acquire(indexrelid);
        merge_lock
            .remove_entry(merge_entry)
            .expect("should be able to remove MergeEntry");
        drop(merge_lock);

        if let Err(e) = merge_result {
            panic!("failed to merge: {:?}", e);
        }
    } else {
        drop(merge_lock)
    };
    drop(cleanup_lock);

    // we can garbage collect and return blocks back to the FSM without being under the MergeLock
    let recycled_entries =
        LinkedItemList::<SegmentMetaEntry>::open(indexrelid, SEGMENT_METAS_START).garbage_collect();
    for entry in recycled_entries {
        for (file_entry, type_) in entry.file_entries() {
            let bytes = LinkedBytesList::open(indexrelid, file_entry.starting_block);
            bytes.return_to_fsm(&entry, Some(type_));
            pg_sys::IndexFreeSpaceMapVacuum(indexrel.as_ptr());
        }
    }

    (ncandidates, nmerged)
}

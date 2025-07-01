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

use crate::api::FieldName;
use crate::gucs;
use crate::index::merge_policy::{LayeredMergePolicy, NumCandidates, NumMerged};
use crate::index::mvcc::MvccSatisfies;
use crate::index::writer::index::{
    IndexWriterConfig, Mergeable, SearchIndexMerger, SerialIndexWriter,
};
use crate::postgres::options::SearchIndexOptions;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::SegmentMetaEntry;
use crate::postgres::storage::buffer::BufferManager;
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use crate::postgres::utils::{
    categorize_fields, item_pointer_to_u64, row_to_search_document, CategorizedFieldData,
};
use crate::schema::SearchField;
use pgrx::{check_for_interrupts, pg_guard, pg_sys, PgMemoryContexts};
use std::panic::{catch_unwind, resume_unwind};
use tantivy::{SegmentMeta, TantivyDocument};

pub struct InsertState {
    #[allow(dead_code)] // field is used by pg<16 for the fakeaminsertcleanup stuff
    pub indexrelid: pg_sys::Oid,
    pub writer: Option<SerialIndexWriter>,
    categorized_fields: Vec<(SearchField, CategorizedFieldData)>,
    key_field_name: FieldName,
    per_row_context: PgMemoryContexts,
}

impl InsertState {
    unsafe fn new(indexrel: &PgSearchRelation) -> anyhow::Result<Self> {
        let config = IndexWriterConfig {
            memory_budget: gucs::adjust_work_mem(),
            max_docs_per_segment: None,
        };
        let writer = SerialIndexWriter::with_mvcc(
            indexrel,
            MvccSatisfies::Mergeable,
            config,
            Default::default(),
        )?;
        let schema = writer.schema();
        let categorized_fields = categorize_fields(indexrel, schema);
        let key_field_name = schema.key_field().field_name();

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
) -> &'static mut InsertState {
    use crate::postgres::fake_aminsertcleanup::{get_insert_state, push_insert_state};

    if index_info.ii_AmCache.is_null() {
        let index_relation = PgSearchRelation::from_pg(index_relation);
        let state = InsertState::new(&index_relation)
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
) -> &mut InsertState {
    if index_info.ii_AmCache.is_null() {
        // we don't have any cached state yet, so create it now
        let index_relation = PgSearchRelation::from_pg(index_relation);
        let state = InsertState::new(&index_relation)
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
pub unsafe extern "C-unwind" fn aminsert(
    index_relation: pg_sys::Relation,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    ctid: pg_sys::ItemPointer,
    _heap_relation: pg_sys::Relation,
    _check_unique: pg_sys::IndexUniqueCheck::Type,
    _index_unchanged: bool,
    index_info: *mut pg_sys::IndexInfo,
) -> bool {
    if pg_sys::IsLogicalWorker() {
        panic!("pg_search logical replication is an enterprise feature");
    }

    let result = catch_unwind(|| {
        let state = init_insert_state(
            index_relation,
            index_info
                .as_mut()
                .expect("index_info argument must not be null"),
        );

        state.per_row_context.switch_to(|cxt| {
            let categorized_fields = &state.categorized_fields;
            let key_field_name = &state.key_field_name;
            let writer = state.writer.as_mut().expect("writer should not be null");

            let mut search_document = TantivyDocument::new();

            row_to_search_document(
                values,
                isnull,
                key_field_name,
                categorized_fields,
                &mut search_document,
            )
            .unwrap_or_else(|err| panic!("{err}"));
            writer
                .insert(search_document, item_pointer_to_u64(*ctid), || {})
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
pub unsafe extern "C-unwind" fn aminsertcleanup(
    _index_relation: pg_sys::Relation,
    index_info: *mut pg_sys::IndexInfo,
) {
    let state = (*index_info).ii_AmCache.cast::<InsertState>();
    if state.is_null() {
        return;
    }

    paradedb_aminsertcleanup(state.as_mut().and_then(|state| state.writer.take()));
}

pub fn paradedb_aminsertcleanup(mut writer: Option<SerialIndexWriter>) {
    if let Some(writer) = writer.take() {
        if let Some((_, indexrel)) = writer
            .commit()
            .expect("must be able to commit inserts in paradedb_aminsertcleanup")
        {
            unsafe {
                do_merge(indexrel);
            }
        }
    }
}

#[allow(clippy::identity_op)]
pub(crate) const DEFAULT_LAYER_SIZES: &[u64] = &[
    100 * 1024,        // 100KB
    1 * 1024 * 1024,   // 1MB
    100 * 1024 * 1024, // 100MB
];

unsafe fn do_merge(indexrel: PgSearchRelation) -> (NumCandidates, NumMerged) {
    let indexrel = {
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

    let index_options = SearchIndexOptions::from_relation(&indexrel);
    let merge_policy = LayeredMergePolicy::new(index_options.layer_sizes());

    merge_index_with_policy(&indexrel, merge_policy, false, false, false)
}

pub unsafe fn merge_index_with_policy(
    indexrel: &PgSearchRelation,
    mut merge_policy: LayeredMergePolicy,
    verbose: bool,
    gc_after_merge: bool,
    consider_create_index_segments: bool,
) -> (NumCandidates, NumMerged) {
    // take a shared lock on the CLEANUP_LOCK and hold it until this function is done.  We keep it
    // locked here so we can cause `ambulkdelete()` to block, waiting for all merging to finish
    // before it decides to find the segments it should vacuum.  The reason is that it needs to see
    // the final merged segment, not the original segments that will be deleted
    let metadata = MetaPage::open(indexrel);
    let cleanup_lock = metadata.cleanup_lock();
    let merge_lock = metadata.acquire_merge_lock();
    let directory = MvccSatisfies::Mergeable.directory(indexrel);
    let merger =
        SearchIndexMerger::open(directory).expect("should be able to open a SearchIndexMerger");
    let merger_segment_ids = merger
        .searchable_segment_ids()
        .expect("SearchIndexMerger should have segment ids");

    // the non_mergeable_segments are those that are concurrently being vacuumed *and* merged
    let mut non_mergeable_segments = metadata.vacuum_list().read_list();
    non_mergeable_segments.extend(merge_lock.merge_list().list_segment_ids());
    let create_index_segment_ids = metadata.create_index_segment_ids();

    if pg_sys::message_level_is_interesting(pg_sys::DEBUG1 as _) {
        pgrx::debug1!("do_merge: non_mergeable_segments={non_mergeable_segments:?}");
        pgrx::debug1!("do_merge: merger_segment_ids={merger_segment_ids:?}");
        pgrx::debug1!("do_merge: create_index_segment_ids={create_index_segment_ids:?}");
    }

    // tell the MergePolicy which segments it's initially allowed to consider for merging
    merge_policy.set_mergeable_segment_entries(merger.all_entries().into_iter().filter(
        |(segment_id, entry)| {
            // skip segments that are already being vacuumed or merged
            if non_mergeable_segments.contains(segment_id) {
                return false;
            }

            // skip segments that were created by CREATE INDEX and have no deletes
            if !consider_create_index_segments
                && create_index_segment_ids.contains(segment_id)
                && entry
                    .delete
                    .is_none_or(|delete_entry| delete_entry.num_deleted_docs == 0)
            {
                return false;
            }

            true
        },
    ));

    // further reduce the set of segments that the LayeredMergePolicy will operate on by internally
    // simulating the process, allowing concurrent merges to consider segments we're not, only retaining
    // the segments it decides can be merged into one or more candidates
    let (merge_candidates, nmerged) = merge_policy.simulate();

    // before we start merging, tell the merger to release pins on the segments it won't be merging
    let mut merger = merger
        .adjust_pins(merge_policy.mergeable_segments())
        .expect("should be table to adjust merger pins");

    let mut need_gc = !gc_after_merge;
    let ncandidates = merge_candidates.len();
    if ncandidates > 0 {
        // record all the segments the SearchIndexMerger can see, as those are the ones that
        // could be merged
        let merge_entry = merge_lock
            .merge_list()
            .add_segment_ids(merge_policy.mergeable_segments())
            .expect("should be able to write current merge segment_id list");
        drop(merge_lock);

        // we are NOT under the MergeLock at this point, which allows concurrent backends to also merge
        //
        // we defer raising a panic in the face of a merge error as we need to remove the created
        // `merge_entry` whether the merge worked or not

        let mut merge_result: anyhow::Result<Option<SegmentMeta>> = Ok(None);

        if !verbose {
            // happy path
            for candidate in merge_candidates {
                merge_result = merger.merge_segments(&candidate.0);
                if merge_result.is_err() {
                    break;
                }
                if gc_after_merge {
                    garbage_collect_index(indexrel);
                    need_gc = false;
                }
            }
        } else {
            // verbose path
            pgrx::warning!(
                "merging {} candidates, totalling {} segments",
                ncandidates,
                nmerged
            );

            for (i, candidate) in merge_candidates.into_iter().enumerate() {
                pgrx::warning!(
                    "merging candidate #{}:  {} segments",
                    i + 1,
                    candidate.0.len()
                );

                let start = std::time::Instant::now();
                merge_result = match merger.merge_segments(&candidate.0) {
                    Ok(Some(segment_meta)) => {
                        pgrx::warning!(
                            "   finished merge in {:?}.  final num_docs={}",
                            start.elapsed(),
                            segment_meta.num_docs()
                        );
                        Ok(Some(segment_meta))
                    }
                    Ok(None) => {
                        pgrx::warning!(
                            "   finished merge in {:?}.  merged to nothing",
                            start.elapsed()
                        );
                        Ok(None)
                    }
                    Err(e) => Err(e),
                };

                if merge_result.is_err() {
                    break;
                }

                if gc_after_merge {
                    garbage_collect_index(indexrel);
                    need_gc = false;
                }
            }
        }

        // re-acquire the MergeLock to remove the entry we made above
        let merge_lock = metadata.acquire_merge_lock();
        merge_lock
            .merge_list()
            .remove_entry(merge_entry)
            .expect("should be able to remove MergeEntry");
        drop(merge_lock);

        // we can garbage collect and return blocks back to the FSM without being under the MergeLock
        if need_gc {
            garbage_collect_index(indexrel);
        }

        // if merging was cancelled due to a legit interrupt we'd prefer that be provided to the user
        check_for_interrupts!();

        if let Err(e) = merge_result {
            panic!("failed to merge: {e:?}");
        }
    } else {
        drop(merge_lock);
    }
    drop(cleanup_lock);

    (ncandidates, nmerged)
}

///
/// Garbage collect the segments, removing any which are no longer visible in transactions
/// occurring in this process.
///
/// If physical replicas might still be executing transactions on some segments, then they are
/// moved to the `SEGMENT_METAS_GARBAGE` list until those replicas indicate that they are no longer
/// in use, at which point they can be freed by `free_garbage`.
///
pub unsafe fn garbage_collect_index(indexrel: &PgSearchRelation) {
    // Remove items which are no longer visible to active local transactions from SEGMENT_METAS,
    // and place them in SEGMENT_METAS_RECYLCABLE until they are no longer visible to remote
    // transactions either.
    //
    // SEGMENT_METAS must be updated atomically so that a consistent list is visible for consumers:
    // SEGMENT_METAS_GARBAGE need not be because it is only ever consumed on the physical
    // replication primary.
    let mut segment_metas_linked_list = MetaPage::open(indexrel).segment_metas();
    let mut segment_metas = segment_metas_linked_list.atomically();
    let entries = segment_metas.garbage_collect();

    // Replication is not enabled: immediately free the entries. It doesn't matter when we
    // commit the segment metas list in this case.
    segment_metas.commit();
    free_entries(indexrel, entries);
}

pub fn free_entries(indexrel: &PgSearchRelation, freeable_entries: Vec<SegmentMetaEntry>) {
    for entry in freeable_entries {
        for (file_entry, _) in entry.file_entries() {
            unsafe {
                LinkedBytesList::open(indexrel, file_entry.starting_block).return_to_fsm();
            }
        }
    }

    unsafe {
        pg_sys::IndexFreeSpaceMapVacuum(indexrel.as_ptr());
    }
}

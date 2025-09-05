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

use std::panic::{catch_unwind, resume_unwind};

use crate::index::writer::index::SerialIndexWriter;
use crate::postgres::merge::{do_merge, MergeStyle};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{
    MutableSegmentEntry, SegmentMetaEntry, SegmentMetaEntryContent, SegmentMetaEntryMutable,
};
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::utils::item_pointer_to_u64;

use pgrx::{pg_guard, pg_sys, PgMemoryContexts};
use tantivy::index::SegmentId;

// TODO: GUC
const MAX_MUTABLE_SEGMENT_SIZE: u32 = 100;

/// TODO: Write as Mutable until we hit a threshold, and then switch to Immutable?
enum InsertMode {
    /*
    Immutable {
        write: SerialIndexWriter,
        categorized_fields: Vec<(SearchField, CategorizedFieldData)>,
        key_field_name: FieldName,
    },
    */
    Mutable { ctids: Vec<u64> },
}

pub struct InsertState {
    #[allow(dead_code)] // field is used by pg<16 for the fakeaminsertcleanup stuff
    indexrelid: pg_sys::Oid,
    indexrel: PgSearchRelation,
    per_row_context: PgMemoryContexts,
    mode: InsertMode,
}

impl InsertState {
    unsafe fn new(indexrel: &PgSearchRelation) -> anyhow::Result<Self> {
        /*
        let schema = indexrel.schema()?;
        let categorized_fields = schema.categorized_fields().clone();
        let key_field_name = schema.key_field_name();
        */

        let per_row_context = pg_sys::AllocSetContextCreateExtended(
            PgMemoryContexts::CurrentMemoryContext.value(),
            c"pg_search aminsert context".as_ptr(),
            pg_sys::ALLOCSET_DEFAULT_MINSIZE as usize,
            pg_sys::ALLOCSET_DEFAULT_INITSIZE as usize,
            pg_sys::ALLOCSET_DEFAULT_MAXSIZE as usize,
        );

        Ok(Self {
            indexrelid: indexrel.oid(),
            mode: InsertMode::Mutable { ctids: Vec::new() },
            indexrel: indexrel.clone(),
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
    _values: *mut pg_sys::Datum,
    _isnull: *mut bool,
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

        match &mut state.mode {
            InsertMode::Mutable { ctids } => state.per_row_context.switch_to(|cxt| {
                ctids.push(item_pointer_to_u64(*ctid));
                cxt.reset();
                true
            }),
        }
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

    let entries = match &mut (*state).mode {
        InsertMode::Mutable { ctids } => ctids
            .drain(..)
            .map(|ctid| MutableSegmentEntry { ctid })
            .collect::<Vec<_>>(),
    };

    let mut segment_metas = MetaPage::open(&(*state).indexrel).segment_metas();

    // Attempt to insert into an existing mutable segment.
    // TODO: Validate that we're acquiring whatever locks are necessary to block `save_metas` from
    // completely swapping the list out from under us.
    // TODO: We're holding two write locks at a time here.
    let inserted = segment_metas.update_item(
        |entry| {
            // TODO: Introduce freezing of entries, and skip them here.
            matches!(entry.content, SegmentMetaEntryContent::Mutable(_) if entry.max_doc() < MAX_MUTABLE_SEGMENT_SIZE)
        },
        |entry| {
            entry.increment_max_doc(entries.len().try_into().unwrap());

            let SegmentMetaEntryContent::Mutable(content) = entry.content else {
                panic!("update_item returned the wrong item.")
            };

            content.open(&(*state).indexrel).add_items(&entries, None);
        },
    );

    // If we didn't find an existing mutable segment, create a new one.
    // TODO: `lookup_ex` and `update_item` should probably return an `Option` rather than a
    // `Result`.
    if inserted.is_err() {
        let (content, mut items) = SegmentMetaEntryMutable::create(&(*state).indexrel);
        items.add_items(&entries, None);
        let entry = SegmentMetaEntry::new_mutable(
            SegmentId::generate_random(),
            entries.len().try_into().unwrap(),
            pg_sys::InvalidTransactionId,
            content,
        );
        segment_metas.add_items(&[entry], None);
    }
}

#[allow(dead_code)] // TODO
pub fn paradedb_aminsertcleanup(mut writer: Option<SerialIndexWriter>) {
    if let Some(writer) = writer.take() {
        if let Some((_, indexrel)) = writer
            .commit()
            .expect("must be able to commit inserts in paradedb_aminsertcleanup")
        {
            /*
             * Recompute VACUUM XID boundaries.
             *
             * We don't actually care about the oldest non-removable XID.  Computing
             * the oldest such XID has a useful side-effect that we rely on: it
             * forcibly updates the XID horizon state for this backend.  This step is
             * essential; GlobalVisCheckRemovableFullXid() will not reliably recognize
             * that it is now safe to recycle newly deleted pages without this step.
             */
            unsafe {
                let heaprel = indexrel
                    .heap_relation()
                    .expect("index should belong to a heap relation");
                pg_sys::GetOldestNonRemovableTransactionId(heaprel.as_ptr());
            }

            unsafe {
                do_merge(
                    &indexrel,
                    MergeStyle::Insert,
                    Some(pg_sys::GetCurrentTransactionId()),
                )
                .expect("should be able to merge");
            }
        }
    }
}

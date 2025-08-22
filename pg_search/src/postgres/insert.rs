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
use crate::index::mvcc::MvccSatisfies;
use crate::index::writer::index::{IndexWriterConfig, SerialIndexWriter};
use crate::postgres::merge::{do_merge, MergeStyle};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::{item_pointer_to_u64, row_to_search_document};
use crate::schema::{CategorizedFieldData, SearchField};
use pgrx::{pg_guard, pg_sys, PgMemoryContexts};
use std::panic::{catch_unwind, resume_unwind};
use tantivy::TantivyDocument;

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
        let categorized_fields = schema.categorized_fields().clone();
        let key_field_name = schema.key_field_name();

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

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

use crate::api::FieldName;
use crate::gucs;
use crate::index::mvcc::MvccSatisfies;
use crate::index::writer::index::{IndexError, IndexWriterConfig, SerialIndexWriter};
use crate::postgres::merge::{do_merge, MergeStyle};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{
    MutableSegmentEntry, SegmentMetaEntry, SegmentMetaEntryContent, SegmentMetaEntryMutable,
};
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::utils::{item_pointer_to_u64, row_to_search_document};
use crate::postgres::IsLogicalWorker;
use crate::schema::{CategorizedFieldData, SearchField};

use pgrx::{pg_guard, pg_sys, PgMemoryContexts};
use tantivy::index::SegmentId;
use tantivy::TantivyDocument;

pub struct InsertModeImmutable {
    writer: Box<SerialIndexWriter>,
    categorized_fields: Vec<(SearchField, CategorizedFieldData)>,
}

impl InsertModeImmutable {
    fn new(indexrel: &PgSearchRelation) -> anyhow::Result<Self> {
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
        let schema = indexrel.schema()?;
        let categorized_fields = schema.categorized_fields().clone();
        Ok(Self {
            writer: Box::new(writer),
            categorized_fields,
        })
    }
}

pub struct InsertModeMutable {
    ctids: Vec<u64>,
    key_field_name: FieldName,
    key_field_attno: usize,
    row_limit: usize,
}

pub enum InsertMode {
    Immutable(InsertModeImmutable),
    Mutable(InsertModeMutable),
    Completed,
}

pub struct InsertState {
    #[allow(dead_code)] // field is used by pg<16 for the fakeaminsertcleanup stuff
    pub indexrelid: pg_sys::Oid,
    indexrel: PgSearchRelation,
    per_row_context: PgMemoryContexts,
    pub mode: InsertMode,
}

impl InsertState {
    unsafe fn new(indexrel: &PgSearchRelation) -> anyhow::Result<Self> {
        let per_row_context = pg_sys::AllocSetContextCreateExtended(
            if cfg!(any(feature = "pg17", feature = "pg18")) {
                if IsLogicalWorker() {
                    PgMemoryContexts::TopTransactionContext.value()
                } else {
                    PgMemoryContexts::CurrentMemoryContext.value()
                }
            } else {
                PgMemoryContexts::CurrentMemoryContext.value()
            },
            c"pg_search aminsert context".as_ptr(),
            pg_sys::ALLOCSET_DEFAULT_MINSIZE as usize,
            pg_sys::ALLOCSET_DEFAULT_INITSIZE as usize,
            pg_sys::ALLOCSET_DEFAULT_MAXSIZE as usize,
        );

        let mode = if let Some(row_limit) = indexrel.options().mutable_segment_rows() {
            let (key_field_name, key_field_attno) = indexrel
                .schema()?
                .categorized_fields()
                .iter()
                .find(|(_, categorized_field)| categorized_field.is_key_field)
                .map(|(search_field, categorized_field)| {
                    (search_field.field_name().clone(), categorized_field.attno)
                })
                .expect("No key field defined.");

            InsertMode::Mutable(InsertModeMutable {
                ctids: Vec::new(),
                key_field_name,
                key_field_attno,
                row_limit: row_limit.into(),
            })
        } else {
            InsertMode::Immutable(InsertModeImmutable::new(indexrel)?)
        };

        Ok(Self {
            indexrelid: indexrel.oid(),
            indexrel: indexrel.clone(),
            mode,
            per_row_context: PgMemoryContexts::For(per_row_context),
        })
    }
}

#[cfg(not(any(feature = "pg17", feature = "pg18")))]
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

#[cfg(any(feature = "pg17", feature = "pg18"))]
#[allow(static_mut_refs)]
unsafe fn logical_worker_state() -> &'static mut rustc_hash::FxHashMap<pg_sys::Oid, InsertState> {
    static mut LOGICAL_WORKER_STATE: Option<rustc_hash::FxHashMap<pg_sys::Oid, InsertState>> = None;

    if LOGICAL_WORKER_STATE.is_none() {
        LOGICAL_WORKER_STATE = Some(Default::default());
        pgrx::register_xact_callback(pgrx::PgXactCallbackEvent::PreCommit, || {
            // on transaction commit, take ownership of the LOGICAL_WORKER_STATE,
            // running each `InsertState` it contains through the normal `paradedb_aminsertcleanup()`
            // process.  Effectively deferring tantivy index commits to the end of the postgres transaction
            if let Some(lwstate) = LOGICAL_WORKER_STATE.take() {
                for (_, mut insert_state) in lwstate {
                    let mode = std::mem::replace(&mut insert_state.mode, InsertMode::Completed);
                    insertcleanup(&insert_state, mode);
                }
            }
        });
        pgrx::register_xact_callback(pgrx::PgXactCallbackEvent::Abort, || {
            LOGICAL_WORKER_STATE = None
        });
    }

    LOGICAL_WORKER_STATE.as_mut().unwrap()
}

#[cfg(any(feature = "pg17", feature = "pg18"))]
pub unsafe fn init_insert_state(
    index_relation: pg_sys::Relation,
    index_info: &mut pg_sys::IndexInfo,
) -> &mut InsertState {
    if IsLogicalWorker() {
        logical_worker_state()
            .entry((*index_relation).rd_id)
            .or_insert_with(|| {
                // When in a Logical Apply Worker, we need to keep the index open through the entire
                // transaction up to the PRE_COMMIT hook, where we do our final tantivy commit and cleanup.
                // This is because Postgres closes relations earlier, before our final cleanup work is complete.
                let index_relation = PgSearchRelation::with_lock(
                    (*index_relation).rd_id,
                    pg_sys::AccessShareLock as pg_sys::LOCKMODE,
                );
                InsertState::new(&index_relation)
                    .expect("should be able to open new SearchIndex for writing")
            })
    } else {
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
            pgrx::PgMemoryContexts::For(index_info.ii_Context).switch_to(|mcxt| {
                index_info.ii_AmCache = mcxt.leak_and_drop_on_delete(state).cast()
            })
        };

        &mut *index_info.ii_AmCache.cast()
    }
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
    aminsert_internal(index_relation, values, isnull, ctid, index_info)
}

#[inline(always)]
unsafe fn aminsert_internal(
    index_relation: pg_sys::Relation,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    ctid: pg_sys::ItemPointer,
    index_info: *mut pg_sys::IndexInfo,
) -> bool {
    #[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
    {
        // Postgres 17 introduced the `aminsertcleanup()` function, which is critical for logical
        // replication to work.  As such, if this isn't v17+ and we're being called from a logical
        // worker, we must fail.  Users will need to upgrade to v17 to support logical replication
        if IsLogicalWorker() {
            panic!("pg_search logical replication is only supported on Postgres v17+")
        }
    }

    let result = catch_unwind(|| {
        let state = init_insert_state(
            index_relation,
            index_info
                .as_mut()
                .expect("index_info argument must not be null"),
        );

        unsafe {
            insert(state, values, isnull, item_pointer_to_u64(*ctid));
        };
    });

    match result {
        Ok(()) => true,
        Err(e) => resume_unwind(e),
    }
}

unsafe fn insert(
    state: &mut InsertState,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    ctid: u64,
) {
    match &mut state.mode {
        InsertMode::Immutable(mode) => state.per_row_context.switch_to(|cxt| {
            let mut search_document = TantivyDocument::new();

            row_to_search_document(
                mode.categorized_fields.iter().map(|(field, categorized)| {
                    let index_attno = categorized.attno;
                    (
                        *values.add(index_attno),
                        *isnull.add(index_attno),
                        field,
                        categorized,
                    )
                }),
                &mut search_document,
            )
            .unwrap_or_else(|err| panic!("{err}"));
            mode.writer
                .insert(search_document, ctid, || {})
                .expect("insertion into index should succeed");

            cxt.reset();
        }),
        InsertMode::Mutable(mode) => {
            if *isnull.add(mode.key_field_attno) {
                panic!("{}", IndexError::KeyIdNull(mode.key_field_name.to_string()));
            }

            if mode.ctids.len() < mode.row_limit {
                mode.ctids.push(ctid);
                return;
            }

            // A large number of inserts have already occurred within this aminsert series:
            // switch modes for this insert, based on the assumption that more are likely
            // on the way.
            //
            // Swap in the new mode, cleanup the old one, and then recurse to insert in the new
            // mode.
            let new_mode = InsertMode::Immutable(
                InsertModeImmutable::new(&state.indexrel)
                    .expect("failed to open index for writing"),
            );
            let old_mode = std::mem::replace(&mut state.mode, new_mode);
            insertcleanup(state, old_mode);
            insert(state, values, isnull, ctid);
        }
        InsertMode::Completed => {
            panic!("aminsertcleanup was already called.");
        }
    }
}

#[cfg(any(feature = "pg17", feature = "pg18"))]
#[pg_guard]
pub unsafe extern "C-unwind" fn aminsertcleanup(
    _index_relation: pg_sys::Relation,
    index_info: *mut pg_sys::IndexInfo,
) {
    if IsLogicalWorker() {
        // do nothing -- doing the work of "aminsertcleanup()" is handled by the commit hook
        // added in `logical_worker_state()`
    } else {
        let state = (*index_info).ii_AmCache.cast::<InsertState>();
        if state.is_null() {
            return;
        }
        let Some(state) = state.as_mut() else {
            return;
        };

        let mode = std::mem::replace(&mut state.mode, InsertMode::Completed);
        insertcleanup(state, mode);
    }
}

pub fn insertcleanup(state: &InsertState, mode: InsertMode) {
    let created_segment = match mode {
        InsertMode::Immutable(mode) => insertcleanup_immutable(mode),
        InsertMode::Mutable(mode) => unsafe { insertcleanup_mutable(&state.indexrel, mode) },
        InsertMode::Completed => {
            panic!("insertcleanup was called twice.");
        }
    };

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
        let heaprel = state
            .indexrel
            .heap_relation()
            .expect("index should belong to a heap relation");
        pg_sys::GetOldestNonRemovableTransactionId(heaprel.as_ptr());

        if created_segment {
            do_merge(
                &state.indexrel,
                MergeStyle::Insert,
                Some(pg_sys::GetCurrentFullTransactionId()),
                Some(pg_sys::ReadNextFullTransactionId()),
            )
            .expect("should be able to merge");
        }
    }
}

fn insertcleanup_immutable(mode: InsertModeImmutable) -> bool {
    mode.writer
        .commit()
        .expect("must be able to commit inserts in insertcleanup");
    true
}

unsafe fn insertcleanup_mutable(indexrel: &PgSearchRelation, mode: InsertModeMutable) -> bool {
    let entries = mode
        .ctids
        .into_iter()
        .map(MutableSegmentEntry::Add)
        .collect::<Vec<_>>();

    let mut segment_metas = MetaPage::open(indexrel).segment_metas();

    // Attempt to insert into an existing mutable segment.
    let inserted = segment_metas.update_item(
        |entry| {
            matches!(entry.content, SegmentMetaEntryContent::Mutable(content) if !content.frozen)
        },
        |entry| {
            entry.mutable_add_items(indexrel, &entries).expect("update_item guard not executed properly")
        },
    );

    // TODO: `lookup_ex` and `update_item` should probably return an `Option` rather than a
    // `Result`.
    if inserted.is_ok() {
        return false;
    }

    // If we didn't find an existing mutable segment, create a new one.
    let (content, mut items) = SegmentMetaEntryMutable::create(indexrel);
    items.add_items(&entries, None);
    let entry = SegmentMetaEntry::new_mutable(
        SegmentId::generate_random(),
        entries.len().try_into().unwrap(),
        pg_sys::InvalidTransactionId,
        content,
    );
    segment_metas.add_items(&[entry], None);

    true
}

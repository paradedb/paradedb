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
#![allow(static_mut_refs)]
use crate::api::HashSet;
use crate::index::mvcc::MVCCDirectory;
use crate::postgres::merge::free_entries;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::SegmentMetaEntry;
use crate::postgres::storage::linked_items;
use crate::postgres::storage::metadata::MetaPage;
use pgrx::pg_sys::{self, panic::ErrorReport};
use pgrx::{function_name, PgLogLevel, PgSqlErrorCode};
use rustc_hash::FxHashMap;
use tantivy::{index::SegmentId, Index};

///
/// Validates that a hot standby is properly configured, ideally at most once per process.
///
pub fn abort_for_mismatched_segments(missing: &HashSet<SegmentId>, found: HashSet<&SegmentId>) {
    if unsafe { pgrx::pg_sys::HotStandbyActive() } {
        pgrx::warning!(
            "load_metas: MvccSatisfies::ParallelWorker didn't load the correct segments. \
            found={found:?}, missing={missing:?}",
        );
        ErrorReport::new(
            PgSqlErrorCode::ERRCODE_QUERY_CANCELED,
            "cancelling query due to replication inconsistency",
            function_name!(),
        )
        .set_detail("detected an inconsistent index segment structure due to mis-configuration")
        .set_hint(
            "when `pg_search` is operating as a `hot_standby`, \
            the `hot_standby_feedback` and `primary_slot_name` settings must be set",
        )
        .report(PgLogLevel::ERROR);
    } else {
        panic!(
            "load_metas: MvccSatisfies::ParallelWorker didn't load the correct segments. \
            found={found:?}, missing={missing:?}",
        );
    }
}

///
/// When `hot_standby`s are active and sending feedback, entries which are removed from
/// SEGMENT_METAS_START are moved to the garbage list until `hot_standby_feedback` indicates that
/// they can be freed.
///
pub fn add_to_garbage(
    indexrel: &PgSearchRelation,
    mut entries: Vec<SegmentMetaEntry>,
    current_xid: pg_sys::FullTransactionId,
) {
    // The entries have an `xmax` value of FrozenTransactionId, because that is our marker for
    // deleted entries in SEGMENT_METAS. But we need a useful xmax value to compare to in the
    // garbage list, so reset the xmax here.
    for entry in &mut entries {
        entry.set_xmax(pg_sys::TransactionId::from_inner(current_xid.value as u32));
    }
    unsafe {
        if let Some(mut garbage) = MetaPage::open(indexrel).segment_metas_garbage() {
            garbage.add_items(&entries, None);
        }
    }
}

///
/// Frees segments which were previously added to the `SEGMENT_METAS_GARBAGE` list, but which are
/// no longer visible on any physical replicas.
///
pub fn free_garbage(
    indexrel: &PgSearchRelation,
    current_xid: pg_sys::FullTransactionId,
) -> Option<()> {
    // TODO: Ditto doing a direct check on replication slots: if no slots exist, then we'll want
    // to clean up any items that exist in the list. But if they do exist and we just stopped
    // getting feedback, then we don't want to clean up until we have.
    let Some(full_hot_standby_feedback_xmin) = feedback_xmin() else {
        // No replication, or no feedback from the hot standbys. Do nothing.
        return None;
    };

    let hot_standby_feedback_xmin =
        pg_sys::TransactionId::from_inner(full_hot_standby_feedback_xmin.value as u32);

    let is_freeable = |entry: &SegmentMetaEntry| -> bool {
        let xmax = entry.xmax();
        assert!(
            xmax != pg_sys::InvalidTransactionId,
            "No entry in segment_metas_garbage should be live: {entry:?}",
        );
        unsafe { pg_sys::TransactionIdPrecedes(xmax, hot_standby_feedback_xmin) }
    };

    // Items which are no longer visible on any physical replica primary can be freed.
    let freeable_entries = unsafe {
        let mut garbage_list = MetaPage::open(indexrel).segment_metas_garbage()?;

        // `retain` acquires mutable buffers which must then be flushed: start by seeing whether there
        // are any entries that are freeable.
        let mut any_freeable = false;
        garbage_list.for_each(|_bman, entry| any_freeable |= is_freeable(&entry));

        if !any_freeable {
            // No freeable entries.
            return None;
        }

        // At least one entry might be freeable: filter them out of the list.
        garbage_list.retain(pg_sys::ReadNextFullTransactionId(), |_bman, entry| {
            if is_freeable(&entry) {
                linked_items::RetainItem::Remove(entry)
            } else {
                linked_items::RetainItem::Retain
            }
        })
    };

    free_entries(indexrel, freeable_entries, current_xid);
    Some(())
}

/// A wrapper around Postgres' [`pg_sys::ProcArrayGetReplicationSlotXmin`] function.
///
/// Return the current slot xmin limits. That's useful to be able to remove
/// data that's older than those limits.
pub fn feedback_xmin() -> Option<pg_sys::FullTransactionId> {
    unsafe {
        // #define EpochFromFullTransactionId(x)	((uint32) ((x).value >> 32))
        #[inline]
        fn epoch_from_full_transaction_id(xid: pg_sys::FullTransactionId) -> u32 {
            (xid.value >> 32) as u32
        }

        let mut xmin = pg_sys::InvalidTransactionId;
        pg_sys::ProcArrayGetReplicationSlotXmin(&mut xmin, std::ptr::null_mut());

        if xmin == pg_sys::InvalidTransactionId {
            return None;
        }

        let current_xid = pg_sys::GetCurrentFullTransactionId();
        let epoch = epoch_from_full_transaction_id(current_xid);
        Some(pg_sys::FullTransactionIdFromEpochAndXid(epoch, xmin))
    }
}

///
/// Meant to be run from a WAL receiver, this function checks to see if new deletes have appeared
/// since the parallel scan had started. If so, it means a vacuum was run on the primary + the visibility
/// was updated. We need to cancel the current query to prevent deleted ctids that this query is about to return
// from showing up as visible, leading to incorrect results.
///
pub unsafe fn check_for_concurrent_vacuum(
    indexrel: &PgSearchRelation,
    old_segments: FxHashMap<SegmentId, u32>,
) {
    let directory =
        MVCCDirectory::parallel_worker(indexrel, old_segments.keys().cloned().collect());
    let index = Index::open(directory).expect("end_custom_scan: should be able to open index");
    let new_metas = index
        .searchable_segment_metas()
        .expect("end_custom_scan: should be able to get segment metas");

    let new_segments: FxHashMap<_, _> = new_metas
        .iter()
        .map(|meta| (meta.id(), meta.num_deleted_docs()))
        .collect();

    for (segment_id, num_deleted_docs) in old_segments {
        if new_segments.get(&segment_id).unwrap_or(&0) != &num_deleted_docs {
            ErrorReport::new(
                PgSqlErrorCode::ERRCODE_QUERY_CANCELED,
                "cancelling query due to conflict with vacuum",
                function_name!(),
            )
            .set_detail("a concurrent vacuum operation on the WAL sender is running")
            .set_hint("retry the query when the vacuum operation has completed")
            .report(PgLogLevel::ERROR);
        }
    }
}

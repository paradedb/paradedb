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
#![allow(unsafe_op_in_unsafe_fn)]

use crate::api::HashSet;
use crate::index::merge_policy::LayeredMergePolicy;
use crate::index::mvcc::MvccSatisfies;
use crate::index::writer::index::SearchIndexMerger;
use crate::postgres::merge::{MergeMode, merge_index};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::SegmentMetaEntry;
use crate::postgres::storage::coordination_test::{live_segments, merge_pause, selection_pause};
use crate::postgres::storage::metadata::MetaPage;
use pgrx::{PgRelation, pg_extern, pg_sys};
use tantivy::index::SegmentId;

/// Runs a foreground `merge_index` whose single layer admits every live segment, and returns how
/// many of the segments live beforehand it consumed. A lone mutable segment converted to an
/// immutable one counts as one; zero means selection found no candidate.
#[pg_extern]
unsafe fn test_merge_index(index: PgRelation) -> i64 {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    let metadata = MetaPage::open(&index);
    let before = live_segments(&metadata);
    let layer_size = before
        .iter()
        .map(|entry| entry.byte_size())
        .max()
        .expect("fixture must have segments");
    let before: HashSet<SegmentId> = before.iter().map(SegmentMetaEntry::segment_id).collect();
    let merge_pins = metadata.pin_merge();
    let merge_lock = metadata.acquire_merge_lock();
    let merger = SearchIndexMerger::open(&index, MvccSatisfies::Mergeable).unwrap();
    selection_pause();
    let mut policy = LayeredMergePolicy::new(vec![layer_size]);
    policy.set_mergeable_segment_entries(&metadata, &merge_lock, &merger);
    let (candidates, _, _) = policy.simulate_supported(&merger).unwrap();
    let candidates = candidates
        .into_iter()
        .map(|(_, candidate)| candidate)
        .collect::<Vec<_>>();
    let selected: HashSet<SegmentId> = candidates
        .iter()
        .flat_map(|candidate| candidate.0.iter().copied())
        .collect();
    merge_index(
        &index,
        merger,
        &candidates,
        merge_lock,
        merge_pins,
        MergeMode::Foreground,
        pg_sys::GetCurrentFullTransactionId(),
        pg_sys::ReadNextFullTransactionId(),
    );
    let after: HashSet<SegmentId> = live_segments(&MetaPage::open(&index))
        .iter()
        .map(SegmentMetaEntry::segment_id)
        .collect();
    before
        .difference(&after)
        .filter(|id| selected.contains(*id))
        .count() as i64
}

#[pg_extern]
unsafe fn test_vacuum_active(index: PgRelation) -> bool {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    MetaPage::open(&index)
        .vacuum_list()
        .is_ambulkdelete_running()
}

#[pg_extern]
unsafe fn test_live_docs(index: PgRelation) -> i64 {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    live_segments(&MetaPage::open(&index))
        .iter()
        .map(|entry| entry.num_docs() as i64)
        .sum()
}

#[pg_extern]
unsafe fn test_hold_merger(index: PgRelation, fail: bool) -> i32 {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    let metadata = MetaPage::open(&index);
    let _merge_pins = metadata.pin_merge();
    drop(metadata.acquire_merge_lock());
    merge_pause();
    if fail {
        pgrx::error!("injected merge-pin ERROR");
    }
    pg_sys::InterruptHoldoffCount as i32
}

#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;

    /// Target for `pgrx_tests::run_test` so `tests/merge_cleanup.rs` can start the test server
    /// without running a real test as a side effect.
    #[pg_test]
    fn bootstrap_test_server() {}
}

#[pg_extern]
unsafe fn test_hold_cleanup_admission(index: PgRelation) {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    let metadata = MetaPage::open(&index);
    let _admission = metadata.cleanup_lock_for_vacuum();
    merge_pause();
}

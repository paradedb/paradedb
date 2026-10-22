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

use crate::index::directory::mvcc::MvccSatisfies;
use crate::index::merge_policy::LayeredMergePolicy;
use crate::index::writer::index::{Mergeable, SearchIndexMerger};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::metadata::MetaPage;
use pgrx::{IntoDatum, PgRelation, direct_function_call, pg_extern, pg_sys};
use std::ffi::CStr;

unsafe fn pause(ready_name: &CStr, gate_name: &CStr) {
    let ready = pg_sys::GetConfigOptionByName(ready_name.as_ptr(), std::ptr::null_mut(), true);
    if ready.is_null() || CStr::from_ptr(ready).to_bytes().is_empty() {
        return;
    }
    // A set `_ready` without its `_gate` is a test bug; let the lookup ERROR.
    let gate = pg_sys::GetConfigOptionByName(gate_name.as_ptr(), std::ptr::null_mut(), false);
    for value in [ready, gate] {
        let key: i64 = CStr::from_ptr(value).to_str().unwrap().parse().unwrap();
        direct_function_call::<()>(pg_sys::pg_advisory_xact_lock_int8, &[key.into_datum()]);
    }
}

pub(crate) unsafe fn vacuum_pause() {
    pause(
        c"pg_search.test_vacuum_ready",
        c"pg_search.test_vacuum_gate",
    );
}

pub(crate) unsafe fn vacuum_setup_pause() {
    pause(
        c"pg_search.test_vacuum_setup_ready",
        c"pg_search.test_vacuum_setup_gate",
    );
}

pub(crate) unsafe fn merge_pause() {
    pause(c"pg_search.test_merge_ready", c"pg_search.test_merge_gate");
}

#[pg_extern]
unsafe fn test_merger_snapshot(index: PgRelation) -> i64 {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    let metadata = MetaPage::open(&index);
    let _merge_pins = metadata.pin_merge();
    let merge_lock = metadata.acquire_merge_lock();
    let merger = SearchIndexMerger::open(&index, MvccSatisfies::Mergeable).unwrap();
    let layer_size = merger
        .all_entries()
        .values()
        .map(|entry| entry.byte_size())
        .max()
        .expect("fixture must have segments");
    pause(
        c"pg_search.test_selection_ready",
        c"pg_search.test_selection_gate",
    );
    let mut policy = LayeredMergePolicy::new(vec![layer_size]);
    policy.set_mergeable_segment_entries(&metadata, &merge_lock, &merger);
    let (candidates, _, _) = policy.simulate_supported(&merger).unwrap();
    let Some(candidate) = candidates.into_iter().next() else {
        return 0;
    };
    let mut merger = merger.adjust_pins(candidate.0.iter()).unwrap();
    let entry = merge_lock
        .merge_list()
        .add_segment_ids(candidate.0.iter(), pg_sys::GetCurrentFullTransactionId())
        .unwrap();
    drop(merge_lock);
    let result = merger.merge_segments(&candidate.0).unwrap();
    metadata
        .acquire_merge_lock()
        .merge_list()
        .remove_entry(entry)
        .unwrap();
    result.map(|meta| meta.max_doc() as i64).unwrap_or(0)
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
    MetaPage::open(&index)
        .segment_metas()
        .list(None)
        .into_iter()
        .filter(|entry| entry.xmax() == pg_sys::InvalidTransactionId)
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

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
#![allow(clippy::missing_safety_doc, unsafe_op_in_unsafe_fn)]

use crate::postgres::storage::block::SegmentMetaEntry;
use crate::postgres::storage::metadata::MetaPage;
use pgrx::{IntoDatum, direct_function_call, pg_sys};
use std::ffi::CStr;

pub unsafe fn pause(ready_name: &CStr, gate_name: &CStr) {
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

pub unsafe fn vacuum_pause() {
    pause(
        c"pg_search.test_vacuum_ready",
        c"pg_search.test_vacuum_gate",
    );
}

pub unsafe fn vacuum_setup_pause() {
    pause(
        c"pg_search.test_vacuum_setup_ready",
        c"pg_search.test_vacuum_setup_gate",
    );
}

pub unsafe fn merge_pause() {
    pause(c"pg_search.test_merge_ready", c"pg_search.test_merge_gate");
}

/// Runs after the merger loaded segment metadata and before selection.
pub unsafe fn selection_pause() {
    pause(
        c"pg_search.test_selection_ready",
        c"pg_search.test_selection_gate",
    );
}

pub unsafe fn live_segments(metadata: &MetaPage) -> Vec<SegmentMetaEntry> {
    metadata
        .segment_metas()
        .list(None)
        .into_iter()
        .filter(|entry| entry.xmax() == pg_sys::InvalidTransactionId)
        .collect()
}

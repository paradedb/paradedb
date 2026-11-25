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

//! The scans of one partitioned table, which compute their window aggregates together.

use std::cell::RefCell;
use std::collections::HashMap;

use pgrx::{PgMemoryContexts, pg_guard, pg_sys};

use crate::postgres::customscan::basescan::BaseScan;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;

/// The scans of one partitioned table in one execution.
///
/// The planner gives every partition its own scan, but a window aggregate covers every row of the
/// query, so each of those scans has to project the aggregate over all the partitions (#6783).
/// The scans join one group when the executor starts them, and the first one to need the
/// aggregates computes them for the whole group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowAggGroupKey {
    table: u64,
    estate: usize,
}

struct WindowAggGroup {
    /// Each scan with the range table index of its partition.
    members: Vec<(pg_sys::Index, *mut CustomScanStateWrapper<BaseScan>)>,
    results: Option<crate::api::HashMap<usize, pg_sys::Datum>>,
}

thread_local! {
    static WINDOW_AGG_GROUPS: RefCell<HashMap<WindowAggGroupKey, WindowAggGroup>> =
        RefCell::default();
}

impl WindowAggGroupKey {
    /// The key of the table [`window_agg_group`] names, until the executor starts the scan.
    ///
    /// [`window_agg_group`]: crate::postgres::customscan::basescan::projections::window_agg::window_agg_group
    pub fn planned(table: u64) -> Self {
        Self { table, estate: 0 }
    }

    /// Add the scan of partition `rti` to the group of its execution.
    ///
    /// # Safety
    /// `member` must be the scan being started in `estate`, which outlives the group.
    pub unsafe fn join(
        &mut self,
        estate: *mut pg_sys::EState,
        rti: pg_sys::Index,
        member: *mut CustomScanStateWrapper<BaseScan>,
    ) {
        self.estate = estate as usize;
        let first_of_execution = WINDOW_AGG_GROUPS.with_borrow_mut(|groups| {
            let first = !groups.keys().any(|key| key.estate == self.estate);
            groups
                .entry(*self)
                .or_insert_with(|| WindowAggGroup {
                    members: Vec::new(),
                    results: None,
                })
                .members
                .push((rti, member));
            first
        });
        if first_of_execution {
            // The members end with the execution, whose query context goes at its end.
            let query_cxt = (*estate).es_query_cxt;
            let callback =
                PgMemoryContexts::For(query_cxt).palloc_struct::<pg_sys::MemoryContextCallback>();
            (*callback).func = Some(forget_window_agg_groups);
            (*callback).arg = estate.cast();
            pg_sys::MemoryContextRegisterResetCallback(query_cxt, callback);
        }
    }

    pub fn members(self) -> Vec<(pg_sys::Index, *mut CustomScanStateWrapper<BaseScan>)> {
        WINDOW_AGG_GROUPS.with_borrow(|groups| {
            groups
                .get(&self)
                .map(|group| group.members.clone())
                .unwrap_or_default()
        })
    }

    pub fn results(self) -> Option<crate::api::HashMap<usize, pg_sys::Datum>> {
        WINDOW_AGG_GROUPS.with_borrow(|groups| groups.get(&self)?.results.clone())
    }

    pub fn set_results(self, results: Option<crate::api::HashMap<usize, pg_sys::Datum>>) {
        WINDOW_AGG_GROUPS.with_borrow_mut(|groups| {
            if let Some(group) = groups.get_mut(&self) {
                group.results = results;
            }
        });
    }
}

#[pg_guard]
unsafe extern "C-unwind" fn forget_window_agg_groups(estate: *mut std::ffi::c_void) {
    WINDOW_AGG_GROUPS
        .with_borrow_mut(|groups| groups.retain(|key, _| key.estate != estate as usize));
}

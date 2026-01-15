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

use crate::api::HashMap;
use crate::index::reader::index::{MultiSegmentSearchResults, SearchIndexReader};
use crate::postgres::customscan::joinscan::build::JoinCSClause;
use crate::postgres::customscan::CustomScanState;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use pgrx::pg_sys;
use std::collections::VecDeque;

/// Represents an inner side row stored in the hash table.
#[derive(Debug, Clone)]
pub struct InnerRow {
    /// The ctid of the inner row.
    pub ctid: u64,
}

/// The execution state for the JoinScan.
#[derive(Default)]
pub struct JoinScanState {
    /// The join clause from planning.
    pub join_clause: JoinCSClause,

    // === Driving side state (side with search predicate - we iterate through this) ===
    /// The heap relation for the driving side.
    pub driving_heaprel: Option<PgSearchRelation>,
    /// The index relation for the driving side.
    pub driving_indexrel: Option<PgSearchRelation>,
    /// The search reader for the driving side.
    pub driving_search_reader: Option<SearchIndexReader>,
    /// The search results iterator for the driving side.
    pub driving_search_results: Option<MultiSegmentSearchResults>,
    /// Visibility checker for the driving side.
    pub driving_visibility_checker: Option<VisibilityChecker>,
    /// Slot for fetching driving side tuples.
    pub driving_fetch_slot: Option<*mut pg_sys::TupleTableSlot>,

    // === Build side state (side we build hash table from) ===
    /// The heap relation for the build side.
    pub build_heaprel: Option<PgSearchRelation>,
    /// Visibility checker for the build side.
    pub build_visibility_checker: Option<VisibilityChecker>,
    /// Heap scan descriptor for build side.
    pub build_scan_desc: Option<*mut pg_sys::TableScanDescData>,
    /// Slot for build side heap scan.
    pub build_scan_slot: Option<*mut pg_sys::TupleTableSlot>,

    // === Hash join state ===
    /// The hash table built from the build side.
    /// Key: join key value (as i64 for simple integer keys), Value: list of build row ctids.
    pub hash_table: HashMap<i64, Vec<InnerRow>>,
    /// Whether the hash table has been built.
    pub hash_table_built: bool,

    // === Probe state ===
    /// Current driving side ctid being probed.
    pub current_driving_ctid: Option<u64>,
    /// Current driving side score.
    pub current_driving_score: f32,
    /// Pending build side ctids that match the current driving row.
    pub pending_build_ctids: VecDeque<u64>,

    // === Result state ===
    /// Result tuple slot.
    pub result_slot: Option<*mut pg_sys::TupleTableSlot>,
    /// Count of rows returned.
    pub rows_returned: usize,

    // === Side tracking ===
    /// Whether the driving side is the outer side (true) or inner side (false).
    pub driving_is_outer: bool,
    /// Whether this is a cross join (no equi-join keys).
    pub is_cross_join: bool,

    // === Join condition evaluation ===
    /// Compiled join qual expression state for evaluating non-equijoin conditions.
    /// This is initialized from custom_exprs during begin_custom_scan.
    pub join_qual_state: Option<*mut pg_sys::ExprState>,
    /// Expression context for evaluating join quals.
    pub join_qual_econtext: Option<*mut pg_sys::ExprContext>,
}

impl JoinScanState {
    /// Reset the scan state for a rescan.
    pub fn reset(&mut self) {
        self.hash_table.clear();
        self.hash_table_built = false;
        self.current_driving_ctid = None;
        self.current_driving_score = 0.0;
        self.pending_build_ctids.clear();
        self.driving_search_results = None;
        self.rows_returned = 0;
    }

    /// Returns the limit from the join clause, if any.
    pub fn limit(&self) -> Option<usize> {
        self.join_clause.limit
    }

    /// Check if we've reached the limit.
    pub fn reached_limit(&self) -> bool {
        if let Some(limit) = self.limit() {
            self.rows_returned >= limit
        } else {
            false
        }
    }
}

impl CustomScanState for JoinScanState {
    fn init_exec_method(&mut self, _cstate: *mut pg_sys::CustomScanState) {
        // No special initialization needed for the plain exec method
    }
}

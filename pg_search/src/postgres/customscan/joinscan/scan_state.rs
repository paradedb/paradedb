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
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::joinscan::build::JoinCSClause;
use crate::postgres::customscan::CustomScanState;
use crate::postgres::rel::PgSearchRelation;
use pgrx::pg_sys;
use std::collections::VecDeque;

/// Represents a row from one side of the join, stored in the hash table.
#[derive(Debug, Clone)]
pub struct JoinRow {
    /// The ctid of the row (used to fetch heap tuple if needed).
    pub ctid: u64,
    /// The score from the search (if applicable).
    pub score: f32,
    /// Values of the join key columns (as raw bytes for hashing).
    pub join_key_values: Vec<Option<Vec<u8>>>,
}

/// The execution state for the JoinScan.
#[derive(Default)]
pub struct JoinScanState {
    /// The join clause from planning.
    pub join_clause: JoinCSClause,

    // === Outer side state ===
    /// The heap relation for the outer side.
    pub outer_heaprel: Option<PgSearchRelation>,
    /// The index relation for the outer side (if it has a BM25 index).
    pub outer_indexrel: Option<PgSearchRelation>,
    /// The search reader for the outer side (if it has a BM25 index with a query).
    pub outer_search_reader: Option<SearchIndexReader>,

    // === Inner side state ===
    /// The heap relation for the inner side.
    pub inner_heaprel: Option<PgSearchRelation>,
    /// The index relation for the inner side (if it has a BM25 index).
    pub inner_indexrel: Option<PgSearchRelation>,
    /// The search reader for the inner side (if it has a BM25 index with a query).
    pub inner_search_reader: Option<SearchIndexReader>,

    // === Hash join state ===
    /// The hash table built from the inner side (build side).
    /// Key: hash of join key values, Value: list of rows with that hash.
    pub hash_table: HashMap<u64, Vec<JoinRow>>,
    /// Whether the hash table has been built.
    pub hash_table_built: bool,

    // === Probe state ===
    /// Iterator over outer side rows (probe side).
    pub outer_iterator_started: bool,
    /// Current outer row being probed.
    pub current_outer_row: Option<JoinRow>,
    /// Pending matches from the hash table for the current outer row.
    pub pending_matches: VecDeque<JoinRow>,

    // === Result state ===
    /// The result tuple slot.
    pub result_slot: Option<*mut pg_sys::TupleTableSlot>,
    /// Count of rows returned.
    pub rows_returned: usize,
}

impl JoinScanState {
    /// Reset the scan state for a rescan.
    pub fn reset(&mut self) {
        self.hash_table.clear();
        self.hash_table_built = false;
        self.outer_iterator_started = false;
        self.current_outer_row = None;
        self.pending_matches.clear();
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

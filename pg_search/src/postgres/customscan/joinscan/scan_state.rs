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

//! Execution state for JoinScan custom scan.

use crate::postgres::customscan::joinscan::build::JoinCSClause;
use crate::postgres::customscan::joinscan::privdat::OutputColumnInfo;
use crate::postgres::customscan::CustomScanState;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use pgrx::pg_sys;

/// The execution state for the JoinScan.
#[derive(Default)]
pub struct JoinScanState {
    /// The join clause from planning.
    pub join_clause: JoinCSClause,

    // === Driving side state (side with search predicate - we iterate through this) ===
    /// The heap relation for the driving side.
    pub driving_heaprel: Option<PgSearchRelation>,
    /// Visibility checker for the driving side.
    pub driving_visibility_checker: Option<VisibilityChecker>,
    /// Slot for fetching driving side tuples.
    pub driving_fetch_slot: Option<*mut pg_sys::TupleTableSlot>,

    // === Build side state (side we build hash table from) ===
    /// The heap relation for the build side.
    pub build_heaprel: Option<PgSearchRelation>,
    /// Visibility checker for the build side.
    pub build_visibility_checker: Option<VisibilityChecker>,
    /// Slot for fetching build side tuples by ctid.
    pub build_scan_slot: Option<*mut pg_sys::TupleTableSlot>,

    // === Side tracking ===
    /// Whether the driving side is the outer side (true) or inner side (false).
    pub driving_is_outer: bool,

    // === Result state ===
    /// Result tuple slot.
    pub result_slot: Option<*mut pg_sys::TupleTableSlot>,

    // === DataFusion State ===
    pub datafusion_stream: Option<datafusion_execution::SendableRecordBatchStream>,
    pub runtime: Option<tokio::runtime::Runtime>,
    pub current_batch: Option<arrow_array::RecordBatch>,
    pub batch_index: usize,

    // === Probe state ===
    /// Current driving side ctid being probed.
    pub current_driving_ctid: Option<u64>,

    // === Output column mapping ===
    /// Mapping of output column positions to their source (outer/inner) and original attribute numbers.
    /// Populated from PrivateData during create_custom_scan_state.
    pub output_columns: Vec<OutputColumnInfo>,

    /// Index of the outer score column in the DataFusion batch, if any.
    pub outer_score_col_idx: Option<usize>,
    /// Index of the inner score column in the DataFusion batch, if any.
    pub inner_score_col_idx: Option<usize>,

    // === Memory tracking ===
    /// Maximum allowed memory for hash table (from work_mem, in bytes).
    pub max_hash_memory: usize,
}

impl JoinScanState {
    /// Reset the scan state for a rescan.
    pub fn reset(&mut self) {
        self.datafusion_stream = None;
        self.current_batch = None;
        self.batch_index = 0;
    }

    /// Returns (outer_slot, inner_slot) based on which side is driving.
    ///
    /// This maps the driving/build slots to outer/inner positions:
    /// - If driving_is_outer: driving_slot=outer, build_slot=inner
    /// - If driving_is_inner: driving_slot=inner, build_slot=outer
    pub fn outer_inner_slots(
        &self,
    ) -> (
        Option<*mut pg_sys::TupleTableSlot>,
        Option<*mut pg_sys::TupleTableSlot>,
    ) {
        if self.driving_is_outer {
            (self.driving_fetch_slot, self.build_scan_slot)
        } else {
            (self.build_scan_slot, self.driving_fetch_slot)
        }
    }

    /// Get the appropriate score for an output column.
    ///
    /// This determines whether to use the driving side score or the build side score
    /// based on which side the column references:
    /// - If `col_is_outer == driving_is_outer`: column references driving side → use driving_score
    /// - Otherwise: column references build side → use build_score
    pub fn score_for_column(&self, col_is_outer: bool, row_idx: usize) -> f32 {
        let score_idx = if col_is_outer {
            self.outer_score_col_idx
        } else {
            self.inner_score_col_idx
        };

        if let Some(idx) = score_idx {
            if let Some(batch) = &self.current_batch {
                let score_col = batch.column(idx);
                let score_array = score_col
                    .as_any()
                    .downcast_ref::<arrow_array::Float32Array>()
                    .expect("Score column should be Float32Array");
                return score_array.value(row_idx);
            }
        }

        0.0
    }
}

impl CustomScanState for JoinScanState {
    fn init_exec_method(&mut self, _cstate: *mut pg_sys::CustomScanState) {
        // No special initialization needed for the plain exec method
    }
}

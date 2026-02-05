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
use crate::index::fast_fields_helper::{FFHelper, FastFieldType, WhichFastField};
use crate::index::reader::index::SearchIndexScore;
use crate::nodecast;
use crate::postgres::customscan::basescan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::basescan::parallel::checkout_segment;
use crate::postgres::customscan::basescan::scan_state::BaseScanState;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::types_arrow::arrow_array_to_datum;
use crate::scan::{Batch, Scanner};

use pgrx::{pg_sys, IntoDatum, PgOid, PgTupleDesc};

struct Inner {
    heaprel: Option<PgSearchRelation>,
    tupdesc: Option<PgTupleDesc<'static>>,

    /// Execution time WhichFastFields.
    pub which_fast_fields: Vec<WhichFastField>,
    pub ffhelper: FFHelper,

    pub slot: *mut pg_sys::TupleTableSlot,

    did_query: bool,
}

impl Inner {
    pub fn new(which_fast_fields: Vec<WhichFastField>) -> Self {
        Self {
            heaprel: None,
            tupdesc: None,
            which_fast_fields,
            ffhelper: Default::default(),
            slot: std::ptr::null_mut(),
            did_query: false,
        }
    }

    pub fn init(&mut self, state: &mut BaseScanState, cstate: *mut pg_sys::CustomScanState) {
        unsafe {
            self.heaprel = Some(Clone::clone(state.heaprel()));
            self.tupdesc = Some(PgTupleDesc::from_pg_unchecked(
                (*cstate).ss.ps.ps_ResultTupleDesc,
            ));
            self.slot = pg_sys::MakeTupleTableSlot(
                (*cstate).ss.ps.ps_ResultTupleDesc,
                &pg_sys::TTSOpsVirtual,
            );
            // Initialize the fast field helper
            self.ffhelper = FFHelper::with_fields(
                state.search_reader.as_ref().unwrap(),
                &self.which_fast_fields,
            );
        }
    }

    pub fn reset(&mut self, _state: &mut BaseScanState) {
        self.did_query = false;
    }
}

/// Execution state for mixed fast field retrieval optimized for both string and numeric fields.
///
/// This execution state is designed to handle two scenarios:
/// 1. Multiple string fast fields in a single query
/// 2. A mix of string and numeric fast fields in a single query
///
/// This struct uses composition to build on the shared `Inner` state while adding
/// optimized processing paths for mixed field types.
///
/// # Usage Context
/// This execution method is selected when a query uses multiple fast fields with at least one
/// string fast field. It processes both string and numeric fields directly from the index's
/// fast field data structures, avoiding the need to fetch full documents.
///
/// # Feature Flag
/// This execution method is controlled by the `paradedb.enable_mixed_fast_field_exec` GUC setting.
/// It is disabled by default and can be enabled with:
/// ```sql
/// SET paradedb.enable_mixed_fast_field_exec = true;
/// ```
pub struct MixedFastFieldExecState {
    /// Core functionality shared with other fast field execution methods
    inner: Inner,

    /// The batch size hint to use for this execution.
    batch_size_hint: Option<usize>,

    /// The scanner that iterates over the results.
    scanner: Option<Scanner>,

    /// The current batch of fast field values
    current_batch: Option<Batch>,
    current_batch_offset: usize,

    /// Statistics tracking the number of visible rows
    num_visible: usize,

    /// Const values extracted from the target list to be projected into the slot
    const_values: HashMap<usize, (pg_sys::Datum, bool)>,
}

/// Populates the target slot with values for each attribute in the tuple descriptor.
///
/// Values are primarily retrieved from the pre-materialized Arrow columns in the `Batch`.
/// Special fields (like `ctid`, `tableoid`, and `score`) or constant expressions are
/// handled as fallbacks when a column is not present in the batch.
#[allow(clippy::too_many_arguments)]
fn populate_slot(
    const_values: &HashMap<usize, (pg_sys::Datum, bool)>,
    batch: &Batch,
    row_idx: usize,
    scored: SearchIndexScore,
    which_fast_fields: &[WhichFastField],
    tupdesc: &pgrx::PgTupleDesc,
    slot: &mut pg_sys::TupleTableSlot,
    datums: &mut [pg_sys::Datum],
    isnull: &mut [bool],
) {
    let fields = &batch.fields;
    for (i, (att, which_fast_field)) in tupdesc.iter().zip(which_fast_fields).enumerate() {
        match &fields[i] {
            Some(column) => {
                // Extract numeric scale if this is a Numeric64 field
                let numeric_scale = match which_fast_field {
                    WhichFastField::Named(_, FastFieldType::Numeric64(scale)) => Some(*scale),
                    _ => None,
                };
                // We extracted this field: convert it into a datum.
                match arrow_array_to_datum(
                    column.as_ref(),
                    row_idx,
                    PgOid::from(att.atttypid),
                    numeric_scale,
                ) {
                    Ok(Some(datum)) => {
                        datums[i] = datum;
                        isnull[i] = false;
                        continue;
                    }
                    Ok(None) => {
                        // Null datum.
                        continue;
                    }
                    Err(e) => {
                        panic!(
                            "Failed to convert to attribute type for \
                                {:?} and {which_fast_field:?}: {e}",
                            att.atttypid
                        );
                    }
                }
            }
            None => {
                // Fall back to manual extraction for special fields, or constant expressions.
                let datum_opt = match which_fast_field {
                    WhichFastField::Ctid => slot.tts_tid.into_datum(),
                    WhichFastField::TableOid => slot.tts_tableOid.into_datum(),
                    WhichFastField::Score => scored.bm25.into_datum(),
                    WhichFastField::Named(_, FastFieldType::String) => {
                        panic!("String fast field {which_fast_field:?} should already have been extracted.");
                    }
                    WhichFastField::Named(_, FastFieldType::Bytes) => {
                        panic!("Bytes fast field {which_fast_field:?} should already have been extracted.");
                    }
                    WhichFastField::Named(_, FastFieldType::Int64)
                    | WhichFastField::Named(_, FastFieldType::UInt64)
                    | WhichFastField::Named(_, FastFieldType::Float64)
                    | WhichFastField::Named(_, FastFieldType::Bool)
                    | WhichFastField::Named(_, FastFieldType::Date)
                    | WhichFastField::Named(_, FastFieldType::Numeric64(_)) => {
                        panic!("Numeric fast field {which_fast_field:?} should already have been extracted.");
                    }
                    WhichFastField::Junk(_) => None,
                };

                if let Some(datum) = datum_opt {
                    datums[i] = datum;
                    isnull[i] = false;
                } else {
                    // if the tlist entry is not null but the datum retrieved is null,
                    // it could mean there was a constant value in the tlist that we can
                    // project into the slot
                    if let Some((val, is_null)) = const_values.get(&i) {
                        datums[i] = *val;
                        isnull[i] = *is_null;
                        continue;
                    } else {
                        pgrx::error!(
                            "Expression in target list is not yet supported. \
                                Please file an issue at https://github.com/paradedb/paradedb/issues."
                        );
                    }
                }
            }
        }
    }
}

impl MixedFastFieldExecState {
    /// Creates a new MixedFastFieldExecState from a list of fast fields.
    ///
    /// This constructor analyzes the provided fast fields and categorizes them
    /// into string and numeric types for optimized processing.
    ///
    /// # Arguments
    ///
    /// * `which_fast_fields` - Vector of fast fields that will be processed
    ///
    /// # Returns
    ///
    /// A new MixedFastFieldExecState instance
    pub fn new(which_fast_fields: Vec<WhichFastField>, limit: Option<usize>) -> Self {
        // If there is a limit, then we use a batch size hint which is a small multiple of the
        // limit, in case of dead tuples.
        let batch_size_hint = limit.map(|limit| limit * 2);
        Self {
            inner: Inner::new(which_fast_fields),
            batch_size_hint,
            scanner: None,
            current_batch: None,
            current_batch_offset: 0,
            num_visible: 0,
            const_values: HashMap::default(),
        }
    }
}

impl ExecMethod for MixedFastFieldExecState {
    /// Initializes the execution state with the necessary context.
    ///
    /// # Arguments
    ///
    /// * `state` - The current scan state containing query information
    /// * `cstate` - PostgreSQL's custom scan state pointer
    fn init(&mut self, state: &mut BaseScanState, cstate: *mut pg_sys::CustomScanState) {
        // Initialize the inner FastFieldExecState
        self.inner.init(state, cstate);

        unsafe {
            let targetlist = (*(*cstate).ss.ps.plan).targetlist;
            let len = pg_sys::list_length(targetlist);
            self.const_values.clear();
            self.const_values.reserve(len as usize);

            for i in 0..len {
                let tle = pg_sys::list_nth(targetlist, i) as *mut pg_sys::TargetEntry;
                if !tle.is_null() && !(*tle).expr.is_null() {
                    if let Some(expr) = nodecast!(Const, T_Const, (*tle).expr) {
                        self.const_values
                            .insert(i as usize, ((*expr).constvalue, (*expr).constisnull));
                    }
                }
            }
        }

        // Reset mixed field specific state
        self.scanner = None;
        self.current_batch = None;
        self.current_batch_offset = 0;
        self.num_visible = 0;
    }

    /// Executes the search query and prepares result processing.
    ///
    /// This method handles both parallel and non-parallel execution paths.
    /// For parallel execution, it processes a single segment at a time.
    /// For non-parallel execution, it processes all segments at once.
    ///
    /// # Arguments
    ///
    /// * `state` - The current scan state containing query information
    ///
    /// # Returns
    ///
    /// `true` if there are results to process, `false` otherwise
    fn query(&mut self, state: &mut BaseScanState) -> bool {
        loop {
            // If we have a scanner, try to get the next batch.
            if let Some(scanner) = &mut self.scanner {
                if let Some(batch) =
                    scanner.next(&mut self.inner.ffhelper, state.visibility_checker())
                {
                    self.current_batch = Some(batch);
                    self.current_batch_offset = 0;
                    return true;
                }
                // No more batches from this scanner.
                self.scanner = None;
            }

            // We need a new scanner.
            let search_results = if let Some(parallel_state) = state.parallel_state {
                // Parallel: try to check out a segment.
                if let Some(segment_id) = unsafe { checkout_segment(parallel_state) } {
                    Some(
                        state
                            .search_reader
                            .as_ref()
                            .unwrap()
                            .search_segments([segment_id].into_iter()),
                    )
                } else {
                    None
                }
            } else if self.inner.did_query {
                // Not parallel and already queried.
                None
            } else {
                // First time query in non-parallel mode.
                self.inner.did_query = true;
                Some(state.search_reader.as_ref().unwrap().search())
            };

            if let Some(results) = search_results {
                let heaprel = self
                    .inner
                    .heaprel
                    .as_ref()
                    .expect("MixedFastFieldsExecState: heaprel should be initialized");
                self.scanner = Some(Scanner::new(
                    results,
                    self.batch_size_hint,
                    self.inner.which_fast_fields.clone(),
                    heaprel.oid().into(),
                ));
            } else {
                return false;
            }
        }
    }

    /// Fetches the next result and prepares it for returning to PostgreSQL.
    ///
    /// This method converts optimized search results into PostgreSQL tuple format,
    /// handling value retrieval for both string and numeric fields.
    ///
    /// # Arguments
    ///
    /// * `state` - The current scan state
    ///
    /// # Returns
    ///
    /// The next execution state containing the result or EOF
    fn internal_next(&mut self, _state: &mut BaseScanState) -> ExecState {
        unsafe {
            let batch = match self.current_batch.as_ref() {
                Some(batch) => batch,
                None => return ExecState::Eof,
            };

            if let Some((scored, _)) = batch.ids.get(self.current_batch_offset) {
                let row_idx = self.current_batch_offset;
                self.current_batch_offset += 1;

                let heaprel = self
                    .inner
                    .heaprel
                    .as_ref()
                    .expect("MixedFastFieldsExecState: heaprel should be initialized");
                let slot = self.inner.slot;
                let natts = (*(*slot).tts_tupleDescriptor).natts as usize;

                // Set ctid and table OID
                crate::postgres::utils::u64_to_item_pointer(scored.ctid, &mut (*slot).tts_tid);
                (*slot).tts_tableOid = heaprel.oid();

                // Setup slot for returning data
                (*slot).tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;
                (*slot).tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
                (*slot).tts_nvalid = natts as _;

                let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
                let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

                // Initialize all values to NULL
                for i in 0..natts {
                    datums[i] = pg_sys::Datum::null();
                    isnull[i] = true;
                }

                let which_fast_fields = &self.inner.which_fast_fields;
                let tupdesc = self.inner.tupdesc.as_ref().unwrap();
                debug_assert!(natts == which_fast_fields.len());

                populate_slot(
                    &self.const_values,
                    batch,
                    row_idx,
                    *scored,
                    which_fast_fields,
                    tupdesc,
                    &mut *slot,
                    datums,
                    isnull,
                );

                ExecState::Virtual { slot }
            } else {
                // This batch is exhausted.
                self.current_batch = None;
                ExecState::Eof
            }
        }
    }

    /// Resets the execution state to its initial state.
    ///
    /// # Arguments
    ///
    /// * `state` - The current scan state
    fn reset(&mut self, state: &mut BaseScanState) {
        // Reset inner FastFieldExecState
        self.inner.reset(state);

        // Reset mixed results state
        self.scanner = None;
        self.current_batch = None;
        self.current_batch_offset = 0;

        // Reset statistics
        self.num_visible = 0;
    }

    /// Increments the count of visible rows.
    ///
    /// Called when a row passes visibility checks.
    fn increment_visible(&mut self) {
        self.num_visible += 1;
    }
}

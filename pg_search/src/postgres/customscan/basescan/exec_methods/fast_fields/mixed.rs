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

use std::sync::Arc;

use arrow_array::{Array, RecordBatch};
use arrow_schema::SchemaRef;
use datafusion_execution::{SendableRecordBatchStream, TaskContext};
use datafusion_physical_plan::ExecutionPlan;
use futures::{Stream, StreamExt};

use crate::api::HashMap;
use crate::index::fast_fields_helper::{FFHelper, WhichFastField};
use crate::nodecast;
use crate::postgres::customscan::basescan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::basescan::parallel::checkout_segment;
use crate::postgres::customscan::basescan::scan_state::BaseScanState;
use crate::postgres::options::SortByField;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::types_arrow::arrow_array_to_datum;
use crate::scan::datafusion_plan::{create_sorted_scan, make_checkout_factory, ScanPlan};
use crate::scan::Scanner;

use pgrx::{pg_sys, IntoDatum, PgOid, PgTupleDesc};

// ============================================================================
// Synchronous stream polling utilities
// ============================================================================

/// Polls a stream for the next item synchronously using a blocking executor.
/// This properly handles `Poll::Pending` by driving the stream to completion,
/// which is necessary for DataFusion operators like `SortPreservingMergeExec`
/// that may buffer data across partitions.
fn poll_next_sync<S: Stream + Unpin>(stream: &mut S) -> Option<S::Item> {
    // Use futures::executor::block_on to properly drive the stream.
    // This handles Poll::Pending correctly by spinning until the stream is ready.
    futures::executor::block_on(stream.next())
}

struct Inner {
    heaprel: Option<PgSearchRelation>,
    tupdesc: Option<PgTupleDesc<'static>>,

    /// Execution time WhichFastFields.
    pub which_fast_fields: Vec<WhichFastField>,

    /// Fast field helper wrapped in Arc for sharing with DataFusion plans.
    pub ffhelper: Option<Arc<FFHelper>>,

    pub slot: *mut pg_sys::TupleTableSlot,

    did_query: bool,
}

impl Inner {
    pub fn new(which_fast_fields: Vec<WhichFastField>) -> Self {
        Self {
            heaprel: None,
            tupdesc: None,
            which_fast_fields,
            ffhelper: None,
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
            // Initialize the fast field helper wrapped in Arc for sharing
            self.ffhelper = Some(Arc::new(FFHelper::with_fields(
                state.search_reader.as_ref().unwrap(),
                &self.which_fast_fields,
            )));
        }
    }

    pub fn reset(&mut self, _state: &mut BaseScanState) {
        self.did_query = false;
    }
}

/// Execution state for mixed fast field retrieval using DataFusion execution.
///
/// This execution state is designed to handle two scenarios:
/// 1. Multiple string fast fields in a single query
/// 2. A mix of string and numeric fast fields in a single query
///
/// The execution method produces data through DataFusion's execution engine,
/// consuming results as Arrow RecordBatches from a DataFusion stream.
///
/// # Sorted Mode
///
/// When `sorted` is true and the index has a `sort_by` configuration, this execution
/// method uses `SortPreservingMergeExec` to merge sorted segment outputs into a
/// globally sorted result. This requires upfront segment checkout (not lazy).
///
/// When `sorted` is false, segments are processed lazily via PostgreSQL's parallel
/// query infrastructure with DataFusion producing batches for each segment.
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

    /// Whether to produce sorted output by merging sorted segments.
    sorted: bool,

    /// The sort order from the index (if any).
    sort_order: Option<SortByField>,

    /// Arrow schema for the RecordBatch
    schema: Option<SchemaRef>,

    /// The DataFusion stream producing RecordBatches.
    stream: Option<SendableRecordBatchStream>,

    /// The current RecordBatch of fast field values (DataFusion format)
    current_record_batch: Option<RecordBatch>,
    current_batch_row_idx: usize,

    /// Column index for ctid in the RecordBatch
    ctid_column_idx: Option<usize>,

    /// Column index for score in the RecordBatch (reserved for future sorted merge support)
    #[allow(dead_code)]
    score_column_idx: Option<usize>,

    /// Statistics tracking the number of visible rows
    num_visible: usize,

    /// Const values extracted from the target list to be projected into the slot
    const_values: HashMap<usize, (pg_sys::Datum, bool)>,
}

/// Populates the target slot with values from a RecordBatch.
///
/// Extracts values from Arrow columns and converts them to PostgreSQL datums.
/// Special handling for ctid and tableoid which are set on the slot directly.
#[allow(clippy::too_many_arguments)]
fn populate_slot_from_record_batch(
    const_values: &HashMap<usize, (pg_sys::Datum, bool)>,
    record_batch: &RecordBatch,
    row_idx: usize,
    which_fast_fields: &[WhichFastField],
    tupdesc: &pgrx::PgTupleDesc,
    slot: &mut pg_sys::TupleTableSlot,
    datums: &mut [pg_sys::Datum],
    isnull: &mut [bool],
) {
    for (i, (att, which_fast_field)) in tupdesc.iter().zip(which_fast_fields).enumerate() {
        let column = record_batch.column(i);

        // Check if this column has a null at this row
        if column.is_null(row_idx) {
            // Check for constant values
            if let Some((val, is_null)) = const_values.get(&i) {
                datums[i] = *val;
                isnull[i] = *is_null;
            }
            // Otherwise leave as null (already initialized)
            continue;
        }

        // Handle special fields that don't need datum conversion
        match which_fast_field {
            WhichFastField::Ctid => {
                // ctid is already set on slot.tts_tid before calling this function
                datums[i] = slot.tts_tid.into_datum().unwrap_or(pg_sys::Datum::null());
                isnull[i] = false;
                continue;
            }
            WhichFastField::TableOid => {
                // tableoid is already set on slot.tts_tableOid before calling this function
                datums[i] = slot
                    .tts_tableOid
                    .into_datum()
                    .unwrap_or(pg_sys::Datum::null());
                isnull[i] = false;
                continue;
            }
            WhichFastField::Junk(_) => {
                // Junk columns produce null
                continue;
            }
            _ => {}
        }

        // Convert Arrow array value to datum
        match arrow_array_to_datum(column.as_ref(), row_idx, PgOid::from(att.atttypid)) {
            Ok(Some(datum)) => {
                datums[i] = datum;
                isnull[i] = false;
            }
            Ok(None) => {
                // Null datum - check for const value
                if let Some((val, is_null)) = const_values.get(&i) {
                    datums[i] = *val;
                    isnull[i] = *is_null;
                }
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
    /// * `limit` - Optional limit for batch size optimization
    /// * `sorted` - Whether to produce globally sorted output via SortPreservingMergeExec
    /// * `sort_order` - The sort order from the index (required if sorted is true)
    ///
    /// # Returns
    ///
    /// A new MixedFastFieldExecState instance
    pub fn new(
        which_fast_fields: Vec<WhichFastField>,
        limit: Option<usize>,
        sorted: bool,
        sort_order: Option<SortByField>,
    ) -> Self {
        // Find ctid and score column indices
        let ctid_column_idx = which_fast_fields
            .iter()
            .position(|f| matches!(f, WhichFastField::Ctid));
        let score_column_idx = which_fast_fields
            .iter()
            .position(|f| matches!(f, WhichFastField::Score));

        // If there is a limit, then we use a batch size hint which is a small multiple of the
        // limit, in case of dead tuples.
        let batch_size_hint = limit.map(|limit| limit * 2);
        Self {
            inner: Inner::new(which_fast_fields),
            batch_size_hint,
            sorted,
            sort_order,
            schema: None,
            stream: None,
            current_record_batch: None,
            current_batch_row_idx: 0,
            ctid_column_idx,
            score_column_idx,
            num_visible: 0,
            const_values: HashMap::default(),
        }
    }

    /// Creates a DataFusion stream for the unsorted path.
    ///
    /// Uses PostgreSQL's lazy segment checkout - one segment at a time.
    /// Each segment is processed through DataFusion's ScanPlan.
    fn create_unsorted_stream(
        &mut self,
        state: &mut BaseScanState,
    ) -> Option<SendableRecordBatchStream> {
        // Get search results (lazily checks out one segment in parallel mode)
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

        let results = search_results?;

        let heaprel = self
            .inner
            .heaprel
            .as_ref()
            .expect("MixedFastFieldsExecState: heaprel should be initialized");
        let ffhelper = self
            .inner
            .ffhelper
            .as_ref()
            .expect("MixedFastFieldsExecState: ffhelper should be initialized");

        // Create scanner
        let scanner = Scanner::new(
            results,
            self.batch_size_hint,
            self.inner.which_fast_fields.clone(),
            heaprel.oid().into(),
        );

        // Capture schema
        self.schema = Some(scanner.schema());

        // Clone visibility checker for the plan
        let visibility = state
            .visibility_checker
            .as_ref()
            .expect("MixedFastFieldsExecState: visibility_checker should be initialized")
            .clone();

        // Create ScanPlan and execute via DataFusion
        let plan =
            ScanPlan::new_with_shared_ffhelper(scanner, Arc::clone(ffhelper), Box::new(visibility));

        let task_ctx = Arc::new(TaskContext::default());
        match plan.execute(0, task_ctx) {
            Ok(stream) => Some(stream),
            Err(e) => {
                pgrx::warning!("Failed to execute ScanPlan: {e}");
                None
            }
        }
    }

    /// Creates a DataFusion stream for the sorted path.
    ///
    /// Uses lazy segment checkout - segments are checked out on-demand via a factory
    /// function when SortPreservingMergeExec calls execute() on each partition.
    /// This defers memory allocation until execution time rather than plan creation.
    fn create_sorted_stream(
        &mut self,
        state: &mut BaseScanState,
    ) -> Option<SendableRecordBatchStream> {
        if self.inner.did_query {
            return None;
        }
        self.inner.did_query = true;

        pgrx::log!("SORTED STREAM: create_sorted_stream() called - using lazy segment checkout");

        let sort_order = self.sort_order.as_ref()?;

        let heaprel = self
            .inner
            .heaprel
            .as_ref()
            .expect("MixedFastFieldsExecState: heaprel should be initialized");
        let ffhelper = self
            .inner
            .ffhelper
            .as_ref()
            .expect("MixedFastFieldsExecState: ffhelper should be initialized");
        let search_reader = state.search_reader.as_ref().unwrap();

        // Get segment count and IDs (cheap - just metadata)
        let segment_readers = search_reader.segment_readers();
        let segment_count = segment_readers.len();

        if segment_count == 0 {
            return None;
        }

        // Collect segment IDs for the factory closure
        let segment_ids: Vec<_> = segment_readers.iter().map(|r| r.segment_id()).collect();

        // Build schema from which_fast_fields (same logic as Scanner::schema())
        let schema = build_schema_from_fast_fields(&self.inner.which_fast_fields);
        self.schema = Some(schema.clone());

        // Capture variables for the factory closure
        let search_reader = state.search_reader.clone().unwrap();
        let batch_size = self.batch_size_hint;
        let which_fast_fields = self.inner.which_fast_fields.clone();
        let table_oid: u32 = heaprel.oid().into();
        let ffhelper = Arc::clone(ffhelper);
        let visibility_checker = state
            .visibility_checker
            .clone()
            .expect("MixedFastFieldsExecState: visibility_checker should be initialized");

        // Create factory that checks out segment on demand (lazy checkout)
        // Uses make_checkout_factory to safely wrap the closure (which captures non-Send/Sync types)
        let checkout_factory = make_checkout_factory(move |partition: usize| {
            pgrx::log!(
                "LAZY CHECKOUT: partition {} of {} (segment_id: {:?})",
                partition,
                segment_ids.len(),
                segment_ids[partition]
            );
            let segment_id = segment_ids[partition];
            let results = search_reader.search_segments([segment_id].into_iter());
            let scanner = Scanner::new(results, batch_size, which_fast_fields.clone(), table_oid);
            let visibility: Box<dyn crate::scan::VisibilityChecker> =
                Box::new(visibility_checker.clone());
            (scanner, Arc::clone(&ffhelper), visibility)
        });

        // Create sorted scan plan with SortPreservingMergeExec (lazy)
        // Returns None if the sort field is not in the schema
        let plan = match create_sorted_scan(segment_count, checkout_factory, schema, sort_order) {
            Some(plan) => plan,
            None => {
                // Sort field not in schema - cannot create sorted merge.
                // Actually fall back to unsorted execution instead of returning None
                // (returning None would cause zero rows to be returned).
                pgrx::warning!(
                    "Sort field '{}' not found in scan schema - falling back to unsorted execution",
                    sort_order.field_name.as_ref()
                );
                return self.create_unsorted_stream(state);
            }
        };

        // Execute the plan (partition 0 for merged output)
        let task_ctx = Arc::new(TaskContext::default());
        match plan.execute(0, task_ctx) {
            Ok(stream) => Some(stream),
            Err(e) => {
                pgrx::warning!("Failed to execute sorted ScanPlan: {e}");
                None
            }
        }
    }
}

/// Build an Arrow schema from the fast fields specification.
/// This mirrors the logic in Scanner::schema() but doesn't require a Scanner instance.
fn build_schema_from_fast_fields(which_fast_fields: &[WhichFastField]) -> SchemaRef {
    use crate::index::fast_fields_helper::FastFieldType;
    use arrow_schema::{DataType, Field, Schema};

    let fields: Vec<Field> = which_fast_fields
        .iter()
        .map(|wff| {
            let data_type = match wff {
                WhichFastField::Ctid => DataType::UInt64,
                WhichFastField::TableOid => DataType::UInt32,
                WhichFastField::Score => DataType::Float32,
                WhichFastField::Named(_, ff_type) => match ff_type {
                    FastFieldType::String => DataType::Utf8View,
                    FastFieldType::Int64 => DataType::Int64,
                    FastFieldType::UInt64 => DataType::UInt64,
                    FastFieldType::Float64 => DataType::Float64,
                    FastFieldType::Bool => DataType::Boolean,
                    FastFieldType::Date => {
                        DataType::Timestamp(arrow_schema::TimeUnit::Nanosecond, None)
                    }
                },
                WhichFastField::Junk(_) => DataType::Null,
            };
            Field::new(wff.name(), data_type, true)
        })
        .collect();
    Arc::new(Schema::new(fields))
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
        self.stream = None;
        self.schema = None;
        self.current_record_batch = None;
        self.current_batch_row_idx = 0;
        self.num_visible = 0;
    }

    /// Executes the search query and prepares result processing.
    ///
    /// This method handles both parallel and non-parallel execution paths using
    /// DataFusion's execution engine to produce RecordBatch streams.
    ///
    /// For sorted mode (`sorted = true`), all segments are checked out upfront
    /// and merged via `SortPreservingMergeExec` for globally sorted output.
    ///
    /// For unsorted mode, segments are processed lazily via PostgreSQL's parallel
    /// query infrastructure, with each segment producing its own DataFusion stream.
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
            // Try to get next batch from existing stream
            if let Some(stream) = &mut self.stream {
                match poll_next_sync(stream) {
                    Some(Ok(batch)) => {
                        self.current_record_batch = Some(batch);
                        self.current_batch_row_idx = 0;
                        return true;
                    }
                    Some(Err(e)) => {
                        pgrx::warning!("Error polling DataFusion stream: {e}");
                        self.stream = None;
                        return false;
                    }
                    None => {
                        // Stream exhausted
                        self.stream = None;
                        // For unsorted mode, try to get another stream (next segment)
                        if !self.sorted {
                            continue;
                        }
                        // For sorted mode, we're done (all segments processed in one stream)
                        return false;
                    }
                }
            }

            // Create a new DataFusion stream
            let new_stream = if self.sorted && self.sort_order.is_some() {
                self.create_sorted_stream(state)
            } else {
                self.create_unsorted_stream(state)
            };

            match new_stream {
                Some(stream) => {
                    self.stream = Some(stream);
                    // Continue loop to poll the new stream
                }
                None => {
                    return false;
                }
            }
        }
    }

    /// Fetches the next result and prepares it for returning to PostgreSQL.
    ///
    /// This method converts DataFusion RecordBatch results into PostgreSQL tuple format,
    /// handling value retrieval for all field types from Arrow columns.
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
            let record_batch = match self.current_record_batch.as_ref() {
                Some(batch) => batch,
                None => return ExecState::Eof,
            };

            let row_idx = self.current_batch_row_idx;
            if row_idx >= record_batch.num_rows() {
                // This batch is exhausted.
                self.current_record_batch = None;
                return ExecState::Eof;
            }

            self.current_batch_row_idx += 1;

            let heaprel = self
                .inner
                .heaprel
                .as_ref()
                .expect("MixedFastFieldsExecState: heaprel should be initialized");
            let slot = self.inner.slot;
            let natts = (*(*slot).tts_tupleDescriptor).natts as usize;

            // Extract ctid from the RecordBatch
            let ctid = if let Some(ctid_idx) = self.ctid_column_idx {
                let ctid_array = record_batch
                    .column(ctid_idx)
                    .as_any()
                    .downcast_ref::<arrow_array::UInt64Array>()
                    .expect("ctid column should be UInt64Array");
                ctid_array.value(row_idx)
            } else {
                0u64
            };

            // Set ctid and table OID on the slot
            crate::postgres::utils::u64_to_item_pointer(ctid, &mut (*slot).tts_tid);
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

            populate_slot_from_record_batch(
                &self.const_values,
                record_batch,
                row_idx,
                which_fast_fields,
                tupdesc,
                &mut *slot,
                datums,
                isnull,
            );

            ExecState::Virtual { slot }
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

        // Reset DataFusion stream state
        self.stream = None;
        self.schema = None;
        self.current_record_batch = None;
        self.current_batch_row_idx = 0;

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

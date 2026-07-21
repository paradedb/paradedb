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

use std::alloc::Layout;
use std::collections::BTreeMap;
use std::io::Write;
use std::ops::Range;

use crate::api::{HashMap, HashSet};
use crate::gucs;
use crate::postgres::build::is_bm25_index;
use crate::postgres::condition_variable::ConditionVariable;
use crate::postgres::locks::Spinlock;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::shared_threshold::ParallelScanThresholdState;
use crate::query::SearchQueryInput;

use pgrx::*;
use tantivy::aggregation::intermediate_agg_result::IntermediateAggregationResults;
use tantivy::index::SegmentId;
use tantivy::SegmentReader;

mod build;
mod cost;
mod delete;
pub mod deparse;
pub mod insert;
mod merge;
pub mod options;
mod ps_status;
mod range;
mod scan;
pub mod shared_threshold;
mod vacuum;
mod validate;

mod build_parallel;
pub mod catalog;
pub mod composite;
mod condition_variable;
pub mod customscan;
pub mod datetime;
#[cfg(not(any(feature = "pg17", feature = "pg18")))]
pub mod fake_aminsertcleanup;
pub mod heap;
pub mod index;
mod jsonb_support;
pub mod locks;
mod parallel;
pub mod pdb_owned_value;
pub mod planner_warnings;
pub mod rel;
pub mod storage;
pub mod types;
pub mod types_arrow;
pub mod utils;
pub mod var;

#[repr(u16)] // b/c that's what [`pg_sys::StrategyNumber`] is
pub enum ScanStrategy {
    TextQuery = 1,
    SearchQueryInput = 2,
    // NB:  Any additions here **mut** update the `amroutine.amstrategies` down below in [`bm25_handler`]
}

impl TryFrom<pg_sys::StrategyNumber> for ScanStrategy {
    type Error = String;

    fn try_from(value: pg_sys::StrategyNumber) -> Result<Self, Self::Error> {
        if value == 1 {
            Ok(ScanStrategy::TextQuery)
        } else if value == 2 {
            Ok(ScanStrategy::SearchQueryInput)
        } else {
            Err(format!("`{value}` is an unknown `ScanStrategy` number"))
        }
    }
}

#[pg_extern(sql = "
CREATE FUNCTION bm25_handler(internal) RETURNS index_am_handler PARALLEL SAFE IMMUTABLE STRICT COST 0.0001 LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
CREATE ACCESS METHOD bm25 TYPE INDEX HANDLER bm25_handler;
COMMENT ON ACCESS METHOD bm25 IS 'bm25 index access method';
")]
fn bm25_handler(_fcinfo: pg_sys::FunctionCallInfo) -> PgBox<pg_sys::IndexAmRoutine> {
    let mut amroutine =
        unsafe { PgBox::<pg_sys::IndexAmRoutine>::alloc_node(pg_sys::NodeTag::T_IndexAmRoutine) };

    amroutine.amstrategies = 2;
    amroutine.amsupport = 0;
    amroutine.amcanmulticol = true;
    amroutine.amsearcharray = true;

    amroutine.amkeytype = pg_sys::InvalidOid;

    amroutine.amvalidate = Some(validate::amvalidate);
    amroutine.ambuild = Some(build::ambuild);
    amroutine.ambuildempty = Some(build::ambuildempty);
    amroutine.aminsert = Some(insert::aminsert);
    #[cfg(any(feature = "pg17", feature = "pg18"))]
    {
        amroutine.aminsertcleanup = Some(insert::aminsertcleanup);
        amroutine.amcanbuildparallel = true;
    }
    amroutine.ambulkdelete = Some(delete::ambulkdelete);
    amroutine.amvacuumcleanup = Some(vacuum::amvacuumcleanup);
    amroutine.amcostestimate = Some(cost::amcostestimate);
    amroutine.amoptions = Some(options::amoptions);
    amroutine.ambeginscan = Some(scan::ambeginscan);
    amroutine.amrescan = Some(scan::amrescan);
    amroutine.amgettuple = Some(scan::amgettuple);
    amroutine.amgetbitmap = Some(scan::amgetbitmap);
    amroutine.amendscan = Some(scan::amendscan);
    amroutine.amcanreturn = Some(scan::amcanreturn);

    amroutine.amcanparallel = true;
    amroutine.aminitparallelscan = Some(parallel::aminitparallelscan);
    amroutine.amestimateparallelscan = Some(parallel::amestimateparallelscan);
    amroutine.amparallelrescan = Some(parallel::amparallelrescan);

    amroutine.into_pg_boxed()
}

/// Finds and returns the `USING bm25` index on the specified relation with the highest OID,
/// along with the heap relation. Returns [`None`] if there isn't one.
///
/// Filters out indexes that aren't yet `indisvalid` (e.g. mid-`CREATE INDEX CONCURRENTLY`
/// or a failed `REINDEX`). When more than one valid bm25 index exists on the relation
/// (only possible via `CREATE INDEX CONCURRENTLY`, which bypasses the single-bm25-index
/// check), the highest-OID one is chosen so that the index added most recently wins.
pub fn rel_get_bm25_index(
    relid: pg_sys::Oid,
) -> Option<(rel::PgSearchRelation, rel::PgSearchRelation)> {
    if relid == pg_sys::Oid::INVALID {
        return None;
    }

    let rel = PgSearchRelation::with_lock(relid, pg_sys::AccessShareLock as _);
    let index = unsafe {
        rel.indices(pg_sys::AccessShareLock as _)
            .filter(|index| pg_sys::get_index_isvalid(index.oid()) && is_bm25_index(index))
            .max_by_key(|i| i.oid().to_u32())?
    };
    Some((rel, index))
}

// 16 bytes for segment UUID
const SEGMENT_ID_SIZE: usize = 16;

const SEGMENT_CLAIM_UNCLAIMED: i32 = -2;

#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
struct AggregatesPayloadHeader {
    watermark: usize,
    received_count: usize,
    serialized_aggregations_len: usize,
}

#[derive(Debug)]
#[repr(C)]
struct ParallelScanPayloadLayout {
    query: Range<usize>,
    /// One `u32` per source: its segment count, in ascending source-index order.
    all_counts: Range<usize>,
    /// Concatenated 16-byte segment UUIDs for all sources, in ascending source-index order.
    all_ids: Range<usize>,
    /// One `u32` per source: remaining unclaimed segments. Lets every source in a
    /// multi-source MPP plan be partitioned across workers, not just the planner-chosen
    /// partitioning source.
    remaining_by_source: Range<usize>,
    /// One `u32` per segment in the partitioning source: deleted doc count.
    primary_deleted_docs: Range<usize>,
    /// One `u32` per segment in the partitioning source: max doc count.
    primary_max_docs: Range<usize>,
    /// One `i32` per segment in the partitioning source: which worker claimed it.
    /// Sized to `primary_nsegments` (the first source), so subsequent sources
    /// have no slot here. Read by [`ParallelScanState::explain_data`] to populate the "Parallel
    /// Workers" section of `EXPLAIN ANALYZE`.
    claims: Range<usize>,
    aggregates_header: Option<Range<usize>>,
    aggregates_data: Option<Range<usize>>,
    /// The padded size of the layout.
    total: Layout,
}

impl ParallelScanPayloadLayout {
    fn new(
        all_nsegments: &[usize],
        serialized_query: &[u8],
        with_aggregates: bool,
    ) -> Result<Self, std::alloc::LayoutError> {
        let n_sources = all_nsegments.len();
        let total_segs: usize = all_nsegments.iter().sum();
        let primary_nsegments = all_nsegments
            .first()
            .copied()
            .expect("primary source empty");

        // Query.
        let layout = Layout::from_size_align(serialized_query.len(), 1)?;
        let query_range = 0..(layout.size());

        // Per-source segment counts: [u32; n_sources].
        let all_counts_layout = Layout::array::<u32>(n_sources)?;
        let (layout, all_counts_offset) = layout.extend(all_counts_layout)?;
        let all_counts_range = all_counts_offset..(all_counts_offset + all_counts_layout.size());

        // All segment IDs concatenated: [[u8; 16]; total_segs].
        let all_ids_layout = Layout::from_size_align(total_segs * SEGMENT_ID_SIZE, 1)?;
        let (layout, all_ids_offset) = layout.extend(all_ids_layout)?;
        let all_ids_range = all_ids_offset..(all_ids_offset + all_ids_layout.size());

        let remaining_layout = Layout::array::<u32>(n_sources)?;
        let (layout, remaining_offset) = layout.extend(remaining_layout)?;
        let remaining_range = remaining_offset..(remaining_offset + remaining_layout.size());

        // Deleted doc counts for the partitioning source only: [u32; primary_nsegments].
        let primary_del_layout = Layout::array::<u32>(primary_nsegments)?;
        let (layout, primary_del_offset) = layout.extend(primary_del_layout)?;
        let primary_deleted_docs_range =
            primary_del_offset..(primary_del_offset + primary_del_layout.size());

        // Max doc counts for the partitioning source only: [u32; primary_nsegments].
        let primary_max_layout = Layout::array::<u32>(primary_nsegments)?;
        let (layout, primary_max_offset) = layout.extend(primary_max_layout)?;
        let primary_max_docs_range =
            primary_max_offset..(primary_max_offset + primary_max_layout.size());

        // Claims for the partitioning source only: [i32; primary_nsegments].
        let claims_layout = Layout::array::<i32>(primary_nsegments)?;
        let (mut layout, claims_offset) = layout.extend(claims_layout)?;
        let claims_range = claims_offset..(claims_offset + claims_layout.size());

        let (aggregates_header, aggregates_data) = if with_aggregates {
            let (l, offset) = layout.extend(Layout::new::<AggregatesPayloadHeader>())?;
            layout = l;
            let header_range =
                Some(offset..offset + std::mem::size_of::<AggregatesPayloadHeader>());

            let data_size = gucs::max_window_aggregate_response_bytes();
            let (l, offset) = layout.extend(Layout::from_size_align(data_size, 1)?)?;
            layout = l;
            let data_range = Some(offset..offset + data_size);

            (header_range, data_range)
        } else {
            (None, None)
        };

        Ok(Self {
            query: query_range,
            all_counts: all_counts_range,
            all_ids: all_ids_range,
            remaining_by_source: remaining_range,
            primary_deleted_docs: primary_deleted_docs_range,
            primary_max_docs: primary_max_docs_range,
            claims: claims_range,
            aggregates_header,
            aggregates_data,
            // Finalize the layout by padding it to its overall alignment.
            total: layout.pad_to_align(),
        })
    }
}

/// The portion of the ParallelScanState which is dynamically sized.
#[derive(Debug)]
#[repr(C)]
struct ParallelScanPayload {
    layout: ParallelScanPayloadLayout,
    // Dynamically sized, allocated after.
    // NOTE: When adjusting the size of this field, you must additionally
    // adjust `ParallelScanPayloadLayout`.
    data: [u8; 0],
}

impl ParallelScanPayload {
    fn init(&mut self, all_sources: &[&[SegmentReader]], query: &[u8], with_aggregates: bool) {
        let all_nsegments: Vec<usize> = all_sources.iter().map(|s| s.len()).collect();
        // Compute and assign the execution-time layout from actual segment counts.
        self.layout = ParallelScanPayloadLayout::new(&all_nsegments, query, with_aggregates)
            .expect("could not layout `ParallelScanPayload` for initialization");

        // Query.
        let query_range = self.layout.query.clone();
        let _ = (&mut self.data_mut()[query_range])
            .write(query)
            .expect("failed to write query bytes");

        // Per-source segment counts.
        let counts_range = self.layout.all_counts.clone();
        let counts_slice: &mut [u32] =
            bytemuck::try_cast_slice_mut(&mut self.data_mut()[counts_range]).unwrap();
        for (source, target) in all_sources.iter().zip(counts_slice.iter_mut()) {
            *target = source.len() as u32;
        }

        // All segment IDs concatenated.
        let ids_range = self.layout.all_ids.clone();
        let ids_slice: &mut [[u8; SEGMENT_ID_SIZE]] =
            bytemuck::try_cast_slice_mut(&mut self.data_mut()[ids_range]).unwrap();
        let mut flat_offset = 0;
        for source in all_sources.iter() {
            for (reader, target) in source.iter().zip(ids_slice[flat_offset..].iter_mut()) {
                let mut writer = &mut target[..];
                writer
                    .write_all(reader.segment_id().uuid_bytes())
                    .expect("failed to write segment bytes");
            }
            flat_offset += source.len();
        }

        // Deleted doc counts for the partitioning source only.
        let del_range = self.layout.primary_deleted_docs.clone();
        let del_slice: &mut [u32] =
            bytemuck::try_cast_slice_mut(&mut self.data_mut()[del_range]).unwrap();
        let primary_source = all_sources.first().expect("primary source empty");
        for (reader, target) in primary_source.iter().zip(del_slice.iter_mut()) {
            *target = reader.num_deleted_docs();
        }

        // Max doc counts for the partitioning source only.
        let max_range = self.layout.primary_max_docs.clone();
        let max_slice: &mut [u32] =
            bytemuck::try_cast_slice_mut(&mut self.data_mut()[max_range]).unwrap();
        for (reader, target) in primary_source.iter().zip(max_slice.iter_mut()) {
            *target = reader.max_doc();
        }

        // Initialize claims (only for the partitioning source).
        for claim in self.claims_mut().iter_mut() {
            *claim = SEGMENT_CLAIM_UNCLAIMED;
        }

        let remaining_range = self.layout.remaining_by_source.clone();
        let remaining_slice: &mut [u32] =
            bytemuck::try_cast_slice_mut(&mut self.data_mut()[remaining_range]).unwrap();
        for (source, target) in all_sources.iter().zip(remaining_slice.iter_mut()) {
            *target = source.len() as u32;
        }
    }

    fn data(&self) -> &[u8] {
        unsafe {
            let data_ptr = std::ptr::addr_of!(self.data);
            std::slice::from_raw_parts(data_ptr.cast(), self.layout.total.size())
        }
    }

    fn data_mut(&mut self) -> &mut [u8] {
        unsafe {
            let data_ptr = std::ptr::addr_of_mut!(self.data);
            std::slice::from_raw_parts_mut(data_ptr.cast(), self.layout.total.size())
        }
    }

    fn query(&self) -> anyhow::Result<Option<SearchQueryInput>> {
        let query_range = self.layout.query.clone();
        if query_range.is_empty() {
            return Ok(None);
        }
        let query_data = &self.data()[query_range];
        Ok(Some(serde_json::from_slice(query_data)?))
    }

    fn all_counts(&self) -> &[u32] {
        bytemuck::try_cast_slice(&self.data()[self.layout.all_counts.clone()]).unwrap()
    }

    fn source_flat_offset(&self, source_idx: usize) -> usize {
        self.all_counts()[..source_idx]
            .iter()
            .map(|&c| c as usize)
            .sum()
    }

    fn source_ids(&self, source_idx: usize) -> &[[u8; SEGMENT_ID_SIZE]] {
        let offset = self.source_flat_offset(source_idx);
        let count = self.all_counts()[source_idx] as usize;
        let all: &[[u8; SEGMENT_ID_SIZE]] =
            bytemuck::try_cast_slice(&self.data()[self.layout.all_ids.clone()]).unwrap();
        &all[offset..offset + count]
    }

    fn primary_deleted_docs(&self) -> &[u32] {
        bytemuck::try_cast_slice(&self.data()[self.layout.primary_deleted_docs.clone()]).unwrap()
    }

    fn primary_max_docs(&self) -> &[u32] {
        bytemuck::try_cast_slice(&self.data()[self.layout.primary_max_docs.clone()]).unwrap()
    }

    fn remaining_by_source_mut(&mut self) -> &mut [u32] {
        let range = self.layout.remaining_by_source.clone();
        bytemuck::try_cast_slice_mut(&mut self.data_mut()[range]).unwrap()
    }

    /// An array of `i32` parallel worker numbers (as returned by pg_sys::ParallelWorkerNumber)
    /// which indicates which worker has claimed each segment at the same idx in the partitioning
    /// source's segment list. Any value less than `-1` (the leader) indicates unclaimed.
    fn claims(&self) -> &[i32] {
        bytemuck::try_cast_slice::<u8, i32>(&self.data()[self.layout.claims.clone()]).unwrap()
    }

    /// See `claims`.
    fn claims_mut(&mut self) -> &mut [i32] {
        let claims_range = self.layout.claims.clone();
        bytemuck::try_cast_slice_mut(&mut self.data_mut()[claims_range]).unwrap()
    }

    fn aggregates_header(&self) -> Option<&AggregatesPayloadHeader> {
        let range = self.layout.aggregates_header.clone()?;
        Some(bytemuck::from_bytes(&self.data()[range]))
    }

    fn aggregates_mut_parts(
        &mut self,
    ) -> (Option<&mut AggregatesPayloadHeader>, Option<&mut [u8]>) {
        if let (Some(header_range), Some(data_range)) = (
            self.layout.aggregates_header.clone(),
            self.layout.aggregates_data.clone(),
        ) {
            // This is safe because we know the layout of the parallel scan payload
            // ensures that the header and data ranges are disjoint.
            unsafe {
                let data_ptr = self.data_mut().as_mut_ptr();
                let header =
                    &mut *(data_ptr.add(header_range.start) as *mut AggregatesPayloadHeader);
                let data = std::slice::from_raw_parts_mut(
                    data_ptr.add(data_range.start),
                    data_range.len(),
                );
                (Some(header), Some(data))
            }
        } else {
            (None, None)
        }
    }

    fn aggregates_data(&self) -> Option<&[u8]> {
        let range = self.layout.aggregates_data.clone()?;
        Some(&self.data()[range])
    }
}

pub struct ParallelScanArgs<'a> {
    /// All sources in ascending source-index order. For basescan this is a single element.
    all_sources: Vec<&'a [SegmentReader]>,
    query: Vec<u8>,
    with_aggregates: bool,
}

impl<'a> ParallelScanArgs<'a> {
    fn all_nsegments(&self) -> Vec<usize> {
        self.all_sources.iter().map(|s| s.len()).collect()
    }
}

// We do not know ahead of time how many workers there will be, so we preallocate fixed size
// arrays for metrics for up to a given number of parallel workers.
const WORKER_METRICS_MAX_COUNT: usize = 256;

/// Sentinel value indicating that the parallel state has not been initialized yet.
/// Workers must wait until this changes before reading segment data.
const PARALLEL_STATE_UNINITIALIZED: usize = usize::MAX;

/// Shared state for coordinating parallel scans across multiple workers.
///
/// # Concurrency Model
///
/// The `basescan` and IAM in ParadeDB use a "lazy checkout" model where parallel workers claim
/// segments on-demand from a shared pool. This allows for dynamic work-sharing without needing to
/// pre-assign segments to specific workers.
///
/// For this model to work effectively, it is critical that workers perform actual work (scanning)
/// between checkouts. If a worker checks out segments in a tight loop without intermediate work,
/// it may claim all segments before other workers have time to start up, resulting in poor
/// parallelism.
///
/// This dynamic model is chosen because it is ~impossible to determine the exact number of parallel
/// workers available to a Custom Scan at runtime. The `ParallelContext` is shared across all
/// nodes in a plan subgraph, and the actual number of workers launched may be a fraction of the
/// planned count. Assuming a fixed number of workers (e.g., for static partitioning) will lead to
/// deadlocks if fewer workers are available than expected. On the other hand, our `aggregatescan`
/// currently spawns its own workers, and so uses a different strategy.
///
/// When a parallel custom scan claims sorted output, PostgreSQL automatically handles merging
/// the output from each worker using a sort-preserving merge (via `Gather Merge`). This allows
/// us to maintain the lazy checkout model even for sorted scans, as each worker only needs
/// to provide a sorted stream for the segments it dynamically claims.
#[repr(C)]
pub struct ParallelScanState {
    mutex: Spinlock,
    /// Condition variable for efficient waiting in `aggregation_wait()`.
    /// Workers sleep on this CV instead of busy-waiting, and are woken
    /// when the last worker calls `aggregation_append()`.
    aggregation_cv: ConditionVariable,
    /// Condition variable for efficient waiting in `wait_for_initialization()`.
    /// Workers sleep on this CV instead of busy-waiting, and are woken
    /// when the leader calls `populate()`.
    init_cv: ConditionVariable,
    /// Number of segments in the partitioning source. Set to PARALLEL_STATE_UNINITIALIZED
    /// until the leader initializes. Protected by mutex. Remaining work lives in
    /// `payload.remaining_by_source[0]`.
    nsegments: usize,
    queries_per_worker: [u16; WORKER_METRICS_MAX_COUNT],
    /// Top-K Shared Threshold Fields
    pub shared_threshold: ParallelScanThresholdState,

    payload: ParallelScanPayload, // must be last field, b/c it allocates on the heap after this struct
}

impl ParallelScanState {
    pub fn payload_capacity_of(
        all_nsegments: &[usize],
        serialized_query: &[u8],
        with_aggregates: bool,
    ) -> usize {
        ParallelScanPayloadLayout::new(all_nsegments, serialized_query, with_aggregates)
            .expect("could not compute DSM payload capacity")
            .total
            .size()
    }

    pub fn size_of(
        all_nsegments: &[usize],
        serialized_query: &[u8],
        with_aggregates: bool,
    ) -> usize {
        std::mem::size_of::<Self>()
            + Self::payload_capacity_of(all_nsegments, serialized_query, with_aggregates)
    }

    pub fn source_count(&self) -> usize {
        let range = self.payload.layout.all_counts.clone();
        range.len() / std::mem::size_of::<u32>()
    }

    /// Phase 1+2: Create the mutex and populate with actual data in one call.
    /// Used by Custom Scan which has all data available at initialization time.
    fn create_and_populate(&mut self, args: ParallelScanArgs) {
        self.mutex.init();
        self.aggregation_cv.init();
        self.init_cv.init();
        self.shared_threshold.init();
        self.populate(&args.all_sources, &args.query, args.with_aggregates);
    }

    /// Phase 2: Populate with actual data (assumes mutex already created via `create`).
    /// Used by Index Scan where the leader initializes the segment pool.
    ///
    /// Caller must hold the mutex. After populating, broadcasts to wake any workers
    /// waiting in `wait_for_initialization()`.
    fn populate(&mut self, all_sources: &[&[SegmentReader]], query: &[u8], with_aggregates: bool) {
        self.payload.init(all_sources, query, with_aggregates);
        self.queries_per_worker = [0; WORKER_METRICS_MAX_COUNT];
        self.shared_threshold.reset();
        let primary_count = all_sources.first().expect("primary source empty").len();
        // Set nsegments LAST - this signals initialization is complete.
        // Remaining counts live in `payload.remaining_by_source` (initialized in
        // `ParallelScanPayload::init`); for the partitioning source the slot starts
        // at `primary_count`.
        self.nsegments = primary_count;

        // Wake up any workers waiting in `wait_for_initialization()`.
        self.init_cv.broadcast();
    }

    /// Phase 1: Create the mutex but mark state as uninitialized.
    /// This is called by `aminitparallelscan` before any participants are launched.
    /// The leader will call `populate` to set up the segment data; workers wait for that.
    pub fn create(&mut self) {
        self.mutex.init();
        self.aggregation_cv.init();
        self.init_cv.init();
        self.shared_threshold.init();
        // Mark as uninitialized so workers know to wait for the leader
        self.mark_uninitialized();
    }

    /// Mark the state as uninitialized to signal that a new scan is starting.
    /// Called by amparallelrescan before rescans.
    pub fn mark_uninitialized(&mut self) {
        self.nsegments = PARALLEL_STATE_UNINITIALIZED;
    }

    /// Signal that initialization is complete with zero segments available.
    /// Used to trigger serial scan fallback when the DSM region is too small for the actual
    /// segment count. Wakes workers so they exit cleanly.
    pub fn mark_initialized_empty(&mut self) {
        self.nsegments = 0;
        for slot in self.payload.remaining_by_source_mut() {
            *slot = 0;
        }
        self.init_cv.broadcast();
    }

    pub fn acquire_mutex(&mut self) -> impl Drop {
        self.mutex.acquire()
    }

    fn query_count(&mut self, parallel_worker_number: i32) -> Option<&mut u16> {
        let offset: usize = (parallel_worker_number + 1).try_into().unwrap();
        // We will not record metrics past WORKER_METRICS_MAX_COUNT workers.
        self.queries_per_worker.get_mut(offset)
    }

    /// Increment the count of queries executed by this worker.
    pub fn increment_query_count(&mut self) {
        let _mutex = self.acquire_mutex();
        let parallel_worker_number = unsafe { pg_sys::ParallelWorkerNumber };
        if let Some(query_count) = self.query_count(parallel_worker_number) {
            *query_count = query_count.saturating_add(1);
        }
    }

    /// Append intermediate aggregation results, including the number of segments that they
    /// represent.
    pub fn aggregation_append(
        &mut self,
        mut result: IntermediateAggregationResults,
        segment_count: usize,
    ) -> anyhow::Result<()> {
        let _lock = self.acquire_mutex();

        let (Some(agg_header), Some(agg_data)) = self.payload.aggregates_mut_parts() else {
            panic!("No aggregations expected!");
        };

        let serialized_result = postcard::to_allocvec(&result)?;
        let result_len = serialized_result.len();
        let watermark = agg_header.watermark;
        let max_response_bytes = gucs::max_window_aggregate_response_bytes();

        assert!(
            result_len < max_response_bytes,
            "Initial aggregate result is too large: {result_len:?} vs {max_response_bytes}. \
            Consider increasing the 'paradedb.max_window_aggregate_response_bytes' GUC."
        );

        let buffer_full = watermark + std::mem::size_of::<usize>() + result_len
            > (agg_data.len() - agg_header.serialized_aggregations_len);

        agg_header.received_count += segment_count;
        let all_received = agg_header.received_count == self.nsegments;

        // If there is a room in the buffer, and we have not received all of the requests,
        // serialize it.
        if !buffer_full && !all_received {
            let buffer = &mut agg_data[agg_header.serialized_aggregations_len..];
            // Write length prefix
            let len_bytes = result_len.to_le_bytes();
            buffer[watermark..watermark + len_bytes.len()].copy_from_slice(&len_bytes);
            // Write data
            let data_start = watermark + len_bytes.len();
            buffer[data_start..data_start + result_len].copy_from_slice(&serialized_result);
            // Update watermark
            agg_header.watermark += len_bytes.len() + result_len;

            return Ok(());
        }

        // The buffer is full, or we are receiving the final result value: aggregate them down to a single
        // result, in place.
        let mut current_offset = 0;
        let buffer = &agg_data[agg_header.serialized_aggregations_len..];
        let watermark = agg_header.watermark;

        while current_offset < watermark {
            let len_bytes_end = current_offset + std::mem::size_of::<usize>();
            let len_bytes: [u8; std::mem::size_of::<usize>()] = buffer
                [current_offset..len_bytes_end]
                .try_into()
                .expect("slice with incorrect length");
            let len = usize::from_le_bytes(len_bytes);

            let data_start = len_bytes_end;
            let data_end = data_start + len;
            let res: IntermediateAggregationResults =
                postcard::from_bytes(&buffer[data_start..data_end])?;
            result.merge_fruits(res)?;
            current_offset = data_end;
        }

        let serialized_merged = postcard::to_allocvec(&result)?;
        let merged_len = serialized_merged.len();
        let len_bytes = merged_len.to_le_bytes();

        assert!(
            merged_len < max_response_bytes,
            "Aggregate result is too large: {merged_len:?} vs {max_response_bytes}. \
            Consider increasing the 'paradedb.max_window_aggregate_response_bytes' GUC."
        );

        // Reset buffer and write single merged result
        agg_header.watermark = 0;
        let watermark = agg_header.watermark;
        let buffer = &mut agg_data[agg_header.serialized_aggregations_len..];
        buffer[watermark..watermark + len_bytes.len()].copy_from_slice(&len_bytes);
        let data_start = watermark + len_bytes.len();
        buffer[data_start..data_start + merged_len].copy_from_slice(&serialized_merged);
        agg_header.watermark += len_bytes.len() + merged_len;

        // Wake up any workers waiting in `aggregation_wait()` now that all results are in.
        if all_received {
            self.aggregation_cv.broadcast();
        }

        Ok(())
    }

    /// Wait for intermediate aggregation results to have been reported for all segments, and then
    /// return the single final result.
    pub fn aggregation_wait(&mut self) -> IntermediateAggregationResults {
        loop {
            check_for_interrupts!();

            // Re-arm the condition variable on every iteration.
            // After ConditionVariableSleep returns (spurious wake, interrupt, or broadcast),
            // we're removed from the wait queue. We must re-prepare before sleeping again.
            self.aggregation_cv.prepare_to_sleep();

            // See whether the aggregations has been finalized: if not, keep waiting.
            let lock = self.acquire_mutex();
            let agg_header = self
                .payload
                .aggregates_header()
                .expect("cannot wait for aggregations without an aggregations payload");
            if agg_header.received_count != self.nsegments {
                std::mem::drop(lock);
                self.aggregation_cv.sleep();
                continue;
            }

            // Aggregation has been finalized: deserialize and return it.
            ConditionVariable::cancel_sleep();
            let agg_data = self.payload.aggregates_data().unwrap();
            let buffer = &agg_data[agg_header.serialized_aggregations_len..];
            assert!(
                agg_header.watermark > 0,
                "Expected a finalized aggregation result."
            );
            let len_bytes_end = std::mem::size_of::<usize>();
            let len_bytes: [u8; std::mem::size_of::<usize>()] = buffer[0..len_bytes_end]
                .try_into()
                .expect("slice with incorrect length");
            let len = usize::from_le_bytes(len_bytes);
            let data_start = len_bytes_end;
            let data_end = data_start + len;
            return postcard::from_bytes(&buffer[data_start..data_end])
                .expect("failed to deserialize aggregation result");
        }
    }

    /// Source-aware segment checkout. For the first source (index 0), also records the
    /// claim in the per-segment `claims` array (which is sized to that source's segment
    /// count, so subsequent sources have no slot to write).
    pub fn checkout_segment_for_source(&mut self, source_idx: usize) -> Option<SegmentId> {
        let parallel_worker_number = unsafe { pg_sys::ParallelWorkerNumber };
        self.wait_for_initialization();

        let total = *self.payload.all_counts().get(source_idx)? as usize;
        if total == 0 {
            return None;
        }
        let writes_claims = source_idx == 0;

        #[cfg(not(feature = "pg15"))]
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(50);

        loop {
            let _mutex = self.acquire_mutex();
            let remaining = *self.payload.remaining_by_source_mut().get(source_idx)? as usize;
            if remaining == 0 {
                return None;
            }

            // `debug_parallel_query` leader-defer applies only to the primary
            // source. With small datasets it gives workers a chance to start up
            // before the leader walks the whole pool, which improves the
            // reproducibility of parallel-worker issues; UNIONs under a Gather node
            // may run without workers at all, hence the deadline escape.
            #[cfg(not(feature = "pg15"))]
            if writes_claims
                && unsafe { pg_sys::debug_parallel_query } != 0
                && parallel_worker_number == -1
                && remaining == total
                && std::time::Instant::now() < deadline
            {
                continue;
            }

            // segments are claimed back-to-front; the per-source ids were organized
            // smallest-to-largest by num_docs in `ParallelScanPayload::init`, so
            // claiming back-to-front gives largest-first.
            let claimed_idx = {
                let slot = self.payload.remaining_by_source_mut().get_mut(source_idx)?;
                *slot -= 1;
                *slot as usize
            };
            if writes_claims {
                self.payload.claims_mut()[claimed_idx] = parallel_worker_number;
            }
            return Some(SegmentId::from_bytes(
                self.payload.source_ids(source_idx)[claimed_idx],
            ));
        }
    }

    /// Returns a map of segment IDs to their deleted document counts.
    ///
    /// This method will wait (spin) until the leader has initialized the segment data.
    /// This is necessary for parallel index scans where workers may call this before
    /// the leader has finished initializing the parallel state.
    pub fn segments(&mut self) -> HashMap<SegmentId, u32> {
        // Wait for initialization, then read segment data while holding the mutex.
        self.wait_for_initialization();

        let _mutex = self.acquire_mutex();
        let mut segments = HashMap::default();
        for i in 0..self.nsegments {
            segments.insert(self.segment_id(i), self.num_deleted_docs(i));
        }
        segments
    }

    /// Wait for parallel state to be initialized by the leader.
    fn wait_for_initialization(&mut self) {
        loop {
            // Check for interrupts to allow query cancellation
            pgrx::check_for_interrupts!();

            // Re-arm the condition variable on every iteration.
            // After ConditionVariableSleep returns (spurious wake, interrupt, or broadcast),
            // we're removed from the wait queue. We must re-prepare before sleeping again.
            self.init_cv.prepare_to_sleep();

            // See whether the state has been initialized: if not, keep waiting.
            {
                let _mutex = self.acquire_mutex();
                if self.nsegments != PARALLEL_STATE_UNINITIALIZED {
                    ConditionVariable::cancel_sleep();
                    return;
                }
            }

            self.init_cv.sleep();
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.nsegments != PARALLEL_STATE_UNINITIALIZED
    }

    /// Returns per-worker `ParallelExplainData`.
    pub fn explain_data(&mut self) -> ParallelExplainData {
        let _mutex = self.acquire_mutex();

        let mut workers: BTreeMap<i32, ParallelExplainWorkerData> = BTreeMap::default();
        for (i, &claiming_worker) in self.payload.claims().iter().enumerate().rev() {
            if claiming_worker <= SEGMENT_CLAIM_UNCLAIMED {
                // Segment is unclaimed.
                continue;
            }
            workers
                .entry(claiming_worker)
                .or_default()
                .claimed_segments
                .push(ClaimedSegmentData {
                    id: self.segment_id(i).short_uuid_string(),
                    deleted_docs: self.num_deleted_docs(i),
                    max_doc: self.segment_max_docs(i),
                });
        }
        let mut total_query_count: usize = 0;
        for (parallel_worker_number, worker) in workers.iter_mut() {
            let query_count = self.query_count(*parallel_worker_number).copied();
            total_query_count += query_count.map(|qc| qc as usize).unwrap_or(0);
            worker.query_count = query_count;
        }

        ParallelExplainData {
            total_query_count,
            workers,
        }
    }

    fn segment_id(&self, i: usize) -> SegmentId {
        SegmentId::from_bytes(self.payload.source_ids(0)[i])
    }

    fn num_deleted_docs(&self, i: usize) -> u32 {
        self.payload.primary_deleted_docs()[i]
    }

    fn segment_max_docs(&self, i: usize) -> u32 {
        self.payload.primary_max_docs()[i]
    }

    fn query(&self) -> anyhow::Result<Option<SearchQueryInput>> {
        self.payload.query()
    }

    /// Restore the per-source remaining counts for a rescan. Called by amparallelrescan.
    pub fn reset(&mut self) {
        // Rescan re-partitions the same way as the initial scan, so reset every source's
        // remaining count back to its initial value.
        let counts: Vec<u32> = self.payload.all_counts().to_vec();
        for (slot, count) in self
            .payload
            .remaining_by_source_mut()
            .iter_mut()
            .zip(counts.iter())
        {
            *slot = *count;
        }
        // NOTE: We do not reset `queries_per_worker` here, so that it can be tracked across
        // rescans.
    }

    /// Per-source frozen segment set for `MvccSatisfies::ParallelWorker(ids)`. Lives
    /// in the shared payload so workers don't need a codec channel for it. Waits for
    /// leader init; otherwise a racing worker reads zero IDs.
    pub fn segment_ids_for_source(&mut self, source_idx: usize) -> HashSet<SegmentId> {
        self.wait_for_initialization();
        self.segment_ids_for_source_unlocked(source_idx)
    }

    /// Read-only sibling of [`Self::segment_ids_for_source`] for callers that already
    /// know initialization is complete. No mutex acquire because the IDs are immutable
    /// after `populate`.
    pub fn segment_ids_for_source_unlocked(&self, source_idx: usize) -> HashSet<SegmentId> {
        self.payload
            .source_ids(source_idx)
            .iter()
            .map(|b| SegmentId::from_bytes(*b))
            .collect()
    }
}

extern "C" {
    pub fn IsLogicalWorker() -> bool;
}

/// The ParallelScanState is torn down after `shutdown_custom_scan`, but before
/// `explain_custom_scan` runs. This struct contains any per-worker state that should be captured
/// from the ParallelScanState for the purposes of EXPLAIN.
#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct ParallelExplainData {
    total_query_count: usize,
    workers: BTreeMap<i32, ParallelExplainWorkerData>,
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct ParallelExplainWorkerData {
    query_count: Option<u16>,
    claimed_segments: Vec<ClaimedSegmentData>,
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct ClaimedSegmentData {
    id: String,
    deleted_docs: u32,
    max_doc: u32,
}

// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::api::HashMap;
use crate::gucs;
use crate::postgres::build::is_bm25_index;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::spinlock::Spinlock;
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
mod vacuum;
mod validate;

mod build_parallel;
pub mod catalog;
pub mod customscan;
pub mod datetime;
#[cfg(not(any(feature = "pg17", feature = "pg18")))]
pub mod fake_aminsertcleanup;
pub mod heap;
pub mod index;
mod jsonb_support;
mod parallel;
pub mod rel;
pub mod spinlock;
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

pub fn rel_get_bm25_index(
    relid: pg_sys::Oid,
) -> Option<(rel::PgSearchRelation, rel::PgSearchRelation)> {
    if relid == pg_sys::Oid::INVALID {
        return None;
    }

    let rel = PgSearchRelation::with_lock(relid, pg_sys::AccessShareLock as _);
    rel.indices(pg_sys::AccessShareLock as _)
        .find(is_bm25_index)
        .map(|index| (rel, index))
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
    ids: Range<usize>,
    deleted_docs: Range<usize>,
    max_docs: Range<usize>,
    claims: Range<usize>,
    aggregates_header: Option<Range<usize>>,
    aggregates_data: Option<Range<usize>>,
    /// The padded size of the layout.
    total: Layout,
}

impl ParallelScanPayloadLayout {
    fn new(
        nsegments: usize,
        serialized_query: &[u8],
        with_aggregates: bool,
    ) -> Result<Self, std::alloc::LayoutError> {
        // Query.
        let layout = Layout::from_size_align(serialized_query.len(), 1)?;
        let query_range = 0..(layout.size());

        // Segment ids.
        let ids_layout = Layout::from_size_align(nsegments * SEGMENT_ID_SIZE, 1)?;
        let (layout, ids_offset) = layout.extend(ids_layout)?;
        let ids_range = (ids_offset)..(ids_offset + ids_layout.size());

        // Deleted docs. Must be aligned for u32.
        let deleted_docs_layout = Layout::array::<u32>(nsegments)?;
        let (layout, deleted_docs_offset) = layout.extend(deleted_docs_layout)?;
        let deleted_docs_range =
            (deleted_docs_offset)..(deleted_docs_offset + deleted_docs_layout.size());

        // Max docs. Must be aligned for u32.
        let max_docs_layout = Layout::array::<u32>(nsegments)?;
        let (layout, max_docs_offset) = layout.extend(max_docs_layout)?;
        let max_docs_range = (max_docs_offset)..(max_docs_offset + max_docs_layout.size());

        // Segment claims. Must be aligned for i32.
        let claims_layout = Layout::array::<i32>(nsegments)?;
        let (mut layout, claims_offset) = layout.extend(claims_layout)?;
        let claims_range = (claims_offset)..(claims_offset + claims_layout.size());

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
            ids: ids_range,
            deleted_docs: deleted_docs_range,
            max_docs: max_docs_range,
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
    fn init(&mut self, segments: &[SegmentReader], query: &[u8], with_aggregates: bool) {
        // Compute and assign our Layout: must match what we were allocated with.
        self.layout = ParallelScanPayloadLayout::new(segments.len(), query, with_aggregates)
            .expect("could not layout `ParallelScanPayload` for initialization");

        // Query.
        let query_range = self.layout.query.clone();
        let _ = (&mut self.data_mut()[query_range])
            .write(query)
            .expect("failed to write query bytes");

        // Segment ids.
        let ids_range = self.layout.ids.clone();
        let ids_slice: &mut [[u8; SEGMENT_ID_SIZE]] =
            bytemuck::try_cast_slice_mut(&mut self.data_mut()[ids_range]).unwrap();
        for (segment, target) in segments.iter().zip(ids_slice.iter_mut()) {
            let mut writer = &mut target[..];
            writer
                .write_all(segment.segment_id().uuid_bytes())
                .expect("failed to write segment bytes");
        }

        // Deleted docs.
        let deleted_docs_range = self.layout.deleted_docs.clone();
        let deleted_docs_slice: &mut [u32] =
            bytemuck::try_cast_slice_mut(&mut self.data_mut()[deleted_docs_range]).unwrap();
        for (segment, target) in segments.iter().zip(deleted_docs_slice.iter_mut()) {
            *target = segment.num_deleted_docs();
        }

        // Max docs.
        let max_docs_range = self.layout.max_docs.clone();
        let max_docs_slice: &mut [u32] =
            bytemuck::try_cast_slice_mut(&mut self.data_mut()[max_docs_range]).unwrap();
        for (segment, target) in segments.iter().zip(max_docs_slice.iter_mut()) {
            *target = segment.max_doc();
        }

        // Segment claims.
        for segment_claim in self.segment_claims_mut().iter_mut() {
            *segment_claim = SEGMENT_CLAIM_UNCLAIMED;
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

    fn segment_ids(&self) -> &[[u8; SEGMENT_ID_SIZE]] {
        bytemuck::try_cast_slice(&self.data()[self.layout.ids.clone()]).unwrap()
    }

    fn segment_deleted_docs(&self) -> &[u32] {
        bytemuck::try_cast_slice(&self.data()[self.layout.deleted_docs.clone()]).unwrap()
    }

    fn segment_max_docs(&self) -> &[u32] {
        bytemuck::try_cast_slice(&self.data()[self.layout.max_docs.clone()]).unwrap()
    }

    /// An array of `i32` parallel worker numbers (as returned by pg_sys::ParallelWorkerNumber)
    /// which indicates which worker has claimed each segment at the same idx in the `segments`
    /// array.
    ///
    /// Any value less than `-1` (the leader) indicates that the segment at that idx has not been
    /// claimed.
    fn segment_claims(&self) -> &[i32] {
        bytemuck::try_cast_slice::<u8, i32>(&self.data()[self.layout.claims.clone()]).unwrap()
    }

    /// See `segment_claims`.
    fn segment_claims_mut(&mut self) -> &mut [i32] {
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
    segment_readers: &'a [SegmentReader],
    query: Vec<u8>,
    with_aggregates: bool,
}

// We do not know ahead of time how many workers there will be, so we preallocate fixed size
// arrays for metrics for up to a given number of parallel workers.
const WORKER_METRICS_MAX_COUNT: usize = 256;

#[repr(C)]
pub struct ParallelScanState {
    mutex: Spinlock,
    remaining_segments: usize,
    nsegments: usize,
    queries_per_worker: [u16; WORKER_METRICS_MAX_COUNT],
    payload: ParallelScanPayload, // must be last field, b/c it allocates on the heap after this struct
}

impl ParallelScanState {
    fn size_of(nsegments: usize, serialized_query: &[u8], with_aggregates: bool) -> usize {
        let dynamic_layout =
            ParallelScanPayloadLayout::new(nsegments, serialized_query, with_aggregates)
                .expect("could not layout `ParallelScanPayload` for allocation");
        std::mem::size_of::<Self>() + dynamic_layout.total.size()
    }

    fn init(&mut self, args: ParallelScanArgs) {
        self.mutex.init();
        self.init_without_mutex(args.segment_readers, &args.query, args.with_aggregates);
    }

    fn init_without_mutex(
        &mut self,
        segments: &[SegmentReader],
        query: &[u8],
        with_aggregates: bool,
    ) {
        self.payload.init(segments, query, with_aggregates);
        self.remaining_segments = segments.len();
        self.nsegments = segments.len();
        self.queries_per_worker = [0; WORKER_METRICS_MAX_COUNT];
    }

    fn init_mutex(&mut self) {
        self.mutex.init();
    }

    fn acquire_mutex(&mut self) -> impl Drop {
        self.mutex.acquire()
    }

    fn decrement_remaining_segments(&mut self) -> usize {
        self.remaining_segments -= 1;
        self.remaining_segments
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

        Ok(())
    }

    /// Wait for intermediate aggregation results to have been reported for all segments, and then
    /// return the single final result.
    pub fn aggregation_wait(&mut self) -> IntermediateAggregationResults {
        loop {
            check_for_interrupts!();

            // See whether the aggregations has been finalized: if not, keep waiting.
            let lock = self.acquire_mutex();
            let agg_header = self
                .payload
                .aggregates_header()
                .expect("cannot wait for aggregations without an aggregations payload");
            if agg_header.received_count != self.nsegments {
                std::mem::drop(lock);
                // TODO: Use another synchronization primitive.
                // https://github.com/paradedb/paradedb/issues/3489
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }

            // Aggregation has been finalized: deserialize and return it.
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

    /// Claim a segment for this worker to work on.
    pub fn checkout_segment(&mut self) -> Option<SegmentId> {
        #[cfg(not(any(feature = "pg14", feature = "pg15")))]
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(50);

        loop {
            let _mutex = self.acquire_mutex();
            if self.remaining_segments == 0 {
                break None;
            }

            let parallel_worker_number = unsafe { pg_sys::ParallelWorkerNumber };

            // If debug_parallel_query is enabled and we're the leader, then do not take the first
            // segment (unless a deadline has passed, since in some cases we may not have any workers:
            // e.g. UNIONS under a Gather node, etc).
            //
            // This significantly improves the reproducibility of parallel worker issues with small
            // datasets, since it means that unlike in the non-parallel case, the leader will be
            // unlikely to emit all of the segments before the workers have had a chance to start up.
            #[cfg(not(any(feature = "pg14", feature = "pg15")))]
            if unsafe { pg_sys::debug_parallel_query } != 0
                && parallel_worker_number == -1
                && self.remaining_segments == self.nsegments
                && std::time::Instant::now() < deadline
            {
                continue;
            }

            // segments are claimed back-to-front and they were already organized smallest-to-largest
            // by num_docs over in [`ParallelScanPayload::init()`].
            //
            // this means we're purposely checking out segments from largest-to-smallest.
            let claimed_segment = self.decrement_remaining_segments();
            self.payload.segment_claims_mut()[claimed_segment] = parallel_worker_number;
            break Some(self.segment_id(claimed_segment));
        }
    }

    /// Returns a map of segment IDs to their deleted document counts.
    pub fn segments(&mut self) -> HashMap<SegmentId, u32> {
        let _mutex = self.acquire_mutex();

        let mut segments = HashMap::default();
        for i in 0..self.nsegments {
            segments.insert(self.segment_id(i), self.num_deleted_docs(i));
        }
        segments
    }

    /// Returns per-worker `ParallelExplainData`.
    pub fn explain_data(&mut self) -> ParallelExplainData {
        let _mutex = self.acquire_mutex();

        let mut workers: BTreeMap<i32, ParallelExplainWorkerData> = BTreeMap::default();
        for (i, &claiming_worker) in self.payload.segment_claims().iter().enumerate().rev() {
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
        SegmentId::from_bytes(self.payload.segment_ids()[i])
    }

    fn num_deleted_docs(&self, i: usize) -> u32 {
        self.payload.segment_deleted_docs()[i]
    }

    fn segment_max_docs(&self, i: usize) -> u32 {
        self.payload.segment_max_docs()[i]
    }

    fn query(&self) -> anyhow::Result<Option<SearchQueryInput>> {
        self.payload.query()
    }

    fn reset(&mut self) {
        self.remaining_segments = self.nsegments;
        // NOTE: We do not reset `queries_per_worker` here, so that it can be tracked across
        // rescans.
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

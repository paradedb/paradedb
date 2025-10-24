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
use crate::postgres::build::is_bm25_index;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::spinlock::Spinlock;
use crate::query::SearchQueryInput;

use pgrx::*;
use tantivy::index::SegmentId;
use tantivy::SegmentReader;

mod build;
mod cost;
mod delete;
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

#[derive(Debug)]
#[repr(C)]
struct ParallelScanPayloadLayout {
    query: Range<usize>,
    ids: Range<usize>,
    deleted_docs: Range<usize>,
    max_docs: Range<usize>,
    claims: Range<usize>,
    /// The padded size of the layout.
    total: Layout,
}

impl ParallelScanPayloadLayout {
    fn new(nsegments: usize, serialized_query: &[u8]) -> Result<Self, std::alloc::LayoutError> {
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
        let (layout, claims_offset) = layout.extend(claims_layout)?;
        let claims_range = (claims_offset)..(claims_offset + claims_layout.size());

        Ok(Self {
            query: query_range,
            ids: ids_range,
            deleted_docs: deleted_docs_range,
            max_docs: max_docs_range,
            claims: claims_range,
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
    fn init(&mut self, segments: &[SegmentReader], query: &[u8]) {
        // Compute and assign our Layout: must match what we were allocated with.
        self.layout = ParallelScanPayloadLayout::new(segments.len(), query)
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
    fn size_of(nsegments: usize, serialized_query: &[u8]) -> usize {
        let dynamic_layout = ParallelScanPayloadLayout::new(nsegments, serialized_query)
            .expect("could not layout `ParallelScanPayload` for allocation");
        size_of::<Self>() + dynamic_layout.total.size()
    }

    fn init(&mut self, segments: &[SegmentReader], query: &[u8]) {
        self.mutex.init();
        self.init_without_mutex(segments, query);
    }

    fn init_without_mutex(&mut self, segments: &[SegmentReader], query: &[u8]) {
        self.payload.init(segments, query);
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

    pub fn segments(&self) -> HashMap<SegmentId, u32> {
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

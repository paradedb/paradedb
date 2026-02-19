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

use crate::api::Cardinality;

use crate::api::HashSet;

use crate::postgres::ParallelScanState;

pub use crate::scan::info::RowEstimate;

use pgrx::pg_sys;

use tantivy::index::SegmentId;

/// Compute the number of workers that should be used for the given ExecMethod.
///
/// This calculation determines the "Parallel Awareness" of the path:
/// - If it returns `0`, the path is marked as `parallel_safe` but NOT `parallel_aware`.
///   PostgreSQL may run this scan in a worker (e.g. inner side of a join), but it will
///   be a "replicated" scan where every worker processes the full data set.
/// - If it returns `> 0`, the path is marked as BOTH `parallel_safe` and `parallel_aware`.
///   It becomes a "partial" path that coordinates with other workers via DSM to
///   partition segments and avoid duplicate work.
///
/// Note: PostgreSQL asserts that `parallel_aware` paths must have `parallel_workers > 0`.
pub fn compute_nworkers(
    declares_sorted_output: bool,
    limit: Option<Cardinality>,
    estimated_total_rows: RowEstimate,
    segment_count: usize,
    has_external_quals: bool,
    has_correlated_param: bool,
    is_join_context: bool,
) -> usize {
    // Start with segment-based parallelism. The leader is not included in `nworkers`,
    // so exclude it here. For example: if we expect to need to query 1 segment, then
    // we don't need any workers.
    let mut nworkers = segment_count.saturating_sub(1);

    // Limit workers based on row estimate if we have reliable stats.
    // See: https://github.com/paradedb/paradedb/issues/3055
    //
    // The worker startup overhead (~10ms) means we need enough rows per worker
    // for parallelism to be worthwhile. Based on benchmarks, the crossover point
    // is around 300K rows total.
    //
    // When RowEstimate::Unknown, we don't limit workers based on rows since we can't
    // trust the estimate - the table could be large.
    //
    // Also, if we are in a join context (is_join_context = true), we aggressively claim workers
    // to enable Parallel Hash Join, ignoring the row count threshold.
    //
    // In a single-table query, the overhead of spawning parallel workers might exceed the performance gain.
    // However, in a join, failing to claim parallel workers can prevent the planner from choosing a
    // `Parallel Hash Join`, leading to inefficient plans where joins or sorts are executed serially after a `Gather`.
    if !is_join_context {
        if let RowEstimate::Known(total_rows) = estimated_total_rows {
            let min_rows_per_worker = crate::gucs::min_rows_per_worker() as u64;
            if min_rows_per_worker > 0 {
                // Calculate max workers such that each worker processes at least min_rows_per_worker
                let max_workers_for_rows = (total_rows / min_rows_per_worker) as usize;
                nworkers = nworkers.min(max_workers_for_rows);
            }
        }
    }

    // parallel workers available to a gather node are limited by max_parallel_workers_per_gather
    // and max_parallel_workers
    nworkers = unsafe {
        nworkers
            .min(pg_sys::max_parallel_workers_per_gather as usize)
            .min(pg_sys::max_parallel_workers as usize)
    };

    // if we are not sorting the data (which always requires fetching data from all segments), then
    // limit the number of workers to the number of segments we expect to have to query to reach
    // the limit.
    //
    // Only apply this optimization when we have reliable row estimates.
    if let (false, Some(limit)) = (declares_sorted_output, limit) {
        if let RowEstimate::Known(total_rows) = estimated_total_rows {
            let rows_per_segment = total_rows as f64 / segment_count.max(1) as f64;
            let segments_to_reach_limit = (limit / rows_per_segment).ceil() as usize;
            // See above re: the leader not being included in `nworkers`.
            let nworkers_for_limited_segments = segments_to_reach_limit.saturating_sub(1);
            nworkers = nworkers.min(nworkers_for_limited_segments);
        }
    }

    if has_external_quals {
        // Don't attempt to parallelize if we depend on external variables (e.g. inner side of a nested loop join).
        // This occurs when a qual contains a Param that references a value from another relation
        // (e.g. t1.val @@@ t2.val). In this case, we are likely executing a parameterized scan
        // where we are re-executed for every row of the outer relation. Parallelism here is
        // complex and often not desired.
        //
        // This is distinct from `is_join_context`, which indicates we are part of a join query
        // (e.g. Hash Join) but our scan keys are independent.
        //
        // TODO: Re-evaluate.
        nworkers = 0;
    }

    if has_correlated_param {
        // Don't attempt to parallelize when we have correlated PARAM_EXEC nodes. Uncorrelated
        // params are solved during BeginCustomScan and pushed down to parallel workers, but
        // correlated params need to be evaluated during the scan itself.
        // TODO: Implement proper correlated PARAM_EXEC param sharing with parallel workers.
        nworkers = 0;
    }

    #[cfg(not(feature = "pg15"))]
    unsafe {
        if nworkers == 0 && pg_sys::debug_parallel_query != 0 {
            // force a parallel worker if the `debug_parallel_query` GUC is on
            nworkers = 1;
        }
    }

    nworkers
}

pub unsafe fn checkout_segment(pscan_state: *mut ParallelScanState) -> Option<SegmentId> {
    (*pscan_state).checkout_segment()
}

pub unsafe fn checkout_range_index(pscan_state: *mut ParallelScanState) -> Option<usize> {
    (*pscan_state).checkout_range_index()
}

pub unsafe fn list_segment_ids(pscan_state: *mut ParallelScanState) -> HashSet<SegmentId> {
    (*pscan_state).segments().into_keys().collect()
}

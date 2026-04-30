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

    // When the scan declares sorted output (TopK with ORDER BY, or sorted columnar),
    // skip row-based worker reductions. TopK must scan ALL segments to produce globally
    // correct results, so the cost is segment-scan dominated and row-count thresholds
    // would starve parallelism for queries matching few rows across many segments.
    // Sorted columnar is lazy (SortPreservingMergeExec can stop early), but we
    // conservatively skip reductions for it too since it still benefits from parallelism
    // across segments.
    //
    // For unsorted scans with reliable row estimates (RowEstimate::Known), we apply two
    // reductions to avoid spawning workers whose startup overhead exceeds the benefit:
    //
    // 1. Limit-based: cap workers to the number of segments needed to reach the LIMIT.
    // 2. Row-based: cap so each worker processes at least `min_rows_per_worker` rows
    //    (~300K default, based on benchmarks where worker startup is ~10ms).
    //    Skipped in join contexts to avoid preventing Parallel Hash Join.
    //
    // When RowEstimate::Unknown (table not ANALYZEd), we don't limit workers since
    // we can't trust the estimate.
    //
    // See: https://github.com/paradedb/paradedb/issues/3055
    if !declares_sorted_output {
        if let RowEstimate::Known(total_rows) = estimated_total_rows {
            // Cap to the number of segments needed to reach the LIMIT
            if let Some(limit) = limit {
                let rows_per_segment = total_rows as f64 / segment_count.max(1) as f64;
                let segments_to_reach_limit = (limit / rows_per_segment).ceil() as usize;
                // The leader is not included in `nworkers`, so subtract 1.
                let nworkers_for_limited_segments = segments_to_reach_limit.saturating_sub(1);
                nworkers = nworkers.min(nworkers_for_limited_segments);
            }

            // Cap so each worker processes at least min_rows_per_worker rows.
            // Skipped for joins: failing to claim workers can prevent the planner from
            // choosing Parallel Hash Join, leading to inefficient serial plans.
            if !is_join_context {
                let min_rows_per_worker = crate::gucs::min_rows_per_worker() as u64;
                #[allow(clippy::manual_checked_ops)]
                if min_rows_per_worker > 0 {
                    let max_workers_for_rows = (total_rows / min_rows_per_worker) as usize;
                    nworkers = nworkers.min(max_workers_for_rows);
                }
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

pub unsafe fn list_segment_ids(pscan_state: *mut ParallelScanState) -> HashSet<SegmentId> {
    (*pscan_state).segments().into_keys().collect()
}

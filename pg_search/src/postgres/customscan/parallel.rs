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

use crate::index::mvcc::SegmentView;

use crate::postgres::ParallelScanState;

pub use crate::scan::info::RowEstimate;

use crate::aggregate::AggregateRequest;
use crate::api::operator::estimate_selectivity_and_cost;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::rel::PgSearchRelation;
use crate::query::SearchQueryInput;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;

use tantivy::index::SegmentId;
use tantivy::query::{
    AllQuery, BooleanQuery, BoostQuery, ConstScoreQuery, EmptyQuery, Query, TermQuery,
};
use tantivy::{Searcher, query::EnableScoring};

/// Why a scan selected its worker budget.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) enum WorkerDecisionReason {
    /// Prunable primarily score-DESC ordering: Block-WAND keeps serial scoring sublinear (#4664), so
    /// workers would only add overhead.
    BlockWandPrunable,
    /// Costable scan, no effective LIMIT: pg_search offered both paths and let PostgreSQL choose. (A
    /// costable scan with no workers to split across also lands here -- it just emits serial.)
    CostModel,
    /// Costable scan with an effective LIMIT (top-K / unsorted LIMIT): pg_search costed the Gather on
    /// `k` and forced the winner, because PostgreSQL over-costs a bounded Gather (see module docs).
    CostModelLimited,
    /// Use the segment-based worker budget when cost estimates are unavailable.
    PerSegment,
    /// A bare document count without MVCC filtering uses the serial count fast path.
    DocumentCount,
    /// The row-count heuristic (`compute_nworkers`): no ANALYZE stats, or an unsorted scan with no
    /// usable cost estimate. Caps workers so each gets at least `min_rows_per_worker` rows.
    RowHeuristic,
}

impl std::fmt::Display for WorkerDecisionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::BlockWandPrunable => "Prunable top-K",
            Self::CostModel => "Cost model",
            Self::CostModelLimited => "Cost model (LIMIT)",
            Self::PerSegment => "Per-segment",
            Self::DocumentCount => "Document count",
            Self::RowHeuristic => "Row-capped",
        })
    }
}

pub(crate) struct ParallelCost {
    pub estimated_work: f64,
    pub parallel_threshold: f64,
}

/// Workers plus a full share for the leader when it participates.
pub(crate) fn parallel_divisor(nworkers: NonZeroUsize, leader_participates: bool) -> f64 {
    if leader_participates {
        (nworkers.get() + 1) as f64
    } else {
        nworkers.get() as f64
    }
}

pub(crate) fn parallel_threshold(
    nworkers: NonZeroUsize,
    leader_participates: bool,
    transfer_cost: f64,
) -> f64 {
    let divisor = parallel_divisor(nworkers, leader_participates);
    if divisor > 1.0 {
        (unsafe { pg_sys::parallel_setup_cost } + transfer_cost) / (1.0 - 1.0 / divisor)
    } else {
        f64::INFINITY
    }
}

/// Compare divided work plus startup and transfer costs with serial work.
/// Transfer covers result rows for Top K, or one partial aggregate per worker.
pub(crate) fn parallel_scan_is_cheaper(
    work: f64,
    nworkers: NonZeroUsize,
    leader_participates: bool,
    transfer_cost: f64,
) -> bool {
    work > parallel_threshold(nworkers, leader_participates, transfer_cost)
}

/// Use workers when dividing traversal, collector updates, and heap visibility checks
/// saves more than parallel startup and transferring one partial result per worker.
/// Unknown estimates keep the existing worker budget.
pub(crate) fn aggregate_nworkers(
    index: &PgSearchRelation,
    reader: &SearchIndexReader,
    query: &SearchQueryInput,
    aggregation: &AggregateRequest,
    solve_mvcc: bool,
) -> (usize, Option<ParallelCost>) {
    unsafe {
        let nworkers = (pg_sys::max_parallel_workers_per_gather as usize)
            .min(reader.segment_readers().len())
            .saturating_sub(usize::from(pg_sys::parallel_leader_participation));
        let Some(workers) = NonZeroUsize::new(
            clamp_to_gather_limits(nworkers).min(pg_sys::max_worker_processes as usize),
        ) else {
            return (0, None);
        };
        let nworkers = workers.get();
        if query.has_heap_filters() || query.has_postgres_expressions() {
            return (nworkers, None);
        }
        let Some(updates_per_doc) = aggregation.updates_per_doc(reader) else {
            return (nworkers, None);
        };
        let Some(heap) = index.heap_relation() else {
            return (nworkers, None);
        };
        let total_rows = RowEstimate::from_reltuples(heap.reltuples().map(f64::from));
        let (selectivity, Some(cost)) =
            estimate_selectivity_and_cost(index, query.clone(), Some(reader))
        else {
            return (nworkers, None);
        };
        let rows = selectivity
            .zip(total_rows.known_rows())
            .map(|(selectivity, rows)| (selectivity * rows).ceil());
        if rows.is_none() && (updates_per_doc > 0 || solve_mvcc) {
            return (nworkers, None);
        }
        let rows = rows.unwrap_or(0.0);
        // The catalog visibility fraction can lag recent writes, as in PostgreSQL costing.
        let stats = &*heap.rd_rel;
        let all_visible = if crate::gucs::enable_visibility_map_shortcuts() && stats.relpages > 0 {
            (stats.relallvisible as f64 / stats.relpages as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let heap_checks = if solve_mvcc {
            rows * (1.0 - all_visible)
        } else {
            0.0
        };
        let work = cost as f64 * pg_sys::cpu_index_tuple_cost
            + rows * updates_per_doc as f64 * pg_sys::cpu_operator_cost
            + heap_checks * pg_sys::cpu_tuple_cost;
        let transfer_cost = workers.get() as f64 * pg_sys::parallel_tuple_cost;
        let mut parallel_cost = ParallelCost {
            estimated_work: work,
            parallel_threshold: parallel_threshold(
                workers,
                pg_sys::parallel_leader_participation,
                transfer_cost,
            ),
        };
        if matches!(aggregation, AggregateRequest::Sql(clause)
            if clause.is_bare_doc_count() && clause.aggregates().all(|agg| agg.can_use_doc_count()))
            && (!solve_mvcc || all_visible == 1.0)
            && work > parallel_cost.parallel_threshold
            && work
                <= parallel_cost.parallel_threshold
                    * crate::gucs::count_parallel_threshold_multiplier()
            && has_fast_count(reader.query(), reader.searcher())
        {
            parallel_cost.parallel_threshold *= crate::gucs::count_parallel_threshold_multiplier();
        }
        let nworkers = if work > parallel_cost.parallel_threshold {
            nworkers
        } else {
            0
        };
        pgrx::debug1!(
            "aggregate traversal cost={cost}, matching rows={rows}, heap checks={heap_checks}, requested parallel workers={nworkers}"
        );
        (nworkers, Some(parallel_cost))
    }
}

fn has_fast_count(query: &dyn Query, searcher: &Searcher) -> bool {
    if let Some(query) = query.downcast_ref::<BoostQuery>() {
        return has_fast_count(query.query().as_ref(), searcher);
    }
    if let Some(query) = query.downcast_ref::<ConstScoreQuery>() {
        return has_fast_count(query.query().as_ref(), searcher);
    }
    if query.is::<AllQuery>() || query.is::<EmptyQuery>() || query.is::<ConstScoreQuery<AllQuery>>()
    {
        return true;
    }
    if query.is::<TermQuery>() {
        if searcher
            .segment_readers()
            .iter()
            .all(|segment| !segment.has_deletes())
        {
            return true;
        }
    } else if !query.downcast_ref::<BooleanQuery>().is_some_and(|query| {
        query
            .clauses()
            .iter()
            .all(|(_, query)| has_fast_count(query.as_ref(), searcher))
    }) {
        return false;
    }
    let Ok(weight) =
        query.weight(EnableScoring::disabled_from_searcher(searcher).with_bitmap_postings(true))
    else {
        return false;
    };
    searcher.segment_readers().iter().all(|segment| {
        weight
            .scorer(segment, 1.0)
            .is_ok_and(|scorer| scorer.has_fast_bitset())
    })
}

fn clamp_to_gather_limits(nworkers: usize) -> usize {
    unsafe {
        nworkers
            .min(pg_sys::max_parallel_workers_per_gather as usize)
            .min(pg_sys::max_parallel_workers as usize)
    }
}

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

    // For scans with reliable row estimates (RowEstimate::Known) we cap workers two
    // ways; an Unknown estimate (table not ANALYZEd) caps nothing, since we can't
    // trust it:
    //
    // 1. Limit-based (UNSORTED only): cap to the segments needed to reach LIMIT.
    //    Sorted output must scan every segment to produce a correct global order, so
    //    it is exempt (#4457).
    // 2. Row-based: cap so each worker processes at least `min_rows_per_worker` rows
    //    (~300K default), for both sorted and unsorted output. Skipped in join
    //    contexts to avoid preventing Parallel Hash Join.
    //
    // See: https://github.com/paradedb/paradedb/issues/3055
    if let RowEstimate::Known(total_rows) = estimated_total_rows {
        // Cap to the number of segments needed to reach the LIMIT. Unsorted only:
        // sorted output needs every segment, so it is exempt (#4457).
        if let (false, Some(limit)) = (declares_sorted_output, limit) {
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

    nworkers = clamp_to_gather_limits(nworkers);

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

/// Maximum number of useful parallel workers given structural constraints only.
///
/// Unlike `compute_nworkers`, this does NOT gate on row count or the
/// `min_rows_per_worker` GUC. The caller uses the returned upper bound in its
/// serial-vs-parallel cost comparison before emitting one chosen path (see
/// #4664).
///
/// Returns 0 if parallelism is structurally impossible:
/// - External quals (parameterized scan in a nested loop).
/// - Correlated PARAM_EXEC nodes.
///
/// Otherwise returns `min(segment_count - 1, max_parallel_workers_per_gather,
/// max_parallel_workers)`. The leader is excluded from `segment_count - 1`.
pub fn max_useful_workers(
    segment_count: usize,
    has_external_quals: bool,
    has_correlated_param: bool,
) -> usize {
    if has_external_quals || has_correlated_param {
        return 0;
    }

    // Only the pg15+ `debug_parallel_query` block below mutates this; on pg15
    // that block is compiled out, leaving the binding immutable.
    #[cfg_attr(feature = "pg15", allow(unused_mut))]
    let mut nworkers = clamp_to_gather_limits(segment_count.saturating_sub(1));

    #[cfg(not(feature = "pg15"))]
    unsafe {
        if nworkers == 0 && pg_sys::debug_parallel_query != 0 {
            nworkers = 1;
        }
    }

    nworkers
}

pub unsafe fn checkout_segment_for_source(
    pscan_state: *mut ParallelScanState,
    source_idx: usize,
) -> Option<SegmentId> {
    (*pscan_state).checkout_segment_for_source(source_idx)
}

pub unsafe fn segment_view(pscan_state: *mut ParallelScanState) -> SegmentView {
    (*pscan_state).segment_view()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::query::QueryParser;
    use tantivy::schema::{Schema, TEXT};
    use tantivy::{Index, TantivyDocument, doc, indexer::NoMergePolicy};

    #[test]
    fn fast_counts_exclude_positional_and_ordinary_iteration() -> tantivy::Result<()> {
        for bitmaps in [false, true] {
            let mut schema = Schema::builder();
            let text = schema.add_text_field(
                "body",
                TEXT.set_indexing_options(
                    TEXT.get_indexing_options()
                        .unwrap()
                        .clone()
                        .set_bitmap_postings(bitmaps),
                ),
            );
            let mut index = Index::create_in_ram(schema.build());
            index.settings_mut().bitmap_postings.use_for_queries = bitmaps;
            let mut writer = index.writer_with_num_threads::<TantivyDocument>(1, 50_000_000)?;
            writer.set_merge_policy(Box::new(NoMergePolicy));
            for i in 0..1024 {
                let body = match i {
                    0 => "alpha beta rare",
                    i if i % 2 == 0 => "alpha beta",
                    _ => "gamma",
                };
                writer.add_document(doc!(text => body))?;
            }
            writer.commit()?;
            let searcher = index.reader()?.searcher();
            let parser = QueryParser::for_index(&index, vec![text]);
            for (query, expected) in [
                ("*", true),
                ("alpha", true),
                ("rare", true),
                ("alpha OR beta", bitmaps),
                ("alpha AND beta", bitmaps),
                ("rare AND alpha", false),
                ("\"alpha beta\"", false),
                ("alpha OR \"alpha beta\"", false),
            ] {
                let parsed = parser.parse_query(query)?;
                assert_eq!(
                    has_fast_count(parsed.as_ref(), &searcher),
                    expected,
                    "{query}, bitmaps={bitmaps}"
                );
                assert_eq!(
                    has_fast_count(&BoostQuery::new(parsed.box_clone(), 2.0), &searcher),
                    expected
                );
                assert_eq!(
                    has_fast_count(&ConstScoreQuery::new(parsed, 1.0), &searcher),
                    expected
                );
            }
            if bitmaps {
                for i in 0..1024 {
                    writer
                        .add_document(doc!(text => if i == 0 { "alpha beta" } else { "gamma" }))?;
                }
                writer.commit()?;
                let searcher = index.reader()?.searcher();
                assert_eq!(searcher.segment_readers().len(), 2);
                assert!(!has_fast_count(
                    parser.parse_query("alpha OR beta")?.as_ref(),
                    &searcher
                ));
            }
        }
        Ok(())
    }
}

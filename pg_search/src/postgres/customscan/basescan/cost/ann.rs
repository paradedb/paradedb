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

use super::{ScanParallelismInputs, WorkerPathPolicy, drive_work, parallel_tuple_cost};
use crate::DEFAULT_STARTUP_COST;
use crate::api::OrderByFeature;
use crate::gucs::{vector_cluster_max_probe, vector_fixed_probe_cost_rows, vector_max_scan_levels};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::basescan::query_has_window_agg_functions;
use crate::postgres::customscan::builders::custom_path::ExecMethodType;
use crate::postgres::customscan::parallel::{
    ParallelCost, WorkerDecisionReason, max_useful_workers, parallel_divisor, parallel_threshold,
};
use crate::postgres::rel::PgSearchRelation;
#[cfg(any(test, feature = "pg_test"))]
use pgrx::pg_schema;
#[cfg(not(feature = "pg15"))]
use pgrx::pg_sys::debug_parallel_query;
use pgrx::pg_sys::{cpu_operator_cost, parallel_setup_cost};
use std::num::NonZeroUsize;
use tantivy::vector::ProbeParams;
use tantivy::vector::diagnostics::{VectorInfo, VectorStorageFormat};

pub(in super::super) struct AnnWork {
    pub total: f64,
    pub largest_segment: f64,
}

pub(in super::super) unsafe fn estimate_ann_work(
    method: &ExecMethodType,
    index: &PgSearchRelation,
    inputs: &ScanParallelismInputs,
) -> Option<AnnWork> {
    let ExecMethodType::TopK {
        orderby_info: Some(orderby),
        window_aggregates,
        limit_offset,
        ..
    } = method
    else {
        return None;
    };
    let OrderByFeature::VectorDistance { name, .. } = &orderby.first()?.feature else {
        return None;
    };
    if !window_aggregates.is_empty()
        || limit_offset.has_any_param()
        || query_has_window_agg_functions(inputs.root)
        || inputs.quals.contains_external_var()
        || inputs.quals.contains_correlated_param(inputs.root)
        || inputs.query.has_heap_filters()
        || inputs.query.has_postgres_expressions()
    {
        return None;
    }
    let matches = inputs.row_estimate.known_rows()?;
    let k = inputs.limit?;
    let filter_work = if inputs.query.is_match_all() {
        0.0
    } else {
        drive_work(inputs.drive_cost?, 1.0)
    };
    let reader = SearchIndexReader::empty(index, MvccSatisfies::LargestSegment).ok()?;
    let segment = reader.segment_readers().first()?;
    let field = index
        .schema()
        .ok()?
        .tantivy_schema()
        .get_field(&name.root())
        .ok()?;
    let vectors = segment.vector_index(field).ok()?;
    let info = vectors.info()?;
    let docs = reader.total_docs() as f64;
    let sample_docs = segment.num_docs() as f64;
    if docs == 0.0 || sample_docs == 0.0 {
        return None;
    }
    let selectivity = if inputs.query.is_match_all() {
        1.0
    } else {
        (matches / docs).clamp(0.0, 1.0)
    };
    let params = ProbeParams {
        max_probe_fraction: vector_cluster_max_probe(),
        fixed_probe_cost_rows: vector_fixed_probe_cost_rows(),
        ..Default::default()
    };
    let layers = info.quantized_layers.as_deref().unwrap_or_default();
    let max_layers = vector_max_scan_levels();
    // A truncated quantization prefix has different pruning quality; keep the fallback policy.
    if max_layers != 0 && max_layers < layers.len() {
        return None;
    }
    let sample_bytes = segment_bytes(
        &info,
        vectors.dim(),
        1.0,
        selectivity,
        k,
        &params,
        max_layers,
    );
    let other_segments = reader.total_segment_count().saturating_sub(1) as f64;
    let other_scale = (docs - sample_docs).max(0.0) / sample_docs / other_segments.max(1.0);
    let other_bytes = segment_bytes(
        &info,
        vectors.dim(),
        other_scale,
        selectivity,
        k,
        &params,
        max_layers,
    );
    // Normalize vector traffic to 64-byte units charged at cpu_operator_cost.
    let byte_cost = unsafe { cpu_operator_cost } / 64.0;
    Some(AnnWork {
        total: filter_work + (sample_bytes + other_segments * other_bytes) * byte_cost,
        largest_segment: filter_work * sample_docs / docs
            + sample_bytes.max(other_bytes) * byte_cost,
    })
}

fn segment_bytes(
    info: &VectorInfo,
    dim: usize,
    scale: f64,
    selectivity: f64,
    k: f64,
    params: &ProbeParams,
    max_layers: usize,
) -> f64 {
    let rows = info.num_vectors as f64 * scale;
    let matches = rows * selectivity;
    if matches <= 0.0 || k <= 0.0 {
        return 0.0;
    }
    let full_width = (dim * size_of::<f32>()) as f64;
    if info.format == VectorStorageFormat::Flat {
        return matches * full_width;
    }
    let clusters = (info.num_centroids.unwrap_or(0) as f64 * scale).max(1.0);
    let average = rows / clusters;
    let open_share =
        (params.fixed_probe_cost_rows / (params.fixed_probe_cost_rows + average)).min(0.5);
    let budget = (clusters * f64::from(params.max_probe_fraction))
        .max(params.min_probe_clusters as f64)
        .min(clusters);
    let located = selectivity < 1.0 && matches <= budget * average / (1.0 - open_share);
    let (candidates, routing) = if located {
        (matches, 0.0)
    } else {
        let probes = (budget / (open_share + (1.0 - open_share) * selectivity))
            .ceil()
            .min(clusters);
        let posting_size = info
            .cluster_stats
            .as_ref()
            .map_or(average, |s| s.avg_cluster_size);
        (
            (probes * posting_size * selectivity).min(matches),
            clusters * full_width,
        )
    };
    let quantized_width = info
        .quantized_bytes_per_row
        .filter(|_| max_layers > 0)
        .unwrap_or(0) as f64;
    if candidates <= 0.0 {
        return routing;
    }
    if quantized_width == 0.0 {
        return routing + candidates * full_width;
    }
    let retained = k.min(candidates);
    // Random-order top-K admits k * (1 + ln(n/k)) rows. Quantized bounds admit extra
    // candidates; this is a work estimate, not a bound on full-precision reranking.
    const RERANK_HEADROOM: f64 = 2.0;
    let exact = (RERANK_HEADROOM * retained * (1.0 + (candidates / retained).ln())).min(candidates);
    routing + candidates * quantized_width + exact * full_width
}

pub(in super::super) unsafe fn decide_ann_parallelism(
    work: &AnnWork,
    inputs: &ScanParallelismInputs,
) -> (WorkerPathPolicy, Option<ParallelCost>) {
    let reason = WorkerDecisionReason::AnnCostModel;
    let max_workers = if inputs.consider_parallel {
        max_useful_workers(inputs.segment_count, false, false)
    } else {
        0
    };
    let mut policy = WorkerPathPolicy::SerialOnly { reason };
    let mut best_cost = f64::INFINITY;
    let mut diagnostic = None;
    for workers in 1..=max_workers {
        let workers = NonZeroUsize::new(workers).unwrap();
        let divisor = parallel_divisor(workers, inputs.parallel_leader_participates);
        if divisor <= 1.0 {
            continue;
        }
        let divided = (work.total / divisor).max(work.largest_segment);
        let transfer =
            parallel_tuple_cost() * inputs.base_result_rows * workers.get() as f64 * 1.05;
        let overhead = transfer + workers.get() as f64 * DEFAULT_STARTUP_COST;
        let threshold = parallel_threshold(
            workers,
            inputs.parallel_leader_participates,
            overhead + divided - work.total / divisor,
        );
        let candidate = unsafe { parallel_setup_cost } + overhead + divided;
        if candidate < best_cost {
            best_cost = candidate;
            diagnostic = Some(ParallelCost {
                estimated_work: work.total,
                parallel_threshold: threshold,
            });
            policy = if candidate < work.total {
                WorkerPathPolicy::ParallelOnly {
                    nworkers: workers,
                    reason,
                }
            } else {
                WorkerPathPolicy::SerialOnly { reason }
            };
        }
    }
    #[cfg(not(feature = "pg15"))]
    if unsafe { debug_parallel_query != 0 }
        && let Some(nworkers) = NonZeroUsize::new(max_workers)
    {
        policy = WorkerPathPolicy::ParallelOnly { nworkers, reason };
    }
    (policy, diagnostic)
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use super::*;
    use crate::postgres::customscan::basescan::cost::TopKPrunability;
    use crate::postgres::customscan::parallel::RowEstimate;
    use crate::postgres::customscan::qual_inspect::Qual;
    use crate::query::SearchQueryInput;
    use pgrx::{Json, Spi, pg_test};
    use std::ptr::null_mut;
    use tantivy::vector::diagnostics::VectorClusterStats;

    #[pg_test]
    fn ann_work_tracks_probe_budget_and_reranking() {
        let mut info = VectorInfo {
            format: VectorStorageFormat::Ivf,
            num_vectors: 1_000_000,
            num_centroids: Some(2500),
            cluster_stats: Some(VectorClusterStats {
                min_cluster_size: 400,
                max_cluster_size: 400,
                avg_cluster_size: 400.0,
                empty_clusters: 0,
            }),
            quantized_layers: Some(vec![1, 1]),
            quantized_bytes_per_row: Some(276),
        };
        let params = ProbeParams {
            max_probe_fraction: 0.02,
            ..Default::default()
        };
        let small = segment_bytes(&info, 1024, 1.0, 1.0, 10.0, &params, 3);
        let large = segment_bytes(&info, 1024, 1.0, 1.0, 1000.0, &params, 3);
        let exact = segment_bytes(&info, 1024, 1.0, 1.0, 10.0, &params, 0);
        assert!(small < large && large < exact);
        assert_eq!(exact, (2500.0 + 50.0 * 400.0) * 4096.0);
        assert_eq!(
            segment_bytes(&info, 1024, 1.0, 0.001, 10.0, &params, 0),
            1000.0 * 4096.0
        );
        assert_eq!(segment_bytes(&info, 1024, 1.0, 0.0, 10.0, &params, 3), 0.0);
        info.format = VectorStorageFormat::Flat;
        assert_eq!(
            segment_bytes(&info, 1024, 1.0, 0.5, 10.0, &params, 3),
            500_000.0 * 4096.0
        );
    }

    #[pg_test]
    fn ann_parallel_cost_accounts_for_startup_and_segment_skew() {
        Spi::run(
            "SET LOCAL parallel_setup_cost = 1000;
            SET LOCAL parallel_tuple_cost = 0.1;
            SET LOCAL max_parallel_workers = 8;
            SET LOCAL max_parallel_workers_per_gather = 2;",
        )
        .unwrap();
        #[cfg(not(feature = "pg15"))]
        Spi::run("SET LOCAL debug_parallel_query = off;").unwrap();
        let mut inputs = ScanParallelismInputs {
            prunability: TopKPrunability::NotPrunable,
            query: &SearchQueryInput::All,
            drive_cost: None,
            row_estimate: RowEstimate::Known(1_000_000),
            is_sorted: true,
            limit: Some(10.0),
            base_result_rows: 10.0,
            segment_count: 8,
            consider_parallel: true,
            quals: &Qual::All,
            root: null_mut(),
            parallel_leader_participates: true,
            is_join_context: false,
            has_grouping: false,
        };
        for (total, largest_segment, expected) in [
            (1000.0, 200.0, 0),
            (4000.0, 800.0, 2),
            (4000.0, 2200.0, 1),
            (4000.0, 3900.0, 0),
        ] {
            let (policy, cost) = unsafe {
                decide_ann_parallelism(
                    &AnnWork {
                        total,
                        largest_segment,
                    },
                    &inputs,
                )
            };
            let workers = match policy {
                WorkerPathPolicy::ParallelOnly { nworkers, .. } => nworkers.get(),
                WorkerPathPolicy::SerialOnly { .. } => 0,
                WorkerPathPolicy::CostedBoth { .. } => panic!("ANN should choose a worker budget"),
            };
            assert_eq!(workers, expected);
            assert_eq!(total > cost.unwrap().parallel_threshold, expected > 0);
        }
        inputs.parallel_leader_participates = false;
        Spi::run("SET LOCAL max_parallel_workers_per_gather = 1;").unwrap();
        let (policy, cost) = unsafe {
            decide_ann_parallelism(
                &AnnWork {
                    total: 4000.0,
                    largest_segment: 800.0,
                },
                &inputs,
            )
        };
        assert!(matches!(policy, WorkerPathPolicy::SerialOnly { .. }));
        assert!(cost.is_none());
    }

    #[pg_test]
    fn ann_generic_plan_uses_vector_work() {
        Spi::run(
            "SET LOCAL max_parallel_workers_per_gather = 2;
            SET LOCAL max_parallel_workers = 8;
            SET LOCAL min_parallel_table_scan_size = 0;
            SET LOCAL parallel_setup_cost = 1000;
            SET LOCAL parallel_tuple_cost = 0.1;
            SET LOCAL cpu_operator_cost = 0.0025;
            SET LOCAL plan_cache_mode = force_generic_plan;
            SET LOCAL paradedb.global_mutable_segment_rows = 0;
            CREATE TABLE ann_cost_vectors (id int PRIMARY KEY, embedding vector(16));
            CREATE INDEX ann_cost_vectors_idx ON ann_cost_vectors USING bm25
                (id, embedding vector_l2_ops)
                WITH (target_segment_count = 4, mutable_segment_rows = 0,
                      layer_sizes = '10TB', background_layer_sizes = '10TB');",
        )
        .unwrap();
        for batch in 0..4 {
            Spi::run(&format!(
                "INSERT INTO ann_cost_vectors
                SELECT i, (ARRAY[i::real] || array_fill(1::real, ARRAY[15]))::vector
                FROM generate_series({}, {}) i",
                batch * 500 + 1,
                (batch + 1) * 500
            ))
            .unwrap();
        }
        Spi::run(
            "ANALYZE ann_cost_vectors;
            PREPARE ann_cost_query(vector) AS SELECT id FROM ann_cost_vectors
            WHERE id @@@ pdb.all() ORDER BY embedding <-> $1 LIMIT 10;",
        )
        .unwrap();
        let query = "EXECUTE ann_cost_query('[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1]')";
        for (operator_cost, parallel) in [(0.0025, false), (4.0, true)] {
            Spi::run(&format!(
                "SET LOCAL cpu_operator_cost = {operator_cost}; DISCARD PLANS;"
            ))
            .unwrap();
            let plan = Spi::get_one::<Json>(&format!("EXPLAIN (VERBOSE, FORMAT JSON) {query}"))
                .unwrap()
                .unwrap()
                .0;
            let mut nodes = vec![&plan[0]["Plan"]];
            let mut workers = 0;
            let mut found = false;
            while let Some(node) = nodes.pop() {
                workers += node["Workers Planned"].as_u64().unwrap_or(0);
                if node["Index"] == "ann_cost_vectors_idx" {
                    assert_eq!(node["Worker Selection"], "ANN cost model");
                    let work = node["Estimated Query Work"].as_f64().unwrap();
                    let threshold = node["Parallel Threshold"].as_f64().unwrap();
                    assert_eq!(work > threshold, parallel);
                    found = true;
                }
                if let Some(children) = node["Plans"].as_array() {
                    nodes.extend(children);
                }
            }
            assert!(found);
            assert_eq!(workers > 0, parallel);
            let ids = Spi::connect(|client| {
                client
                    .select(query, None, &[])
                    .unwrap()
                    .map(|row| row["id"].value::<i32>().unwrap().unwrap())
                    .collect::<Vec<_>>()
            });
            assert_eq!(ids, (1..=10).collect::<Vec<_>>());
        }
        Spi::run("DEALLOCATE ann_cost_query;").unwrap();
    }
}

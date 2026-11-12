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

use super::cost::{WorkerDecisionReason, WorkerPathPolicy, parallel_divisor};
use crate::api::FieldName;
use crate::gucs;
use crate::postgres::options::BM25IndexOptions;
use crate::query::SearchQueryInput;
use crate::schema::SearchFieldType;
use std::num::NonZeroUsize;

// Minimum estimated byte-work saved to cover worker startup and coordination.
const MIN_PARALLEL_SAVINGS_BYTES: f64 = 64.0 * 1024.0 * 1024.0;

impl WorkerPathPolicy {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn for_vector_topk(
        query: &SearchQueryInput,
        uses_fast_fields: Option<bool>,
        vectors: f64,
        selectivity: f64,
        field: &FieldName,
        options: &BM25IndexOptions,
        workers: usize,
        leader_participates: bool,
    ) -> Self {
        let reason = WorkerDecisionReason::VectorWork;
        // TODO: We have not yet seen a speedup for parallel unfiltered queries, but should be
        // able to with a good global adaptive probing model.
        if query.is_match_all() {
            return Self::SerialOnly { reason };
        }
        let Some(nworkers) = NonZeroUsize::new(workers) else {
            return Self::SerialOnly { reason };
        };
        let Some(SearchFieldType::Vector(_, dimensions, _)) = options.get_field_type(field) else {
            return Self::SerialOnly { reason };
        };
        let stats = VectorSearchStats {
            vectors,
            matches: vectors * selectivity,
        };
        let layers = options.field_config_or_default(field).quantization_layers();
        let full_row_bytes = (dimensions * size_of::<f32>()) as f64;
        let row_bytes = layers.as_ref().map_or(full_row_bytes, |layers| {
            (dimensions * usize::from(layers[0])).div_ceil(u8::BITS as usize) as f64
        });
        let average_cluster_size =
            options.target_leaf_size() as f64 / f64::from(options.training_sample_ratio());
        let clusters = (vectors / average_cluster_size).max(1.0);
        let replication = if layers.is_some() && options.vector_router_quantization() {
            options.replication()
        } else {
            1
        };
        let average = average_cluster_size * f64::from(replication);
        let x = (gucs::vector_fixed_probe_cost_rows()
            / (gucs::vector_fixed_probe_cost_rows() + average))
            .min(0.5);
        let cap = (gucs::vector_cluster_max_probe() as f64).min(clusters) * average / (1.0 - x);
        let column_filter = uses_fast_fields.unwrap_or_else(|| !query.is_text_only());
        let late = column_filter
            && layers.is_some()
            && gucs::vector_max_scan_levels() > 0
            && stats.matches > cap;
        // TODO: Account for column materialization when estimating eager filter work.
        if column_filter && !late {
            return Self::ParallelOnly { nworkers, reason };
        }
        let stats = if late {
            VectorSearchStats {
                matches: vectors,
                ..stats
            }
        } else {
            stats
        };
        let work = VectorWork::estimate(
            &stats,
            options.target_leaf_size() as f64 / f64::from(options.training_sample_ratio()),
            if layers.is_some() && options.vector_router_quantization() {
                options.replication()
            } else {
                1
            },
            gucs::vector_cluster_max_probe(),
            gucs::vector_fixed_probe_cost_rows(),
            row_bytes,
            full_row_bytes,
        );
        if work.parallel_wins(parallel_divisor(nworkers, leader_participates)) {
            Self::ParallelOnly { nworkers, reason }
        } else {
            Self::SerialOnly { reason }
        }
    }
}

struct VectorSearchStats {
    vectors: f64,
    matches: f64,
}

struct VectorWork {
    scan_bytes: f64,
    extra_routing_bytes: f64,
}

impl VectorWork {
    fn estimate(
        stats: &VectorSearchStats,
        average_cluster_size: f64,
        replication: u8,
        max_probe: usize,
        fixed_probe_cost_rows: f64,
        row_bytes: f64,
        full_row_bytes: f64,
    ) -> Self {
        let vectors = stats.vectors.max(1.0);
        let clusters = (vectors / average_cluster_size).max(1.0);
        let replication = f64::from(replication);
        let average = vectors * replication / clusters;
        let selectivity = (stats.matches / vectors).clamp(0.0, 1.0);
        let matches = vectors * selectivity;
        let budget = (max_probe as f64).min(clusters);
        // Match the probe controller's cluster-open share and maximum-budget accounting.
        let open_share = (fixed_probe_cost_rows / (fixed_probe_cost_rows + average)).min(0.5);
        let routed = (budget / (open_share + (1.0 - open_share) * selectivity)).min(clusters);
        let located = selectivity < 1.0 && matches <= budget * average / (1.0 - open_share);
        let (scored, opened) = if located {
            (matches, matches.min(clusters))
        } else {
            (
                (routed * average * selectivity).min(matches * replication),
                routed,
            )
        };
        let read_bytes = (opened * average * row_bytes).min(scored * full_row_bytes);
        let filter_bytes = if selectivity < 1.0 {
            matches * size_of::<tantivy::DocId>() as f64
        } else {
            0.0
        };
        Self {
            scan_bytes: read_bytes + scored * row_bytes + filter_bytes,
            // Located serial scans skip routing; parallel scans still read and score centroids.
            extra_routing_bytes: if located {
                2.0 * routed * full_row_bytes
            } else {
                0.0
            },
        }
    }

    fn parallel_wins(&self, participants: f64) -> bool {
        participants > 1.0
            && self.scan_bytes * (1.0 - 1.0 / participants) - self.extra_routing_bytes
                > MIN_PARALLEL_SAVINGS_BYTES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work(matches: f64, max_probe: usize, dimensions: usize, bits: usize) -> VectorWork {
        VectorWork::estimate(
            &VectorSearchStats {
                vectors: 8_000_000.0,
                matches,
            },
            512.0,
            2,
            max_probe,
            2.0,
            (dimensions * bits).div_ceil(u8::BITS as usize) as f64,
            (dimensions * size_of::<f32>()) as f64,
        )
    }

    #[test]
    fn larger_probe_budgets_cover_worker_startup() {
        assert!(!work(8_000_000.0, 32, 1024, 1).parallel_wins(8.0));
        assert!(work(8_000_000.0, 1024, 1024, 1).parallel_wins(8.0));
    }

    #[test]
    fn vector_width_changes_the_crossover() {
        assert!(!work(8_000_000.0, 128, 128, 1).parallel_wins(8.0));
        assert!(work(8_000_000.0, 128, 4096, 1).parallel_wins(8.0));
        assert!(!work(8_000_000.0, 128, 1024, 1).parallel_wins(8.0));
        assert!(work(8_000_000.0, 128, 1024, 4).parallel_wins(8.0));
        let full_precision_bits = size_of::<f32>() * u8::BITS as usize;
        assert!(work(8_000_000.0, 128, 1024, full_precision_bits).parallel_wins(8.0));
    }

    #[test]
    fn filtered_reads_can_cover_startup_at_smaller_probe_budgets() {
        assert!(!work(8_000_000.0, 128, 1024, 1).parallel_wins(8.0));
        assert!(work(800_000.0, 128, 1024, 1).parallel_wins(8.0));
    }

    #[test]
    fn tiny_match_sets_stay_serial_even_with_large_budgets() {
        for budget in [32, 1024, 65_536] {
            let work = work(100.0, budget, 1024, 1);
            assert!(work.extra_routing_bytes > work.scan_bytes);
            assert!(!work.parallel_wins(8.0));
        }
    }

    #[test]
    fn routing_can_erase_the_parallel_savings() {
        let work = VectorWork {
            scan_bytes: 4.0 * MIN_PARALLEL_SAVINGS_BYTES,
            extra_routing_bytes: 3.0 * MIN_PARALLEL_SAVINGS_BYTES,
        };
        assert!(!work.parallel_wins(8.0));
        assert!(
            VectorWork {
                extra_routing_bytes: 0.0,
                ..work
            }
            .parallel_wins(8.0)
        );
    }

    #[test]
    fn additional_workers_must_save_enough_work() {
        let work = VectorWork {
            scan_bytes: 2.0 * MIN_PARALLEL_SAVINGS_BYTES,
            extra_routing_bytes: 0.0,
        };
        for participants in [0.0, 1.0, 2.0] {
            assert!(!work.parallel_wins(participants));
        }
        assert!(work.parallel_wins(4.0));
    }

    #[test]
    fn replication_increases_scan_work_without_adding_clusters() {
        let stats = VectorSearchStats {
            vectors: 8_000_000.0,
            matches: 8_000_000.0,
        };
        let single = VectorWork::estimate(&stats, 512.0, 1, 128, 2.0, 128.0, 4096.0);
        let replicated = VectorWork::estimate(&stats, 512.0, 2, 128, 2.0, 128.0, 4096.0);
        assert_eq!(replicated.scan_bytes, 2.0 * single.scan_bytes);
        assert_eq!(replicated.extra_routing_bytes, single.extra_routing_bytes);
    }

    #[test]
    fn located_work_does_not_duplicate_matches() {
        let stats = VectorSearchStats {
            vectors: 8_000_000.0,
            matches: 100.0,
        };
        let single = VectorWork::estimate(&stats, 512.0, 1, 65_536, 2.0, 128.0, 4096.0);
        let replicated = VectorWork::estimate(&stats, 512.0, 2, 65_536, 2.0, 128.0, 4096.0);
        assert_eq!(replicated.scan_bytes, single.scan_bytes);
        assert_eq!(replicated.extra_routing_bytes, single.extra_routing_bytes);
    }

    #[test]
    fn budgets_are_capped_by_available_work() {
        for matches in [100.0, 800_000.0, 8_000_000.0] {
            let capped = work(matches, 65_536, 1024, 1);
            let unbounded = work(matches, usize::MAX, 1024, 1);
            assert_eq!(capped.scan_bytes, unbounded.scan_bytes);
            assert_eq!(capped.extra_routing_bytes, unbounded.extra_routing_bytes);
        }
        assert!(!work(0.0, usize::MAX, 1024, 1).parallel_wins(8.0));
    }
}

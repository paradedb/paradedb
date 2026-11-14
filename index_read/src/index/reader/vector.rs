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

use std::sync::{Arc, Mutex, OnceLock};

use pgrx::pg_sys;
use tantivy::query::Scorer;
use tantivy::vector::{
    AtomicVectorWork, Candidate, ClusterRef, LocatedPath, ProbeParams, SharedVectorWork,
    VectorRoutingStats, VectorSearchBudget,
};
use tantivy::{Result as TantivyResult, Score, SegmentReader};

use crate::api::HashMap;
use crate::gucs;
use crate::postgres::ParallelScanState;
use crate::postgres::customscan::mpp::{MainThread, run_on_main_thread};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct VectorRoutingResults {
    pub budget: VectorSearchBudget,
    pub candidates: HashMap<tantivy::SegmentOrdinal, Vec<Candidate<ClusterRef>>>,
    pub statistics: Vec<VectorRoutingStats>,
}

pub struct SharedVectorRouting {
    pub budget: VectorSearchBudget,
    pub results: OnceLock<tantivy::Result<VectorRoutingResults>>,
    pub work: OnceLock<Arc<dyn SharedVectorWork>>,
    pub shared: Option<MainThread<(*mut ParallelScanState, *mut pg_sys::dsa_area)>>,
}

impl From<VectorRoutingResults> for SharedVectorRouting {
    fn from(results: VectorRoutingResults) -> Self {
        Self {
            budget: results.budget,
            results: OnceLock::from(Ok(results)),
            work: OnceLock::new(),
            shared: None,
        }
    }
}

impl SharedVectorRouting {
    pub fn work(&self) -> Arc<dyn SharedVectorWork> {
        Arc::clone(
            self.work
                .get_or_init(|| Arc::new(AtomicVectorWork::default())),
        )
    }

    pub fn get(&self) -> tantivy::Result<&VectorRoutingResults> {
        self.results
            .get_or_init(|| {
                pg_sys::thread_check::check_active_thread();
                run_on_main_thread(|token| {
                    let (state, area) = *self
                        .shared
                        .as_ref()
                        .expect("pending routing requires shared state")
                        .get(token);
                    unsafe { (*state).poll_vector_routing_plan(area) }
                        .map_err(|err| tantivy::TantivyError::SystemError(err.to_string()))?
                        .ok_or_else(|| {
                            tantivy::TantivyError::InternalError(
                                "parallel vector scan requires routing results".into(),
                            )
                        })
                })
            })
            .as_ref()
            .map_err(Clone::clone)
    }
}

impl super::index::VectorRouting {
    pub(super) fn work_for_pull(&self) -> Option<Arc<dyn SharedVectorWork>> {
        match self {
            Self::Local => None,
            Self::Shared(plan, spent) => Some(Arc::new(VectorPullWork {
                shared: plan.work(),
                spent: Arc::clone(spent),
                credit: spent.charged(),
                replayed: Mutex::new(0.0),
            })),
        }
    }
}

// Expanding Top-K windows replay earlier probes. Only work beyond this stream's previous
// charge consumes more of the shared budget; other streams cannot spend its replay credit.
struct VectorPullWork {
    shared: Arc<dyn SharedVectorWork>,
    spent: Arc<AtomicVectorWork>,
    credit: f64,
    replayed: Mutex<f64>,
}

impl SharedVectorWork for VectorPullWork {
    fn charge(&self, work: f64) {
        let additional = if self.credit == 0.0 {
            work
        } else {
            let mut replayed = self.replayed.lock().unwrap();
            let before = (*replayed - self.credit).max(0.0);
            *replayed += work;
            (*replayed - self.credit).max(0.0) - before
        };
        if additional > 0.0 {
            self.shared.charge(additional);
            self.spent.charge(additional);
        }
    }

    fn charged(&self) -> f64 {
        if self.credit > 0.0 && *self.replayed.lock().unwrap() < self.credit {
            0.0
        } else {
            self.shared.charged()
        }
    }
}

struct PgSharedVectorWork(*const AtomicVectorWork);

// The scan owns the DSM mapping until every participant finishes; only atomics are accessed.
unsafe impl Send for PgSharedVectorWork {}
unsafe impl Sync for PgSharedVectorWork {}

impl SharedVectorWork for PgSharedVectorWork {
    fn charge(&self, work: f64) {
        unsafe { &*self.0 }.charge(work);
    }

    fn charged(&self) -> f64 {
        unsafe { &*self.0 }.charged()
    }
}

/// Creates a work counter over initialized scan DSM.
///
/// # Safety
/// The DSM must remain mapped until all returned handles are dropped.
pub unsafe fn shared_vector_work(
    state: *const ParallelScanState,
    source: usize,
) -> Arc<dyn SharedVectorWork> {
    assert!(!state.is_null());
    Arc::new(PgSharedVectorWork(unsafe { (*state).vector_work(source) }))
}

pub(super) fn probe_params() -> ProbeParams {
    let params = ProbeParams {
        adaptive_probing: gucs::vector_adaptive_probing(),
        max_probe: gucs::vector_cluster_max_probe(),
        fixed_probe_cost_rows: gucs::vector_fixed_probe_cost_rows(),
        router_rerank_factor: gucs::vector_router_rerank_factor(),
        router_min_beam_fraction: gucs::vector_router_min_beam_fraction(),
        router_aps: gucs::vector_router_aps(),
        ..Default::default()
    };
    // Router audits route every segment, so both arms rank the same segments.
    match gucs::vector_router_audit() {
        gucs::VectorRouterAudit::Off => params,
        gucs::VectorRouterAudit::Stored => ProbeParams {
            located: LocatedPath::Off,
            ..params
        },
        gucs::VectorRouterAudit::Flat => ProbeParams {
            located: LocatedPath::Off,
            router_flat: true,
            ..params
        },
    }
}

pub(super) struct VectorFilterWeight {
    pub base: std::sync::Arc<super::scorer::LazyWeight>,
    pub constrained: Option<std::sync::Arc<super::scorer::LazyWeight>>,
    pub visible_docs: HashMap<tantivy::index::SegmentId, tantivy_common::BitSet>,
    pub constrained_segments: crate::api::HashSet<tantivy::index::SegmentId>,
}

impl VectorFilterWeight {
    fn get(&self, reader: &tantivy::SegmentReader) -> &dyn tantivy::query::Weight {
        if self.constrained_segments.contains(&reader.segment_id()) {
            self.constrained
                .as_ref()
                .expect("range-constrained segment requires bounds")
                .get()
        } else {
            self.base.get()
        }
    }
}

impl tantivy::query::Weight for VectorFilterWeight {
    fn scorer(
        &self,
        reader: &tantivy::SegmentReader,
        boost: tantivy::Score,
    ) -> tantivy::Result<Box<dyn tantivy::query::Scorer>> {
        if let Some(docs) = self.visible_docs.get(&reader.segment_id()) {
            Ok(Box::new(tantivy::query::ConstScorer::new(
                tantivy::query::BitSetDocSet::from(docs.clone()),
                boost,
            )))
        } else {
            self.get(reader).scorer(reader, boost)
        }
    }
    fn scorer_for_candidates_unpositioned(
        &self,
        reader: &SegmentReader,
        boost: Score,
        candidate_size: u32,
    ) -> TantivyResult<Box<dyn Scorer>> {
        if self.visible_docs.contains_key(&reader.segment_id()) {
            self.scorer(reader, boost)
        } else {
            self.get(reader)
                .scorer_for_candidates_unpositioned(reader, boost, candidate_size)
        }
    }

    fn explain(
        &self,
        reader: &tantivy::SegmentReader,
        doc: tantivy::DocId,
    ) -> tantivy::Result<tantivy::query::Explanation> {
        self.get(reader).explain(reader, doc)
    }
}

#[cfg(test)]
mod work_tests {
    use super::*;

    #[test]
    fn vector_work_retries_replay_only_their_own_charges() {
        let shared: Arc<dyn SharedVectorWork> = Arc::new(AtomicVectorWork::default());
        let spent = Arc::new(AtomicVectorWork::default());
        let pull = || VectorPullWork {
            shared: Arc::clone(&shared),
            spent: Arc::clone(&spent),
            credit: spent.charged(),
            replayed: Mutex::new(0.0),
        };
        let first = pull();
        first.charge(10.0);
        shared.charge(5.0);
        assert_eq!(first.charged(), 15.0);
        let retry = pull();
        assert_eq!(retry.charged(), 0.0);
        retry.charge(6.0);
        assert_eq!(retry.charged(), 0.0);
        assert_eq!(shared.charged(), 15.0);
        retry.charge(6.0);
        assert_eq!(retry.charged(), 17.0);
        assert_eq!(shared.charged(), 17.0);
        assert_eq!(spent.charged(), 12.0);
        shared.charge(100.0);
        let third = pull();
        assert_eq!(third.charged(), 0.0);
        third.charge(12.0);
        assert_eq!(third.charged(), 117.0);
        assert_eq!(shared.charged(), 117.0);
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;
    use serde_json::Value;

    /// Per-segment vector statistics of every ParadeDB scan in a JSON plan.
    fn segment_stats(plan: &Value) -> Vec<Value> {
        let mut nodes = vec![&plan[0]["Plan"]];
        let mut stats = Vec::new();
        while let Some(node) = nodes.pop() {
            if let Some(info) = node["Segment Info"].as_str() {
                let info: serde_json::Map<String, Value> = serde_json::from_str(info).unwrap();
                stats.extend(info.into_iter().map(|(_, value)| value));
            }
            if let Some(children) = node["Plans"].as_array() {
                nodes.extend(children);
            }
        }
        stats
    }

    /// Both audit arms route every segment; the flat arm ranks every centroid exactly and, at a
    /// budget that opens every cluster, returns the stored arm's rows.
    #[pg_test]
    fn router_audit_routes_every_segment_and_flat_ranks_every_centroid() {
        Spi::run(
            "SET max_parallel_workers_per_gather = 0;
             SET paradedb.vector_clustering_threshold = 1;
             SET paradedb.global_mutable_segment_rows = 0;
             SET paradedb.vector_stats = on;
             SET paradedb.vector_adaptive_probing = off;
             SET paradedb.vector_cluster_max_probe = 100000;
             CREATE TABLE router_audit (id bigint PRIMARY KEY, body text, embedding vector(3));
             CREATE INDEX router_audit_idx ON router_audit
             USING paradedb (id, body, embedding vector_l2_ops)
             WITH (target_segment_count = 2, background_layer_sizes = '0');
             INSERT INTO router_audit
             SELECT g, CASE WHEN g % 4 = 0 THEN 'kept' ELSE 'dropped' END,
                    ARRAY[cos(g)::real, sin(g)::real, (g % 11)::real]::vector
             FROM generate_series(1, 400) g;
             INSERT INTO router_audit
             SELECT g, CASE WHEN g % 4 = 0 THEN 'kept' ELSE 'dropped' END,
                    ARRAY[cos(g)::real, sin(g)::real, (g % 11)::real]::vector
             FROM generate_series(401, 800) g;
             REINDEX INDEX router_audit_idx;",
        )
        .unwrap();
        let query = "SELECT id FROM router_audit WHERE body ||| 'kept' \
                     ORDER BY embedding <-> '[0.5, 0.5, 3]' LIMIT 10";
        let run = |audit: &str| {
            Spi::run(&format!("SET paradedb.vector_router_audit = {audit}")).unwrap();
            let plan = Spi::get_one::<pgrx::Json>(&format!(
                "EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON) {query}"
            ))
            .unwrap()
            .unwrap()
            .0;
            let ids = Spi::get_one::<Vec<i64>>(&format!("SELECT array_agg(id) FROM ({query}) q"))
                .unwrap()
                .unwrap();
            (segment_stats(&plan), ids)
        };

        let (off, _) = run("off");
        assert!(off.iter().any(|s| s["access_path"] == "Located"), "{off:?}");
        let (stored, stored_ids) = run("stored");
        let (flat, flat_ids) = run("flat");
        assert!(stored.len() >= 2, "{stored:?}");
        assert_eq!(flat.len(), stored.len());
        for (segment, mode) in stored
            .iter()
            .map(|s| (s, "hierarchical"))
            .chain(flat.iter().map(|s| (s, "flat")))
        {
            assert_eq!(segment["access_path"], "Routed", "{segment}");
            assert_eq!(segment["routing_mode"], mode, "{segment}");
            let opened = segment["postings_row"].as_u64().unwrap()
                + segment["postings_skipped"].as_u64().unwrap();
            assert_eq!(
                opened,
                segment["segment_clusters"].as_u64().unwrap(),
                "{segment}"
            );
        }
        for segment in &flat {
            assert_eq!(segment["routing_kind"], "exact", "{segment}");
            assert_eq!(
                segment["routing_exact_scored"], segment["segment_clusters"],
                "{segment}"
            );
        }
        assert_eq!(stored_ids.len(), 10);
        assert_eq!(flat_ids, stored_ids);
    }
}

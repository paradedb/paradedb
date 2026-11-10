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
use tantivy::vector::{
    AtomicVectorWork, Candidate, ClusterRef, ProbeParams, SharedVectorWork, VectorRoutingStats,
    VectorSearchBudget,
};

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
    pub(crate) work: OnceLock<Arc<dyn SharedVectorWork>>,
    pub(crate) shared: Option<MainThread<(*mut ParallelScanState, *mut pg_sys::dsa_area)>>,
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
pub(crate) unsafe fn shared_vector_work(
    state: *const ParallelScanState,
    source: usize,
) -> Arc<dyn SharedVectorWork> {
    assert!(!state.is_null());
    Arc::new(PgSharedVectorWork(unsafe { (*state).vector_work(source) }))
}

pub(super) fn probe_params() -> ProbeParams {
    ProbeParams {
        adaptive_probing: gucs::vector_adaptive_probing(),
        max_probe: gucs::vector_cluster_max_probe(),
        fixed_probe_cost_rows: gucs::vector_fixed_probe_cost_rows(),
        router_rerank_factor: gucs::vector_router_rerank_factor(),
        router_min_beam_fraction: gucs::vector_router_min_beam_fraction(),
        router_aps: gucs::vector_router_aps(),
        ..Default::default()
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

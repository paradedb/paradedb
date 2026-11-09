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

use std::sync::OnceLock;

use pgrx::pg_sys;
use tantivy::vector::{
    Candidate, ClusterRef, ProbeParams, VectorRoutingBudget, VectorRoutingStats, VectorSearchBudget,
};

use crate::api::HashMap;
use crate::gucs;
use crate::postgres::ParallelScanState;
use crate::postgres::customscan::mpp::{MainThread, run_on_main_thread};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct VectorRoutingResults {
    pub budget: VectorSearchBudget,
    pub candidates: HashMap<tantivy::SegmentOrdinal, Vec<Candidate<ClusterRef>>>,
    pub allocation: VectorRoutingBudget,
    pub statistics: Vec<VectorRoutingStats>,
}

pub struct SharedVectorRouting {
    pub budget: VectorSearchBudget,
    pub results: OnceLock<tantivy::Result<VectorRoutingResults>>,
    pub(crate) shared: Option<MainThread<(*mut ParallelScanState, *mut pg_sys::dsa_area)>>,
}

impl From<VectorRoutingResults> for SharedVectorRouting {
    fn from(results: VectorRoutingResults) -> Self {
        Self {
            budget: results.budget,
            results: OnceLock::from(Ok(results)),
            shared: None,
        }
    }
}

impl SharedVectorRouting {
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

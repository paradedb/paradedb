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

//! Builds the collector and visibility checker for executing an aggregation
//! request.
//!
//! When every aggregation in a request is a cardinality over a string field,
//! MVCC is solved inside the aggregation: tantivy consults the visibility
//! filter only for docs whose value is not yet confirmed visible, instead of
//! vischecking every matched doc up front.

use parking_lot::Mutex;
use std::sync::Arc;

use crate::index::fast_fields_helper::FFType;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use pgrx::pg_sys;
use tantivy::aggregation::agg_req::{AggregationVariants, Aggregations};
use tantivy::aggregation::{
    AggContextParams, AggregationLimitsGuard, DistributedAggregationCollector,
    DocVisibilityFilterFactory,
};

pub trait AggregationExec {
    /// Builds the collector for executing this request, along with the
    /// visibility checker the caller must wrap it with. `None` means there is
    /// nothing left to wrap: either `solve_mvcc` is off, or the request is
    /// cardinality-only over string fields, in which case MVCC is solved
    /// inside the aggregation with a per-value visibility filter.
    fn plan(
        &self,
        reader: &SearchIndexReader,
        heaprel: &PgSearchRelation,
        solve_mvcc: bool,
        limits: AggregationLimitsGuard,
    ) -> (DistributedAggregationCollector, Option<VisibilityChecker>);
}

impl AggregationExec for Aggregations {
    #[allow(clippy::arc_with_non_send_sync)]
    fn plan(
        &self,
        reader: &SearchIndexReader,
        heaprel: &PgSearchRelation,
        solve_mvcc: bool,
        limits: AggregationLimitsGuard,
    ) -> (DistributedAggregationCollector, Option<VisibilityChecker>) {
        let use_cardinality_fast_path = solve_mvcc
            && !self.is_empty()
            && self.values().all(|agg| {
                agg.sub_aggregation.is_empty()
                    && match &agg.agg {
                        AggregationVariants::Cardinality(card) => reader
                            .schema()
                            .search_field(&card.field)
                            .is_some_and(|field| field.is_text()),
                        _ => false,
                    }
            });
        let tokenizers = reader.searcher().index().tokenizers().clone();
        let mut params = AggContextParams::new(limits, tokenizers);
        if use_cardinality_fast_path {
            let vischeck = SendSyncWrapper(Arc::new(Mutex::new(
                VisibilityChecker::with_rel_and_snap(heaprel, unsafe {
                    pg_sys::GetActiveSnapshot()
                }),
            )));
            let factory: DocVisibilityFilterFactory = Arc::new(move |segment_reader| {
                let ctid_ff = FFType::new_ctid(segment_reader.fast_fields());
                let vischeck = vischeck.get().clone();
                Some(Box::new(move |doc| {
                    let Some(ctid) = ctid_ff.as_u64(doc) else {
                        return false;
                    };
                    vischeck.lock().check_one(ctid)
                }))
            });
            params = params.with_doc_visibility_factory(factory);
        }
        let collector = DistributedAggregationCollector::from_aggs(self.clone(), params);
        let vischeck = (solve_mvcc && !use_cardinality_fast_path).then(|| {
            VisibilityChecker::with_rel_and_snap(heaprel, unsafe { pg_sys::GetActiveSnapshot() })
        });
        (collector, vischeck)
    }
}

struct SendSyncWrapper<T>(T);
// SAFETY: collection runs single-threaded within this backend/parallel-worker
// process.
unsafe impl<T> Send for SendSyncWrapper<T> {}
unsafe impl<T> Sync for SendSyncWrapper<T> {}

impl<T> SendSyncWrapper<T> {
    fn get(&self) -> &T {
        &self.0
    }
}

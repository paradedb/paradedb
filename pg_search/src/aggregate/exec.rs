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
use crate::schema::SearchIndexSchema;
use pgrx::pg_sys;
use tantivy::aggregation::agg_req::{AggregationVariants, Aggregations};
use tantivy::aggregation::{
    AggContextParams, AggregationLimitsGuard, DistributedAggregationCollector,
    DocVisibilityFilterFactory,
};

pub trait AggregationExec {
    /// True when `solve_mvcc` is requested and every aggregation in the
    /// request is a cardinality over a string field, with no
    /// sub-aggregations
    fn use_cardinality_fast_path(&self, schema: &SearchIndexSchema, solve_mvcc: bool) -> bool;

    /// Builds the collector for executing this request. When `solve_mvcc` is
    /// set and the request is cardinality-only over string fields, MVCC is
    /// solved inside the aggregation with a per-value visibility filter;
    /// otherwise the caller is responsible for visibility filtering if it
    /// requested `solve_mvcc` (see [`visibility_checker`]).
    ///
    /// [`visibility_checker`]: AggregationExec::visibility_checker
    fn collector(
        &self,
        reader: &SearchIndexReader,
        heaprel: &PgSearchRelation,
        solve_mvcc: bool,
        limits: AggregationLimitsGuard,
    ) -> DistributedAggregationCollector;

    /// The visibility checker the caller must apply when `solve_mvcc` is set
    /// but [`collector`] does not solve MVCC inside the aggregation.
    ///
    /// [`collector`]: AggregationExec::collector
    fn visibility_checker(
        &self,
        reader: &SearchIndexReader,
        heaprel: &PgSearchRelation,
        solve_mvcc: bool,
    ) -> Option<VisibilityChecker>;
}

impl AggregationExec for Aggregations {
    fn use_cardinality_fast_path(&self, schema: &SearchIndexSchema, solve_mvcc: bool) -> bool {
        solve_mvcc
            && !self.is_empty()
            && self.values().all(|agg| {
                agg.sub_aggregation.is_empty()
                    && match &agg.agg {
                        AggregationVariants::Cardinality(card) => schema
                            .search_field(&card.field)
                            .is_some_and(|field| field.is_text()),
                        _ => false,
                    }
            })
    }

    #[allow(clippy::arc_with_non_send_sync)]
    fn collector(
        &self,
        reader: &SearchIndexReader,
        heaprel: &PgSearchRelation,
        solve_mvcc: bool,
        limits: AggregationLimitsGuard,
    ) -> DistributedAggregationCollector {
        let tokenizers = reader.searcher().index().tokenizers().clone();
        let mut params = AggContextParams::new(limits, tokenizers);
        if self.use_cardinality_fast_path(reader.schema(), solve_mvcc) {
            let vischeck = SendSyncWrapper(Arc::new(Mutex::new(
                VisibilityChecker::with_rel_and_snap(heaprel, unsafe {
                    pg_sys::GetActiveSnapshot()
                }),
            )));
            let factory: DocVisibilityFilterFactory = Arc::new(move |segment_reader| {
                let ctid_ff = FFType::new(segment_reader.fast_fields(), "ctid");
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
        DistributedAggregationCollector::from_aggs(self.clone(), params)
    }

    fn visibility_checker(
        &self,
        reader: &SearchIndexReader,
        heaprel: &PgSearchRelation,
        solve_mvcc: bool,
    ) -> Option<VisibilityChecker> {
        (solve_mvcc && !self.use_cardinality_fast_path(reader.schema(), solve_mvcc)).then(|| {
            VisibilityChecker::with_rel_and_snap(heaprel, unsafe { pg_sys::GetActiveSnapshot() })
        })
    }
}

struct SendSyncWrapper<T>(T);
// SAFETY: same rationale as MVCCFilterCollector — collection runs
// single-threaded within this backend/parallel-worker process.
unsafe impl<T> Send for SendSyncWrapper<T> {}
unsafe impl<T> Sync for SendSyncWrapper<T> {}

impl<T> SendSyncWrapper<T> {
    // Method (not field) access so closures capture the whole wrapper,
    // keeping the unsafe Send/Sync impls effective under edition-2021
    // disjoint capture.
    fn get(&self) -> &T {
        &self.0
    }
}

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

//! MVCC-aware execution of cardinality-only aggregation requests.
//!
//! When every aggregation in a request is a cardinality over a string field,
//! tantivy consults the visibility filter only for docs whose value is not yet
//! confirmed visible, instead of vischecking every matched doc up front.

use parking_lot::Mutex;
use std::sync::Arc;

use crate::aggregate::interrupt_collector::InterruptableCollector;
use crate::index::fast_fields_helper::FFType;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::schema::SearchIndexSchema;
use pgrx::pg_sys;
use tantivy::aggregation::agg_req::{AggregationVariants, Aggregations};
use tantivy::aggregation::intermediate_agg_result::IntermediateAggregationResults;
use tantivy::aggregation::{
    AggContextParams, AggregationLimitsGuard, DistributedAggregationCollector,
    DocVisibilityFilterFactory,
};
use tantivy::tokenizer::TokenizerManager;

/// Builds an aggregation context that solves MVCC through tantivy's per-value
/// visibility filter, if the request qualifies: every aggregation must be a
/// cardinality over a string field, with no sub-aggregations. Tantivy
/// validates the same conditions per segment and errors on violations, so the
/// check here decides routing only.
///
/// Returns `None` when the request doesn't qualify; the caller must then
/// solve MVCC itself (e.g. with `MVCCFilterCollector`).
pub fn mvcc_agg_context(
    aggregations: &Aggregations,
    schema: &SearchIndexSchema,
    heaprel: &PgSearchRelation,
    limits: AggregationLimitsGuard,
    tokenizers: TokenizerManager,
) -> Option<AggContextParams> {
    let eligible = !aggregations.is_empty()
        && aggregations.values().all(|agg| {
            agg.sub_aggregation.is_empty()
                && match &agg.agg {
                    AggregationVariants::Cardinality(card) => schema
                        .search_field(&card.field)
                        .is_some_and(|field| field.is_text()),
                    _ => false,
                }
        });
    eligible.then(|| {
        AggContextParams::new(limits, tokenizers).with_doc_visibility_factory(
            doc_visibility_factory(heaprel, unsafe { pg_sys::GetActiveSnapshot() }),
        )
    })
}

/// Executes a cardinality-only aggregation request with MVCC enabled.
///
/// Returns `None` when the request doesn't qualify (see [`mvcc_agg_context`]);
/// the caller must then solve MVCC itself.
pub fn execute_with_mvcc(
    reader: &SearchIndexReader,
    aggregations: &Aggregations,
    schema: &SearchIndexSchema,
    heaprel: &PgSearchRelation,
    limits: AggregationLimitsGuard,
    tokenizers: TokenizerManager,
) -> Option<IntermediateAggregationResults> {
    let context = mvcc_agg_context(aggregations, schema, heaprel, limits, tokenizers)?;
    let collector = DistributedAggregationCollector::from_aggs(aggregations.clone(), context);
    Some(reader.collect(InterruptableCollector::new(collector)))
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

#[allow(clippy::arc_with_non_send_sync)]
fn doc_visibility_factory(
    heaprel: &PgSearchRelation,
    snapshot: pg_sys::Snapshot,
) -> DocVisibilityFilterFactory {
    let vischeck = SendSyncWrapper(Arc::new(Mutex::new(VisibilityChecker::with_rel_and_snap(
        heaprel, snapshot,
    ))));
    Arc::new(move |segment_reader| {
        let ctid_ff = FFType::new(segment_reader.fast_fields(), "ctid");
        let vischeck = vischeck.get().clone();
        Some(Box::new(move |doc| {
            let Some(ctid) = ctid_ff.as_u64(doc) else {
                return false;
            };
            vischeck.lock().check_one(ctid)
        }))
    })
}

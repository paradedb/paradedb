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
#![allow(clippy::missing_safety_doc, unsafe_op_in_unsafe_fn)]

pub use common::api::operator::*;

use crate::api::{CTID_FIELD_NAME, FieldName};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::{DocsEstimate, SearchIndexReader};
use crate::postgres::catalog::lookup_reltuples;
use crate::postgres::index::{
    free_leaf_partition_indexes, is_partitioned_index, leaf_partition_index_oids,
};
use crate::postgres::rel::PgSearchRelation;
use crate::query::SearchQueryInput;
use crate::scan::info::RowEstimate;
use pgrx::pg_sys;
use tantivy::vector::VectorFilterStrategy;

/// #4172: for queries whose scorer is expensive to build (fuzzy, regex, range),
/// estimate heuristically rather than opening the index. Both selectivity and the
/// TopK worker decision honor this so neither pays full scorer construction at
/// plan time.
fn heuristic_selectivity(search_query_input: &SearchQueryInput) -> Option<f64> {
    crate::gucs::enable_heuristic_selectivity()
        .then(|| search_query_input.expensive_selectivity_heuristic())
        .flatten()
}

/// Reuse the supplied reader, or open one via [`SearchIndexReader::mvcc_for_estimation`],
/// to estimate matching docs, total docs, Tantivy `DocSet::cost()`, and the candidate
/// segment count in one pass. The single source for `estimate_selectivity`, planner
/// traversal estimates, and segment count estimates.
///
/// Returns `None` if the reader can't be opened (e.g. a transient/concurrent-DDL
/// failure); callers degrade gracefully rather than crash planning. The same open
/// would also fail in the executor, so `None` here doesn't mask a real problem.
fn open_and_estimate_docs(
    indexrel: &PgSearchRelation,
    search_query_input: SearchQueryInput,
    reader: Option<&SearchIndexReader>,
    vector_field: Option<&FieldName>,
) -> Option<(DocsEstimate, usize, VectorFilterStrategy)> {
    // A partitioned index has no storage of its own (#4643). A predicate above an Append
    // covers every partition, so one leaf stands in for the tree: its match ratio is the
    // selectivity, and its counts scale up to the rows of every leaf. The largest leaf is
    // tried first, since the first in OID order is often a default or near-empty partition
    // whose rows say nothing about the rest. A leaf a `REINDEX` holds is skipped instead
    // of waited on, and one that holds no rows yields to the next.
    if is_partitioned_index(indexrel.oid()) {
        let mut leaves: Vec<_> = leaf_partition_index_oids(indexrel)
            .into_iter()
            .map(|oid| (oid, leaf_heap_rows(oid)))
            .collect();
        leaves.sort_by(|a, b| b.1.unwrap_or(-1.0).total_cmp(&a.1.unwrap_or(-1.0)));
        let ordered: Vec<pg_sys::Oid> = leaves.iter().map(|(oid, _)| *oid).collect();
        let (sample_oid, estimate, segment_count, strategy) = free_leaf_partition_indexes(&ordered)
            .find_map(|leaf| {
                open_and_estimate_docs(&leaf, search_query_input.clone(), None, vector_field)
                    .filter(|(estimate, _, _)| estimate.total_docs > 0)
                    .map(|(estimate, segment_count, strategy)| {
                        (leaf.oid(), estimate, segment_count, strategy)
                    })
            })?;
        // A leaf that has never been counted is taken to be the sample's size.
        let sample_rows = estimate.total_docs as f64;
        let total_rows = leaves
            .iter()
            .filter(|(oid, _)| *oid != sample_oid)
            .map(|(_, rows)| rows.unwrap_or(sample_rows))
            .sum::<f64>()
            + sample_rows;
        let scale = total_rows / sample_rows;
        return Some((
            DocsEstimate {
                matching_docs: (estimate.matching_docs as f64 * scale).ceil() as usize,
                total_docs: total_rows.ceil() as u64,
                query_cost: (estimate.query_cost as f64 * scale).ceil() as u64,
            },
            segment_count,
            strategy,
        ));
    }

    let heap_rel = indexrel
        .heap_relation()
        .expect("indexrel should be an index");
    let row_estimate = RowEstimate::from_reltuples(heap_rel.reltuples().map(|r| r as f64));

    let opened;
    let reader = match reader {
        Some(reader) => reader,
        None => {
            let mvcc_style = if vector_field.is_some() {
                MvccSatisfies::Snapshot
            } else {
                SearchIndexReader::mvcc_for_estimation(indexrel, &search_query_input)
            };
            opened =
                SearchIndexReader::open(indexrel, search_query_input, false, mvcc_style).ok()?;
            &opened
        }
    };
    let estimate = reader.estimate_docs(row_estimate);
    let strategy = match vector_field {
        Some(field) => reader.plan_vector_filter(field).ok()?,
        None => VectorFilterStrategy::default(),
    };
    Some((estimate, reader.estimated_segment_count(), strategy))
}

/// The planner's row count for the partition a leaf index is attached to, `None` when
/// that partition has never been counted.
fn leaf_heap_rows(index_oid: pg_sys::Oid) -> Option<f64> {
    let heap_oid = unsafe { pg_sys::IndexGetRelation(index_oid, true) };
    (heap_oid != pg_sys::InvalidOid)
        .then(|| lookup_reltuples(heap_oid))
        .flatten()
}

pub struct SelectivityAndCost {
    pub selectivity: Option<f64>,
    pub query_cost: Option<u64>,
    pub segment_count: Option<usize>,
    pub vector_filter_strategy: VectorFilterStrategy,
}

/// One index open, all planning answers: selectivity (matching/total docs), the
/// query's Tantivy `DocSet::cost()`, and the estimated (pruned) segment count.
/// basescan path generation needs all three for the same combined query, so it opens
/// once here instead of calling `estimate_selectivity` and a separate traversal/segment
/// estimate back-to-back.
///
/// Returns [`SelectivityAndCost`]:
/// - expensive-to-estimate query (#4172): `(selectivity_heuristic, scaled match estimate, None)`
///   -- no open; the cost is the heuristic match estimate scaled by
///   `EXPENSIVE_QUERY_COST_FACTOR` (`None` only when the row count is unknown);
/// - empty index / no valid total: `(None, Some(0), Some(segment_count))` -- the cost is a known 0;
/// - open failure: `(None, None, None)`.
pub fn estimate_selectivity_and_cost(
    indexrel: &PgSearchRelation,
    search_query_input: SearchQueryInput,
    reader: Option<&SearchIndexReader>,
    vector_field: Option<&FieldName>,
) -> SelectivityAndCost {
    if let Some(selectivity) = heuristic_selectivity(&search_query_input) {
        // #4172 skips opening the index, so derive the work estimate from the same
        // heuristic: these shapes drive far more docset work than they match, so scale the
        // heuristic match estimate (selectivity x rows) by EXPENSIVE_QUERY_COST_FACTOR.
        let cost = indexrel
            .heap_relation()
            .and_then(|heap| heap.reltuples())
            .map(|reltuples| {
                (selectivity * reltuples as f64 * crate::gucs::expensive_query_cost_factor()) as u64
            });
        return SelectivityAndCost {
            selectivity: Some(selectivity),
            query_cost: cost,
            segment_count: None,
            vector_filter_strategy: VectorFilterStrategy::default(),
        };
    }

    let Some((estimate, segment_count, vector_filter_strategy)) =
        open_and_estimate_docs(indexrel, search_query_input, reader, vector_field)
    else {
        return SelectivityAndCost {
            selectivity: None,
            query_cost: None,
            segment_count: None,
            vector_filter_strategy: VectorFilterStrategy::default(),
        };
    };

    let total_rows = estimate.total_docs as f64;
    let selectivity =
        (total_rows > 0.0).then(|| (estimate.matching_docs as f64 / total_rows).min(1.0));
    SelectivityAndCost {
        selectivity,
        query_cost: Some(estimate.query_cost),
        segment_count: Some(segment_count),
        vector_filter_strategy,
    }
}

pub fn estimate_selectivity(
    indexrel: &PgSearchRelation,
    search_query_input: SearchQueryInput,
) -> Option<f64> {
    estimate_selectivity_and_cost(indexrel, search_query_input, None, None).selectivity
}

/// The estimated number of heap rows matching `search_query_input`, scaled from the
/// largest candidate segment up to the candidate portion of the relation. Unlike
/// `estimate_selectivity` this is an absolute row count, which is what
/// `visibility => 'threshold'` compares against `paradedb.visibility_threshold`.
///
/// `None` when there is nothing to estimate from: an index that can't be opened, or an
/// expensive-to-estimate query (#4172) over a heap with no `reltuples`.
pub fn estimate_matching_rows(
    indexrel: &PgSearchRelation,
    search_query_input: SearchQueryInput,
    reader: Option<&SearchIndexReader>,
) -> Option<u64> {
    if let Some(selectivity) = heuristic_selectivity(&search_query_input) {
        return indexrel
            .heap_relation()
            .and_then(|heap| heap.reltuples())
            .map(|reltuples| (selectivity * reltuples as f64) as u64);
    }

    open_and_estimate_docs(indexrel, search_query_input, reader, None)
        .map(|(estimate, _, _)| estimate.matching_docs as u64)
}

/// Estimate the query's traversal cost using the shared selectivity/cost estimator.
/// Returns `None` if the index cannot be opened, or an expensive query has no row estimate.
pub fn estimate_query_cost(
    indexrel: &PgSearchRelation,
    search_query_input: SearchQueryInput,
) -> Option<u64> {
    estimate_selectivity_and_cost(indexrel, search_query_input, None, None).query_cost
}

/// Given a [`pg_sys::PlannerInfo`] and a [`pg_sys::Var`] from it, figure out the name of the `Var`
///
/// Returns the heap relation [`pg_sys::Oid`] that contains the `Var` along with its name.
pub unsafe fn attname_from_var(
    heaprel: &PgSearchRelation,
    var: *mut pg_sys::Var,
) -> Option<FieldName> {
    if (*var).varattno == 0 {
        return None;
    }
    let tupdesc = heaprel.tuple_desc();

    if (*var).varattno == pg_sys::SelfItemPointerAttributeNumber as pg_sys::AttrNumber {
        Some(CTID_FIELD_NAME.into())
    } else {
        tupdesc
            .get((*var).varattno as usize - 1)
            .map(|attribute| attribute.name().into())
    }
}

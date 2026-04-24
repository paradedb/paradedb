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

//! Exact-distance rerank for vector ORDER BY.
//!
//! The TurboQuant collector produces an approximate top-K keyed on
//! quantized distance estimates, which have a recall ceiling set by
//! the 4-bit encoding. This module lets callers over-fetch K' > K
//! quantized candidates, reload each candidate's raw f32 vector from
//! the Postgres heap, recompute the exact metric score, and return the
//! top-K ordered by that exact score. Activated when
//! `paradedb.vector_rerank_multiplier > 1.0`.
//!
//! Heap fetch, attno lookup, and vector normalization mirror the code
//! in `crate::vector::sampler` (k-means training sampler) — we reuse
//! the same proven pattern.

use std::cmp::Ordering;

use pgrx::{pg_sys, FromDatum};
use tantivy::{DocAddress, SegmentOrdinal};

use crate::api::HashMap;
use crate::index::fast_fields_helper::{resolve_ctid, FFType};
use crate::index::reader::index::SearchIndexScore;
use crate::postgres::catalog::is_pgvector_oid;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::u64_to_item_pointer;
use crate::vector::metric::{l2_normalize_in_place, VectorMetric};
use crate::vector::PgVector;

/// Build a map from vector-column name to Postgres `AttrNumber` for the
/// given heap relation. Mirrors the walk in `postgres::rel` used by the
/// sampler factory. Returns an empty map if the heap has no pgvector
/// columns (in which case rerank is a no-op — the caller skips it).
pub(crate) fn heap_attno_by_vector_name(
    heaprel: &PgSearchRelation,
) -> HashMap<String, pg_sys::AttrNumber> {
    let tupdesc = heaprel.tuple_desc();
    (0..tupdesc.len())
        .filter_map(|i| {
            let attr = tupdesc.get(i)?;
            is_pgvector_oid(attr.type_oid().value())
                .then(|| (attr.name().to_string(), (i + 1) as pg_sys::AttrNumber))
        })
        .collect()
}

/// Rerank quantized candidates by exact distance, keeping the top
/// `final_n` by metric-appropriate score.
///
/// For `L2` and `Cosine`, both the insert path and the query path
/// L2-normalize vectors (see `requires_unit_norm`), so ranking by
/// descending dot product is equivalent to ranking by ascending cosine
/// or L2 distance — no separate distance formula needed. For
/// `InnerProduct`, the dot product on raw (non-normalized) vectors is
/// exactly the metric value, and we want it to be large (pgvector's
/// `<#>` operator negates, but both tantivy and pg_search carry
/// "higher score = closer" internally; see `SortByTurboQuantDistance`).
///
/// Candidates whose heap tuple is no longer visible or whose vector
/// column is NULL are dropped silently — the result may contain fewer
/// than `final_n` rows. This matches the behaviour of the sampler on
/// truncated blocks.
pub(crate) fn rerank_by_heap_vectors(
    candidates: Vec<(SearchIndexScore, DocAddress)>,
    searcher: &tantivy::Searcher,
    heaprel: &PgSearchRelation,
    heap_attno: pg_sys::AttrNumber,
    query_vec: &[f32],
    metric: VectorMetric,
    final_n: usize,
) -> Vec<(SearchIndexScore, DocAddress)> {
    if candidates.is_empty() || final_n == 0 {
        return Vec::new();
    }

    let mut ctid_cache: Option<(SegmentOrdinal, FFType)> = None;
    // (exact_score, (original_approx_score, doc_address)). We keep the
    // original SearchIndexScore so downstream callers that expect a
    // bm25-shaped score (e.g. projection) still get a plausible value.
    let mut scored: Vec<(f32, (SearchIndexScore, DocAddress))> =
        Vec::with_capacity(candidates.len());

    unsafe {
        let snapshot = pg_sys::GetActiveSnapshot();
        let slot = pg_sys::MakeSingleTupleTableSlot(
            heaprel.tuple_desc().as_ptr(),
            &pg_sys::TTSOpsBufferHeapTuple,
        );

        for (approx_score, doc_address) in candidates {
            let ctid = resolve_ctid(&mut ctid_cache, searcher, doc_address);
            let mut tid = pg_sys::ItemPointerData::default();
            u64_to_item_pointer(ctid, &mut tid);

            let found =
                pg_sys::table_tuple_fetch_row_version(heaprel.as_ptr(), &mut tid, snapshot, slot);
            if !found {
                continue;
            }

            let mut is_null = false;
            let datum = pg_sys::slot_getattr(slot, heap_attno as i32, &mut is_null);
            let Some(PgVector(mut floats)) = PgVector::from_datum(datum, is_null) else {
                continue;
            };
            if floats.len() != query_vec.len() {
                // defensive: mismatched dims (shouldn't happen if the
                // index is consistent with the heap column)
                continue;
            }
            if metric.requires_unit_norm() {
                l2_normalize_in_place(&mut floats);
            }

            let exact = dot(query_vec, &floats);
            scored.push((exact, (approx_score, doc_address)));
        }

        pg_sys::ExecDropSingleTupleTableSlot(slot);
    }

    // Descending by exact score: for unit-norm L2/Cosine higher dot =
    // smaller distance; for IP higher dot = larger inner product =
    // closer per the `<#>` convention used in the rest of pg_search.
    scored.sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
    scored.truncate(final_n);
    scored.into_iter().map(|(_, pair)| pair).collect()
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_basic() {
        assert_eq!(dot(&[1.0, 0.0, 0.0], &[1.0, 0.0, 0.0]), 1.0);
        assert_eq!(dot(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]), 32.0);
        assert_eq!(dot(&[], &[]), 0.0);
    }
}

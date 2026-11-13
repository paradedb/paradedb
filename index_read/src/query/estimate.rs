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

use std::collections::Bound;

use crate::index::segment_pruning::SegmentStatsSnapshot;
use crate::index::stats::{EmpiricalStats, SegmentStats};
use crate::postgres::datetime::PostgresDateTime;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use tantivy::query::{
    AllQuery, BooleanQuery, BoostQuery, ConstScoreQuery, EmptyQuery, FastFieldRangeQuery, Occur,
    PhraseQuery, Query, RangeQuery, SynonymQuery, TermQuery, TermSetQuery,
};
use tantivy::schema::{Field, Type};
use tantivy::{SegmentReader, Term};

/// Maximum number of terms in a [`TermSetQuery`] for which plan-time estimation probes the term
/// dictionary instead of falling back to `scorer_estimate`.
pub(crate) const MAX_ESTIMATED_TERM_SET_LEN: usize = 64;

/// Maximum number of terms in a range query for which plan-time estimation streams `doc_freq`
/// entries from the segment's term dictionary before falling back to `.stats` interpolation or a
/// heuristic floor.
const MAX_ESTIMATED_RANGE_TERMS: usize = 512;

/// Minimum per-segment range-term inspection budget when dividing [`MAX_ESTIMATED_RANGE_TERMS`]
/// across multiple candidate segments.
const MIN_ESTIMATED_RANGE_TERMS_PER_SEGMENT: usize = 16;

/// Fallback strategy when a range query spans more than `max_range_terms` dictionary terms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RangeFallbackPolicy {
    /// Conservative row-count heuristic for planner and executor cardinality estimation:
    /// returns `0` when `.stats` proves disjointness, `max_doc` when `.stats` proves full
    /// non-null coverage, and `matches.max(max_doc / 10).min(max_doc)` otherwise so a single
    /// outlier `min` or `max` on a skewed column cannot inflate open-ended inequalities to ~100%.
    #[default]
    ConservativeRowCount,
    /// Linear `[min, max]` interpolation over `.stats` bounds for HNSW vector routing
    /// (`estimate_vector_match_fraction`), where over-estimating a broad range avoids excessive
    /// cluster over-fetching.
    UniformStats,
}

/// Shared state threaded into [`QueryEstimate::estimate_docs_with_context`].
///
/// Reuses cached [`SegmentStatsSnapshot`] handles across segments so wide range estimates do not
/// re-open `.stats` per segment, selects the wide-range fallback policy, and bounds the
/// per-segment FST range-term stream length when iterating across candidate segments.
#[derive(Clone, Copy)]
pub struct EstimateContext<'a> {
    stats_snapshot: Option<&'a SegmentStatsSnapshot>,
    range_fallback: RangeFallbackPolicy,
    max_range_terms: usize,
}

impl Default for EstimateContext<'_> {
    fn default() -> Self {
        Self {
            stats_snapshot: None,
            range_fallback: RangeFallbackPolicy::ConservativeRowCount,
            max_range_terms: MAX_ESTIMATED_RANGE_TERMS,
        }
    }
}

impl<'a> EstimateContext<'a> {
    pub(crate) fn for_vector_match_fraction(
        stats_snapshot: &'a SegmentStatsSnapshot,
        num_candidates: usize,
    ) -> Self {
        let max_range_terms = (MAX_ESTIMATED_RANGE_TERMS / num_candidates.max(1))
            .max(MIN_ESTIMATED_RANGE_TERMS_PER_SEGMENT);
        Self {
            stats_snapshot: Some(stats_snapshot),
            range_fallback: RangeFallbackPolicy::UniformStats,
            max_range_terms,
        }
    }

    pub(crate) fn for_single_segment(stats_snapshot: &'a SegmentStatsSnapshot) -> Self {
        Self {
            stats_snapshot: Some(stats_snapshot),
            range_fallback: RangeFallbackPolicy::ConservativeRowCount,
            max_range_terms: MAX_ESTIMATED_RANGE_TERMS,
        }
    }

    fn empirical_stats(
        self,
        reader: &SegmentReader,
        field: Field,
    ) -> tantivy::Result<Option<EmpiricalStats>> {
        if let Some(snapshot) = self.stats_snapshot {
            return match snapshot.try_segment_stats(reader.segment_id())? {
                Some(stats) => Ok(stats.empirical(field)?),
                None => Ok(None),
            };
        }
        match SegmentStats::of_reader(reader)? {
            Some(stats) => Ok(stats.empirical(field)?),
            None => Ok(None),
        }
    }
}

fn term_to_pdb_scalar(term: &Term) -> Option<PdbOwnedValue> {
    let val = term.value();
    match val.typ() {
        Type::I64 => val.as_i64().map(PdbOwnedValue::I64),
        Type::U64 => val.as_u64().map(PdbOwnedValue::U64),
        Type::F64 => val.as_f64().map(PdbOwnedValue::F64),
        Type::Date => val
            .as_date()
            .and_then(|dt| PostgresDateTime::try_from(dt).ok())
            .map(PdbOwnedValue::Date),
        _ => None,
    }
}

fn is_nan_scalar(value: &PdbOwnedValue) -> bool {
    matches!(value, PdbOwnedValue::F64(v) if v.is_nan())
}

fn map_term_bound(bound: &Bound<Term>) -> Option<Bound<PdbOwnedValue>> {
    Some(match bound {
        Bound::Included(t) => {
            let v = term_to_pdb_scalar(t)?;
            (!is_nan_scalar(&v)).then_some(Bound::Included(v))?
        }
        Bound::Excluded(t) => {
            let v = term_to_pdb_scalar(t)?;
            (!is_nan_scalar(&v)).then_some(Bound::Excluded(v))?
        }
        Bound::Unbounded => Bound::Unbounded,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RangeStatsProof {
    Disjoint,
    CoversAllNonNull,
    Partial,
}

fn stats_range_proof(
    stats: &EmpiricalStats,
    lower: &Bound<Term>,
    upper: &Bound<Term>,
) -> Option<RangeStatsProof> {
    if is_nan_scalar(&stats.min) || is_nan_scalar(&stats.max) {
        return None;
    }
    let lower_bound = map_term_bound(lower)?;
    let upper_bound = map_term_bound(upper)?;
    if !stats.intersects(&lower_bound, &upper_bound) {
        Some(RangeStatsProof::Disjoint)
    } else if !stats.nullable && stats.is_subset_of(&lower_bound, &upper_bound) {
        Some(RangeStatsProof::CoversAllNonNull)
    } else {
        Some(RangeStatsProof::Partial)
    }
}

/// Estimates the fraction of documents matching `(lower, upper)` assuming values are uniformly
/// distributed between `stats.min` and `stats.max` when `.stats` carries non-nullable scalar
/// bounds.
fn uniform_range_fraction(
    stats: &EmpiricalStats,
    lower: &Bound<Term>,
    upper: &Bound<Term>,
) -> Option<f64> {
    if stats.nullable {
        // Nullability also covers multivalued fields; min/max cannot estimate their document counts.
        return None;
    }
    let lower_bound = map_term_bound(lower)?;
    let upper_bound = map_term_bound(upper)?;
    let distance = |value: &PdbOwnedValue| {
        let distance = match (&stats.min, value) {
            (PdbOwnedValue::I64(min), PdbOwnedValue::I64(value)) => {
                (i128::from(*value) - i128::from(*min)) as f64
            }
            (PdbOwnedValue::U64(min), PdbOwnedValue::U64(value)) => {
                (i128::from(*value) - i128::from(*min)) as f64
            }
            (PdbOwnedValue::Date(min), PdbOwnedValue::Date(value)) => {
                (i128::from(value.into_inner()) - i128::from(min.into_inner())) as f64
            }
            (PdbOwnedValue::F64(min), PdbOwnedValue::F64(value)) => value - min,
            _ => return None,
        };
        distance.is_finite().then_some(distance)
    };
    let span = distance(&stats.max)?;
    let step = if matches!(stats.min, PdbOwnedValue::F64(_)) {
        0.0
    } else {
        1.0
    };
    let lower_offset = match &lower_bound {
        Bound::Included(value) => distance(value)?,
        Bound::Excluded(value) => distance(value)? + step,
        Bound::Unbounded => 0.0,
    };
    let upper_offset = match &upper_bound {
        Bound::Included(value) => distance(value)? + step,
        Bound::Excluded(value) => distance(value)?,
        Bound::Unbounded => span + step,
    };
    let fraction = if !stats.intersects(&lower_bound, &upper_bound) || lower_offset > upper_offset {
        0.0
    } else if span == 0.0 {
        1.0
    } else {
        ((upper_offset.min(span + step) - lower_offset.max(0.0)) / (span + step)).clamp(0.0, 1.0)
    };
    Some(fraction)
}

/// Lower and upper serialized [`Term`] bounds for a Tantivy range query.
#[derive(Clone, Debug)]
struct TermBounds {
    lower: Bound<Term>,
    upper: Bound<Term>,
}

impl TermBounds {
    fn is_empty(&self) -> bool {
        match (&self.lower, &self.upper) {
            (Bound::Included(lo), Bound::Included(hi)) => {
                lo.serialized_value_bytes() > hi.serialized_value_bytes()
            }
            (Bound::Included(lo), Bound::Excluded(hi))
            | (Bound::Excluded(lo), Bound::Included(hi))
            | (Bound::Excluded(lo), Bound::Excluded(hi)) => {
                lo.serialized_value_bytes() >= hi.serialized_value_bytes()
            }
            _ => false,
        }
    }

    fn intersect(&self, other: &Self) -> Self {
        let lower = match (&self.lower, &other.lower) {
            (Bound::Unbounded, bound) | (bound, Bound::Unbounded) => bound.clone(),
            (Bound::Included(a), Bound::Included(b)) => Bound::Included(
                if a.serialized_value_bytes() >= b.serialized_value_bytes() {
                    a.clone()
                } else {
                    b.clone()
                },
            ),
            (Bound::Excluded(a), Bound::Excluded(b)) => Bound::Excluded(
                if a.serialized_value_bytes() >= b.serialized_value_bytes() {
                    a.clone()
                } else {
                    b.clone()
                },
            ),
            (Bound::Included(inc), Bound::Excluded(exc))
            | (Bound::Excluded(exc), Bound::Included(inc)) => {
                if inc.serialized_value_bytes() > exc.serialized_value_bytes() {
                    Bound::Included(inc.clone())
                } else {
                    Bound::Excluded(exc.clone())
                }
            }
        };
        let upper = match (&self.upper, &other.upper) {
            (Bound::Unbounded, bound) | (bound, Bound::Unbounded) => bound.clone(),
            (Bound::Included(a), Bound::Included(b)) => Bound::Included(
                if a.serialized_value_bytes() <= b.serialized_value_bytes() {
                    a.clone()
                } else {
                    b.clone()
                },
            ),
            (Bound::Excluded(a), Bound::Excluded(b)) => Bound::Excluded(
                if a.serialized_value_bytes() <= b.serialized_value_bytes() {
                    a.clone()
                } else {
                    b.clone()
                },
            ),
            (Bound::Included(inc), Bound::Excluded(exc))
            | (Bound::Excluded(exc), Bound::Included(inc)) => {
                if inc.serialized_value_bytes() < exc.serialized_value_bytes() {
                    Bound::Included(inc.clone())
                } else {
                    Bound::Excluded(exc.clone())
                }
            }
        };
        Self { lower, upper }
    }

    fn estimate_docs(
        &self,
        field: Field,
        reader: &SegmentReader,
        ctx: EstimateContext<'_>,
    ) -> tantivy::Result<Option<Estimate>> {
        let field_entry = reader.schema().get_field_entry(field);
        if !field_entry.is_indexed() || field_entry.field_type().is_json() {
            return Ok(None);
        }
        if self.is_empty() {
            return Ok(Some(Estimate {
                matches: 0.0,
                cost: 0,
            }));
        }

        let max_range_terms = ctx.max_range_terms;
        let inverted_index = reader.inverted_index(field)?;
        let mut range_builder = inverted_index.terms().range();
        range_builder = match &self.lower {
            Bound::Included(term) => range_builder.ge(term.serialized_value_bytes()),
            Bound::Excluded(term) => range_builder.gt(term.serialized_value_bytes()),
            Bound::Unbounded => range_builder,
        };
        range_builder = match &self.upper {
            Bound::Included(term) => range_builder.le(term.serialized_value_bytes()),
            Bound::Excluded(term) => range_builder.lt(term.serialized_value_bytes()),
            Bound::Unbounded => range_builder,
        };
        let mut stream = range_builder
            .limit(max_range_terms as u64 + 1)
            .into_stream()?;

        let mut terms_seen = 0usize;
        let mut exceeded_limit = false;
        let mut matches = 0u64;
        let mut cost = 0u64;
        while stream.advance() {
            if terms_seen < max_range_terms {
                terms_seen += 1;
                let df = u64::from(stream.value().doc_freq);
                matches = matches.saturating_add(df);
                cost = cost.saturating_add(df);
            } else {
                exceeded_limit = true;
                break;
            }
        }

        let max_doc = u64::from(reader.max_doc());
        if exceeded_limit {
            // TODO: Keep more granular statistics to estimate nonuniform distributions above 512 terms.
            if let Some(stats) = ctx.empirical_stats(reader, field)? {
                match stats_range_proof(&stats, &self.lower, &self.upper) {
                    Some(RangeStatsProof::Disjoint) => {
                        return Ok(Some(Estimate {
                            matches: 0.0,
                            cost: 0,
                        }));
                    }
                    Some(RangeStatsProof::CoversAllNonNull) => {
                        return Ok(Some(Estimate {
                            matches: max_doc as f64,
                            cost: max_doc.max(cost),
                        }));
                    }
                    Some(RangeStatsProof::Partial) | None => {}
                }
                if ctx.range_fallback == RangeFallbackPolicy::UniformStats
                    && let Some(fraction) = uniform_range_fraction(&stats, &self.lower, &self.upper)
                {
                    let est = (fraction * max_doc as f64)
                        .max(matches as f64)
                        .min(max_doc as f64);
                    return Ok(Some(Estimate {
                        matches: est,
                        cost: max_doc.max(cost),
                    }));
                }
            }
            let fallback = if matches!(
                (&self.lower, &self.upper),
                (Bound::Unbounded, Bound::Unbounded)
            ) {
                max_doc
            } else {
                max_doc / 10
            };
            let est = matches.max(fallback).min(max_doc);
            Ok(Some(Estimate {
                matches: est as f64,
                cost: cost.max(est),
            }))
        } else {
            Ok(Some(Estimate {
                matches: (matches as f64).min(max_doc as f64),
                cost,
            }))
        }
    }
}

fn bound_term(bound: &Bound<Term>) -> Option<&Term> {
    match bound {
        Bound::Included(term) | Bound::Excluded(term) => Some(term),
        Bound::Unbounded => None,
    }
}

fn estimate_range_bounds(
    lower: &Bound<Term>,
    upper: &Bound<Term>,
    reader: &SegmentReader,
    ctx: EstimateContext<'_>,
) -> tantivy::Result<Option<Estimate>> {
    let Some(term) = bound_term(lower).or_else(|| bound_term(upper)) else {
        return AllQuery.estimate_docs_with_context(reader, ctx);
    };
    TermBounds {
        lower: lower.clone(),
        upper: upper.clone(),
    }
    .estimate_docs(term.field(), reader, ctx)
}

impl QueryEstimate for RangeQuery {
    fn estimate_docs_with_context(
        &self,
        reader: &SegmentReader,
        ctx: EstimateContext<'_>,
    ) -> tantivy::Result<Option<Estimate>> {
        estimate_range_bounds(self.lower_bound(), self.upper_bound(), reader, ctx)
    }
}

impl QueryEstimate for FastFieldRangeQuery {
    fn estimate_docs_with_context(
        &self,
        reader: &SegmentReader,
        ctx: EstimateContext<'_>,
    ) -> tantivy::Result<Option<Estimate>> {
        estimate_range_bounds(self.lower_bound(), self.upper_bound(), reader, ctx)
    }
}

/// Whether a zero `live_docs` result from [`QueryEstimate::estimate_docs`] proves that zero live
/// documents in the segment can match `query`, allowing `RA` lookup execution to skip the segment
/// without opening a scorer.
pub(crate) fn is_zero_estimate_exact(query: &dyn Query) -> bool {
    let query = unwrap_score_wrappers(query);
    if query.is::<AllQuery>()
        || query.is::<EmptyQuery>()
        || query.is::<TermQuery>()
        || query.is::<SynonymQuery>()
        || query.is::<TermSetQuery>()
        || query.is::<RangeQuery>()
        || query.is::<FastFieldRangeQuery>()
        || query.is::<ConstScoreQuery<AllQuery>>()
    {
        return true;
    }
    if let Some(bq) = query.downcast_ref::<BooleanQuery>() {
        return !has_duplicate_must_range_field(bq)
            && bq.clauses().iter().all(|(occur, child)| {
                *occur != Occur::MustNot && is_zero_estimate_exact(child.as_ref())
            });
    }
    false
}

/// Whether `Weight::scorer` iterates inverted-index postings proportional to the matching
/// document count (as opposed to scanning all `max_doc` fast-field values), making `query` cheap
/// to use as the driving side of a selective `RA` lookup intersection.
pub(crate) fn can_drive_lookup_from_postings(query: &dyn Query) -> bool {
    let query = unwrap_score_wrappers(query);
    if query.is::<EmptyQuery>()
        || query.is::<TermQuery>()
        || query.is::<SynonymQuery>()
        || query.is::<TermSetQuery>()
    {
        return true;
    }
    if let Some(bq) = query.downcast_ref::<BooleanQuery>() {
        return !bq.clauses().is_empty()
            && bq.clauses().iter().all(|(occur, child)| {
                *occur != Occur::MustNot && can_drive_lookup_from_postings(child.as_ref())
            });
    }
    false
}

pub trait QueryEstimate {
    fn is_fast_count(&self) -> bool {
        false
    }

    fn estimate_docs(&self, reader: &SegmentReader) -> tantivy::Result<Option<Estimate>> {
        self.estimate_docs_with_context(reader, EstimateContext::default())
    }

    fn estimate_docs_with_context(
        &self,
        reader: &SegmentReader,
        ctx: EstimateContext<'_>,
    ) -> tantivy::Result<Option<Estimate>>;
}

pub struct Estimate {
    matches: f64,
    pub cost: u64,
}

impl Estimate {
    pub fn live_docs(&self, reader: &SegmentReader) -> u64 {
        let live_fraction = reader.num_docs() as f64 / reader.max_doc().max(1) as f64;
        ((self.matches * live_fraction).ceil() as u64).min(reader.num_docs() as u64)
    }
}

impl QueryEstimate for dyn Query {
    fn is_fast_count(&self) -> bool {
        self.is::<AllQuery>()
            || self.is::<TermQuery>()
            || self.is::<ConstScoreQuery<AllQuery>>()
            || self
                .downcast_ref::<BooleanQuery>()
                .is_some_and(QueryEstimate::is_fast_count)
            || self
                .downcast_ref::<BoostQuery>()
                .is_some_and(QueryEstimate::is_fast_count)
            || self
                .downcast_ref::<ConstScoreQuery>()
                .is_some_and(QueryEstimate::is_fast_count)
    }

    fn estimate_docs_with_context(
        &self,
        reader: &SegmentReader,
        ctx: EstimateContext<'_>,
    ) -> tantivy::Result<Option<Estimate>> {
        macro_rules! estimate_as {
            ($($query:ty),* $(,)?) => {$(
                if let Some(query) = self.downcast_ref::<$query>() {
                    return query.estimate_docs_with_context(reader, ctx);
                }
            )*};
        }
        estimate_as! {
            AllQuery, TermQuery, SynonymQuery, TermSetQuery, PhraseQuery, BooleanQuery,
            EmptyQuery, RangeQuery, FastFieldRangeQuery, ConstScoreQuery, BoostQuery
        }
        // SearchQueryInput::All wraps AllQuery directly, without boxing it.
        if let Some(query) = self.downcast_ref::<ConstScoreQuery<AllQuery>>() {
            return query.query().estimate_docs_with_context(reader, ctx);
        }
        Ok(None)
    }
}

impl QueryEstimate for AllQuery {
    fn is_fast_count(&self) -> bool {
        true
    }

    fn estimate_docs_with_context(
        &self,
        reader: &SegmentReader,
        _ctx: EstimateContext<'_>,
    ) -> tantivy::Result<Option<Estimate>> {
        Ok(Some(Estimate {
            matches: reader.max_doc() as f64,
            cost: reader.max_doc() as u64,
        }))
    }
}

impl QueryEstimate for ConstScoreQuery {
    fn is_fast_count(&self) -> bool {
        self.query().is_fast_count()
    }

    fn estimate_docs_with_context(
        &self,
        reader: &SegmentReader,
        ctx: EstimateContext<'_>,
    ) -> tantivy::Result<Option<Estimate>> {
        self.query().estimate_docs_with_context(reader, ctx)
    }
}

impl QueryEstimate for BoostQuery {
    fn is_fast_count(&self) -> bool {
        self.query().is_fast_count()
    }

    fn estimate_docs_with_context(
        &self,
        reader: &SegmentReader,
        ctx: EstimateContext<'_>,
    ) -> tantivy::Result<Option<Estimate>> {
        self.query().estimate_docs_with_context(reader, ctx)
    }
}

impl QueryEstimate for TermQuery {
    fn is_fast_count(&self) -> bool {
        true
    }

    fn estimate_docs_with_context(
        &self,
        reader: &SegmentReader,
        _ctx: EstimateContext<'_>,
    ) -> tantivy::Result<Option<Estimate>> {
        // The index already stores how many documents contain this term.
        let term = self.term();
        if !reader.schema().get_field_entry(term.field()).is_indexed() {
            return Ok(None);
        }
        let count = reader.inverted_index(term.field())?.doc_freq(term)?;
        Ok(Some(Estimate {
            matches: count as f64,
            cost: count as u64,
        }))
    }
}

impl QueryEstimate for SynonymQuery {
    fn estimate_docs_with_context(
        &self,
        reader: &SegmentReader,
        _ctx: EstimateContext<'_>,
    ) -> tantivy::Result<Option<Estimate>> {
        // A group matches a document that has any of its terms, like an OR of the terms.
        let field = self.terms()[0].field();
        if !reader.schema().get_field_entry(field).is_indexed() {
            return Ok(None);
        }
        let inverted_index = reader.inverted_index(field)?;
        let total = reader.max_doc().max(1) as f64;
        let mut no_match = 1.0;
        let mut cost = 0u64;
        for term in self.terms() {
            let count = inverted_index.doc_freq(term)?;
            no_match *= 1.0 - count as f64 / total;
            cost = cost.saturating_add(count as u64);
        }
        Ok(Some(Estimate {
            matches: (total * (1.0 - no_match)).clamp(0.0, total),
            cost,
        }))
    }
}

impl QueryEstimate for TermSetQuery {
    fn estimate_docs_with_context(
        &self,
        reader: &SegmentReader,
        _ctx: EstimateContext<'_>,
    ) -> tantivy::Result<Option<Estimate>> {
        let mut terms = Vec::new();
        let mut exceeded_limit = false;
        self.query_terms(Field::from_field_id(0), reader, &mut |term, _| {
            if terms.len() < MAX_ESTIMATED_TERM_SET_LEN {
                terms.push(term.clone());
            } else {
                exceeded_limit = true;
            }
        });
        if exceeded_limit {
            return Ok(None);
        }
        if terms.is_empty() {
            return Ok(Some(Estimate {
                matches: 0.0,
                cost: 0,
            }));
        }
        terms.sort_unstable();
        terms.dedup();

        let total = reader.max_doc() as f64;
        let mut matches = 0u64;
        let mut cost = 0u64;
        for term in &terms {
            if !reader.schema().get_field_entry(term.field()).is_indexed() {
                return Ok(None);
            }
            let count = u64::from(reader.inverted_index(term.field())?.doc_freq(term)?);
            matches = matches.saturating_add(count);
            cost = cost.saturating_add(count);
        }
        Ok(Some(Estimate {
            matches: (matches as f64).min(total),
            cost,
        }))
    }
}

impl QueryEstimate for PhraseQuery {
    fn estimate_docs_with_context(
        &self,
        reader: &SegmentReader,
        _ctx: EstimateContext<'_>,
    ) -> tantivy::Result<Option<Estimate>> {
        // Multiply the fraction of documents containing each term, then scale back to a count.
        // Divide by 10 times the phrase length to roughly account for word order.
        let total = reader.max_doc().max(1) as f64;
        let terms = self.phrase_terms();
        let mut fraction = 1.0;
        let mut shortest = u64::MAX;
        for term in &terms {
            let count = reader.inverted_index(term.field())?.doc_freq(term)?;
            fraction *= count as f64 / total;
            shortest = shortest.min(count as u64);
        }
        let checks = (10 * terms.len()) as f64;
        Ok(Some(Estimate {
            matches: fraction * total / checks,
            cost: ((fraction * total * checks).ceil() as u64).max(shortest),
        }))
    }
}

fn unwrap_score_wrappers(mut query: &dyn Query) -> &dyn Query {
    loop {
        if let Some(cs) = query.downcast_ref::<ConstScoreQuery>() {
            query = cs.query().as_ref();
        } else if let Some(boost) = query.downcast_ref::<BoostQuery>() {
            query = boost.query().as_ref();
        } else if let Some(bq) = query.downcast_ref::<BooleanQuery>()
            && let [(occur @ (Occur::Must | Occur::Should), child)] = bq.clauses()
            && bq.get_minimum_number_should_match() <= usize::from(*occur == Occur::Should)
        {
            query = child.as_ref();
        } else {
            return query;
        }
    }
}

fn extract_estimable_range(query: &dyn Query) -> Option<(Field, TermBounds)> {
    let unwrapped = unwrap_score_wrappers(query);
    let (lower, upper) = if let Some(rq) = unwrapped.downcast_ref::<RangeQuery>() {
        (rq.lower_bound(), rq.upper_bound())
    } else {
        let rq = unwrapped.downcast_ref::<FastFieldRangeQuery>()?;
        (rq.lower_bound(), rq.upper_bound())
    };
    let term = bound_term(lower).or_else(|| bound_term(upper))?;
    Some((
        term.field(),
        TermBounds {
            lower: lower.clone(),
            upper: upper.clone(),
        },
    ))
}

fn collect_must_leaves<'a>(clauses: &'a [(Occur, Box<dyn Query>)], out: &mut Vec<&'a dyn Query>) {
    for (occur, child) in clauses {
        if *occur != Occur::Must {
            continue;
        }
        if let Some(bq) = unwrap_score_wrappers(child.as_ref()).downcast_ref::<BooleanQuery>()
            && !bq.clauses().is_empty()
            && bq.clauses().iter().all(|(o, _)| *o == Occur::Must)
            && bq.get_minimum_number_should_match() == 0
        {
            collect_must_leaves(bq.clauses(), out);
        } else {
            out.push(child.as_ref());
        }
    }
}

fn has_duplicate_must_range_field(query: &BooleanQuery) -> bool {
    let mut leaves = Vec::new();
    collect_must_leaves(query.clauses(), &mut leaves);
    let mut seen_fields: Vec<Field> = Vec::new();
    for leaf in leaves {
        if let Some((field, _)) = extract_estimable_range(leaf) {
            if seen_fields.contains(&field) {
                return true;
            }
            seen_fields.push(field);
        }
    }
    false
}

impl QueryEstimate for BooleanQuery {
    fn is_fast_count(&self) -> bool {
        self.clauses()
            .iter()
            .all(|(_, query)| query.is_fast_count())
    }

    fn estimate_docs_with_context(
        &self,
        reader: &SegmentReader,
        ctx: EstimateContext<'_>,
    ) -> tantivy::Result<Option<Estimate>> {
        // Exact match queries become terms joined by AND or OR, so they use this estimate too.
        // AND starts with the smallest estimate; each extra condition reduces it less.
        // OR estimates how many documents match enough conditions.
        // NOT removes the estimated share of documents matching the excluded conditions.
        if let [(occur @ (Occur::Must | Occur::Should), child)] = self.clauses()
            && self.get_minimum_number_should_match() <= usize::from(*occur == Occur::Should)
        {
            return child.estimate_docs_with_context(reader, ctx);
        }
        let total = reader.max_doc().max(1) as f64;
        let mut required = Vec::new();
        let mut excluded = 1.0;
        let mut optional = Vec::new();
        let mut required_cost: Option<u64> = None;
        let mut optional_cost = 0u64;

        // SQL range predicates (`col >= a AND col < b`, `BETWEEN`) arrive as separate `Occur::Must`
        // range clauses on the same field. Intersecting their bounds avoids treating half-open
        // bounds as independent 10%-selectivity filters.
        if has_duplicate_must_range_field(self) {
            let mut must_leaves = Vec::new();
            collect_must_leaves(self.clauses(), &mut must_leaves);
            let mut merged_ranges: Vec<(Field, TermBounds)> = Vec::new();
            for leaf in must_leaves {
                if let Some((field, bounds)) = extract_estimable_range(leaf) {
                    if let Some((_, existing)) = merged_ranges.iter_mut().find(|(f, _)| *f == field)
                    {
                        *existing = existing.intersect(&bounds);
                    } else {
                        merged_ranges.push((field, bounds));
                    }
                } else {
                    let Some(child) = leaf.estimate_docs_with_context(reader, ctx)? else {
                        return Ok(None);
                    };
                    required.push(child.matches);
                    required_cost =
                        Some(required_cost.map_or(child.cost, |cost| cost.min(child.cost)));
                }
            }
            for (field, bounds) in &merged_ranges {
                let Some(range_est) = bounds.estimate_docs(*field, reader, ctx)? else {
                    return Ok(None);
                };
                required.push(range_est.matches);
                required_cost =
                    Some(required_cost.map_or(range_est.cost, |cost| cost.min(range_est.cost)));
            }
            for (occur, child) in self.clauses() {
                if *occur == Occur::Must {
                    continue;
                }
                let Some(child) = child.estimate_docs_with_context(reader, ctx)? else {
                    return Ok(None);
                };
                match occur {
                    Occur::Should => {
                        optional.push(child.matches / total);
                        optional_cost = optional_cost.saturating_add(child.cost);
                    }
                    Occur::MustNot => excluded *= 1.0 - child.matches / total,
                    Occur::Must => unreachable!(),
                }
            }
        } else {
            for (occur, child) in self.clauses() {
                let Some(child) = child.estimate_docs_with_context(reader, ctx)? else {
                    return Ok(None);
                };
                match occur {
                    Occur::Must => {
                        required.push(child.matches);
                        required_cost =
                            Some(required_cost.map_or(child.cost, |cost| cost.min(child.cost)));
                    }
                    Occur::Should => {
                        optional.push(child.matches / total);
                        optional_cost = optional_cost.saturating_add(child.cost);
                    }
                    Occur::MustNot => excluded *= 1.0 - child.matches / total,
                }
            }
        }

        required.sort_unstable_by(f64::total_cmp);
        let mut required = required.into_iter();
        let mut required_matches = required.next().unwrap_or(total);
        let mut exponent = 0.5;
        for matches in required {
            required_matches *= (matches / total).powf(exponent);
            exponent *= 0.5;
        }
        let minimum = self
            .get_minimum_number_should_match()
            .max(usize::from(required_cost.is_none()));
        let optional_fraction = if minimum == 0 {
            1.0
        } else if minimum > optional.len() {
            0.0
        } else {
            let mut probabilities = vec![0.0; minimum + 1];
            probabilities[0] = 1.0;
            for fraction in optional {
                probabilities[minimum] += probabilities[minimum - 1] * fraction;
                for n in (1..minimum).rev() {
                    probabilities[n] =
                        probabilities[n] * (1.0 - fraction) + probabilities[n - 1] * fraction;
                }
                probabilities[0] *= 1.0 - fraction;
            }
            probabilities[minimum]
        };
        Ok(Some(Estimate {
            matches: (required_matches * (optional_fraction * excluded)).clamp(0.0, total),
            cost: required_cost.unwrap_or(optional_cost),
        }))
    }
}

// Tokenization can turn an eligible match or phrase into an empty query.
impl QueryEstimate for EmptyQuery {
    fn estimate_docs_with_context(
        &self,
        _reader: &SegmentReader,
        _ctx: EstimateContext<'_>,
    ) -> tantivy::Result<Option<Estimate>> {
        Ok(Some(Estimate {
            matches: 0.0,
            cost: 0,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use tantivy::collector::Count;
    use tantivy::query::RegexQuery;
    use tantivy::schema::{
        DateOptions, DateTimePrecision, FAST, INDEXED, IndexRecordOption, STORED, STRING, Schema,
        TEXT,
    };
    use tantivy::{DateTime, Index, TantivyDocument, Term, doc};

    #[derive(Clone, Debug)]
    enum Exactness {
        Exact,
        MustNumRange(Vec<(Bound<i64>, Bound<i64>)>),
        Approximate,
        Unsupported,
    }

    impl Exactness {
        fn is_exact_on_undeleted(&self, indexed_nums: &[i64]) -> bool {
            match self {
                Self::Exact => true,
                Self::MustNumRange(ranges) => {
                    indexed_nums
                        .iter()
                        .filter(|&&v| {
                            ranges.iter().all(|(lo, hi)| {
                                let lo_ok = match lo {
                                    Bound::Included(b) => v >= *b,
                                    Bound::Excluded(b) => v > *b,
                                    Bound::Unbounded => true,
                                };
                                let hi_ok = match hi {
                                    Bound::Included(b) => v <= *b,
                                    Bound::Excluded(b) => v < *b,
                                    Bound::Unbounded => true,
                                };
                                lo_ok && hi_ok
                            })
                        })
                        .count()
                        <= MAX_ESTIMATED_RANGE_TERMS
                }
                Self::Approximate | Self::Unsupported => false,
            }
        }
    }

    #[derive(Debug)]
    struct GenQuery {
        query: Box<dyn Query>,
        exactness: Exactness,
        fast_count: bool,
    }

    impl Clone for GenQuery {
        fn clone(&self) -> Self {
            Self {
                query: self.query.box_clone(),
                exactness: self.exactness.clone(),
                fast_count: self.fast_count,
            }
        }
    }

    impl GenQuery {
        fn new(query: impl Query, exactness: Exactness, fast_count: bool) -> Self {
            Self {
                query: Box::new(query),
                exactness,
                fast_count,
            }
        }
    }

    fn arb_query(
        num_field: Field,
        text_field: Field,
        unindexed_field: Field,
        json_field: Field,
    ) -> impl Strategy<Value = GenQuery> {
        let nums = prop::sample::select(vec![
            -10i64, 0, 10, 20, 21, 549, 550, 1000, 1500, 2000, 9999,
        ]);
        let words = prop::sample::select(vec!["alpha", "beta", "gamma", "cluster", "absent"]);
        let finite_bound = prop_oneof![
            nums.clone().prop_map(Bound::Included),
            nums.clone().prop_map(Bound::Excluded),
        ];
        let range_bounds = prop_oneof![
            (finite_bound.clone(), finite_bound.clone()),
            finite_bound.clone().prop_map(|lo| (lo, Bound::Unbounded)),
            finite_bound.prop_map(|hi| (Bound::Unbounded, hi)),
        ];
        let map_bound = move |b: Bound<i64>| match b {
            Bound::Included(v) => Bound::Included(Term::from_field_i64(num_field, v)),
            Bound::Excluded(v) => Bound::Excluded(Term::from_field_i64(num_field, v)),
            Bound::Unbounded => Bound::Unbounded,
        };
        let mut json_lo = Term::from_field_json_path(json_field, "k", false);
        json_lo.append_type_and_fast_value(0i64);
        let mut json_hi = Term::from_field_json_path(json_field, "k", false);
        json_hi.append_type_and_fast_value(10i64);

        let leaf = prop_oneof![
            2 => Just(GenQuery::new(AllQuery, Exactness::Exact, true)),
            1 => Just(GenQuery::new(ConstScoreQuery::new(AllQuery, 0.0), Exactness::Exact, true)),
            1 => Just(GenQuery::new(EmptyQuery, Exactness::Exact, false)),
            3 => nums.clone().prop_map(move |v| GenQuery::new(
                TermQuery::new(Term::from_field_i64(num_field, v), IndexRecordOption::Basic),
                Exactness::Exact,
                true,
            )),
            3 => words.clone().prop_map(move |w| GenQuery::new(
                TermQuery::new(Term::from_field_text(text_field, w), IndexRecordOption::Basic),
                Exactness::Exact,
                true,
            )),
            2 => prop::collection::vec(nums.clone(), 0..=4).prop_map(move |vs| GenQuery::new(
                TermSetQuery::new(vs.into_iter().map(|v| Term::from_field_i64(num_field, v))),
                Exactness::Exact,
                false,
            )),
            2 => prop::collection::vec(words.clone(), 0..=4).prop_map(move |ws| GenQuery::new(
                TermSetQuery::new(ws.into_iter().map(|w| Term::from_field_text(text_field, w))),
                Exactness::Approximate,
                false,
            )),
            2 => prop::collection::vec(words.clone(), 1..=4).prop_map(move |ws| GenQuery::new(
                SynonymQuery::new(ws.into_iter().map(|w| Term::from_field_text(text_field, w)).collect()),
                Exactness::Approximate,
                false,
            )),
            4 => (any::<bool>(), range_bounds).prop_map(move |(ff, (lo, hi))| {
                let exactness = Exactness::MustNumRange(vec![(lo, hi)]);
                if ff {
                    GenQuery::new(FastFieldRangeQuery::new(map_bound(lo), map_bound(hi)), exactness, false)
                } else {
                    GenQuery::new(RangeQuery::new(map_bound(lo), map_bound(hi)), exactness, false)
                }
            }),
            2 => prop::collection::vec(words, 2..=3).prop_map(move |ws| GenQuery::new(
                PhraseQuery::new(ws.into_iter().map(|w| Term::from_field_text(text_field, w)).collect()),
                Exactness::Approximate,
                false,
            )),
            1 => Just(GenQuery::new(
                TermSetQuery::new((0..=MAX_ESTIMATED_TERM_SET_LEN as i64).map(|v| Term::from_field_i64(num_field, v))),
                Exactness::Unsupported,
                false,
            )),
            1 => nums.prop_map(move |v| GenQuery::new(
                TermQuery::new(Term::from_field_i64(unindexed_field, v), IndexRecordOption::Basic),
                Exactness::Unsupported,
                true,
            )),
            1 => Just(GenQuery::new(
                SynonymQuery::new(vec![
                    Term::from_field_i64(unindexed_field, 1),
                    Term::from_field_i64(unindexed_field, 2),
                ]),
                Exactness::Unsupported,
                false,
            )),
            1 => Just(GenQuery::new(
                TermSetQuery::new([1, 2].map(|v| Term::from_field_i64(unindexed_field, v))),
                Exactness::Unsupported,
                false,
            )),
            1 => Just(GenQuery::new(
                RangeQuery::new(
                    Bound::Included(Term::from_field_i64(unindexed_field, 0)),
                    Bound::Included(Term::from_field_i64(unindexed_field, 10)),
                ),
                Exactness::Unsupported,
                false,
            )),
            1 => Just(GenQuery::new(
                RangeQuery::new(Bound::Included(json_lo), Bound::Included(json_hi)),
                Exactness::Unsupported,
                false,
            )),
            1 => prop::sample::select(vec!["alp.*", "zzz.*"]).prop_map(move |pat| GenQuery::new(
                RegexQuery::from_pattern(pat, text_field).unwrap(),
                Exactness::Unsupported,
                false,
            )),
        ];

        leaf.prop_recursive(2, 10, 4, |inner| {
            let occur = prop_oneof![
                3 => Just(Occur::Must),
                3 => Just(Occur::Should),
                1 => Just(Occur::MustNot),
            ];
            prop_oneof![
                5 => (prop::collection::vec((occur, inner.clone()), 0..=4), 0usize..=4)
                    .prop_map(|(clauses, min_should)| {
                        let fast_count = clauses.iter().all(|(_, c)| c.fast_count);
                        let exactness = if let [(occur @ (Occur::Must | Occur::Should), c)] = clauses.as_slice() {
                            if min_should <= usize::from(*occur == Occur::Should) {
                                c.exactness.clone()
                            } else if matches!(c.exactness, Exactness::Unsupported) {
                                Exactness::Unsupported
                            } else {
                                Exactness::Exact
                            }
                        } else if clauses.iter().any(|(_, c)| matches!(c.exactness, Exactness::Unsupported)) {
                            Exactness::Unsupported
                        } else if min_should == 0
                            && !clauses.is_empty()
                            && clauses.iter().all(|(o, c)| *o == Occur::Must && matches!(c.exactness, Exactness::MustNumRange(_)))
                        {
                            let merged = clauses
                                .iter()
                                .filter_map(|(_, c)| match &c.exactness {
                                    Exactness::MustNumRange(rs) => Some(rs.iter().copied()),
                                    _ => None,
                                })
                                .flatten()
                                .collect();
                            Exactness::MustNumRange(merged)
                        } else {
                            Exactness::Approximate
                        };
                        let subqueries = clauses.into_iter().map(|(o, c)| (o, c.query)).collect();
                        GenQuery::new(
                            BooleanQuery::with_minimum_required_clauses(subqueries, min_should),
                            exactness,
                            fast_count,
                        )
                    }),
                1 => inner.clone().prop_map(|c| GenQuery::new(BoostQuery::new(c.query, 2.0), c.exactness, c.fast_count)),
                1 => inner.prop_map(|c| GenQuery::new(ConstScoreQuery::new(c.query, 0.5), c.exactness, c.fast_count)),
            ]
        })
    }

    #[test]
    fn query_estimate_properties() {
        let mut sb = Schema::builder();
        let num_field = sb.add_i64_field("num", INDEXED | FAST);
        let text_field = sb.add_text_field("text", TEXT);
        let unindexed_field = sb.add_i64_field("unindexed", FAST);
        let json_field = sb.add_json_field("json", STORED | TEXT | FAST);
        let index = Index::create_in_ram(sb.build());
        let mut writer = index
            .writer_with_num_threads::<TantivyDocument>(1, 50_000_000)
            .unwrap();

        // 600 docs (> MAX_ESTIMATED_RANGE_TERMS = 512): 550 distinct values 0..550 plus clusters
        // at 1000 (20 docs) and 2000 (30 docs).
        for id in 0i64..550 {
            let text = match id % 4 {
                0 => "alpha beta",
                1 => "alpha gamma",
                2 => "beta gamma",
                _ => "noise",
            };
            writer
                .add_document(doc!(num_field => id, unindexed_field => id, text_field => text))
                .unwrap();
        }
        for (val, count) in [(1000i64, 20usize), (2000, 30)] {
            for _ in 0..count {
                writer
                    .add_document(
                        doc!(num_field => val, unindexed_field => val, text_field => "cluster"),
                    )
                    .unwrap();
            }
        }
        writer.commit().unwrap();

        let reader = index.reader().unwrap();
        let full_searcher = reader.searcher();
        let full_segment = full_searcher.segment_reader(0);
        assert_eq!(full_segment.num_docs(), 600);

        writer.delete_term(Term::from_field_text(text_field, "noise"));
        writer.commit().unwrap();
        reader.reload().unwrap();
        let deleted_searcher = reader.searcher();
        let deleted_segment = deleted_searcher.segment_reader(0);
        assert!(deleted_segment.num_docs() < full_segment.num_docs());

        let indexed_nums: Vec<i64> = (0i64..550).chain([1000, 2000]).collect();

        let unbounded_range: Box<dyn Query> =
            Box::new(RangeQuery::new(Bound::Unbounded, Bound::Unbounded));
        let unbounded_est = unbounded_range
            .estimate_docs(full_segment)
            .unwrap()
            .unwrap();
        assert_eq!(
            (unbounded_est.live_docs(full_segment), unbounded_est.cost),
            (600, 600)
        );

        proptest!(|(g in arb_query(num_field, text_field, unindexed_field, json_field))| {
            let q = g.query.as_ref();
            prop_assert_eq!(q.is_fast_count(), g.fast_count);
            if can_drive_lookup_from_postings(q) {
                prop_assert!(is_zero_estimate_exact(q));
            }
            if let Some(bq) = q.downcast_ref::<BooleanQuery>()
                && has_duplicate_must_range_field(bq)
            {
                prop_assert!(!is_zero_estimate_exact(q));
            }

            let est_full = q.estimate_docs(full_segment).unwrap();
            let est_del = q.estimate_docs(deleted_segment).unwrap();
            let unsupported = matches!(g.exactness, Exactness::Unsupported);
            prop_assert_eq!(est_full.is_none(), unsupported);
            prop_assert_eq!(est_del.is_none(), unsupported);

            if let (Some(ef), Some(ed)) = (est_full, est_del) {
                let full_live = ef.live_docs(full_segment);
                let del_live = ed.live_docs(deleted_segment);
                prop_assert!(full_live <= u64::from(full_segment.num_docs()));
                prop_assert!(del_live <= u64::from(deleted_segment.num_docs()));
                prop_assert!(del_live <= full_live);
                prop_assert_eq!(ed.cost, ef.cost);

                for wrapped in [
                    Box::new(BoostQuery::new(q.box_clone(), 3.0)) as Box<dyn Query>,
                    Box::new(ConstScoreQuery::new(q.box_clone(), 0.0)),
                    Box::new(BooleanQuery::new(vec![(Occur::Must, q.box_clone())])),
                ] {
                    let wf = wrapped.estimate_docs(full_segment).unwrap().unwrap();
                    prop_assert_eq!((wf.matches, wf.cost), (ef.matches, ef.cost));
                    prop_assert_eq!(wrapped.is_fast_count(), q.is_fast_count());
                }

                if let Some(bq) = q.downcast_ref::<BooleanQuery>() {
                    let rev_clauses = bq
                        .clauses()
                        .iter()
                        .rev()
                        .map(|(o, c)| (*o, c.box_clone()))
                        .collect();
                    let rev_q: Box<dyn Query> = Box::new(
                        BooleanQuery::with_minimum_required_clauses(
                            rev_clauses,
                            bq.get_minimum_number_should_match(),
                        ),
                    );
                    let rf = rev_q.estimate_docs(full_segment).unwrap().unwrap();
                    prop_assert_eq!(rf.cost, ef.cost);
                    prop_assert!((rf.matches - ef.matches).abs() <= 1e-9);
                }

                let actual_full = full_searcher.search(q, &Count).unwrap() as u64;
                let actual_del = deleted_searcher.search(q, &Count).unwrap() as u64;
                if is_zero_estimate_exact(q) {
                    if full_live == 0 {
                        prop_assert_eq!(actual_full, 0);
                    }
                    if del_live == 0 {
                        prop_assert_eq!(actual_del, 0);
                    }
                }
                if g.exactness.is_exact_on_undeleted(&indexed_nums) {
                    prop_assert_eq!(full_live, actual_full);
                }
            }
        });
    }

    #[test]
    fn uniform_stats_range_estimation() {
        let mut sb = Schema::builder();
        let id_field = sb.add_i64_field("id", INDEXED | FAST);
        let unsigned_field = sb.add_u64_field("unsigned", INDEXED | FAST);
        let wide_field = sb.add_i64_field("wide", INDEXED | FAST);
        let date_field = sb.add_date_field(
            "date",
            DateOptions::from(INDEXED | FAST).set_precision(DateTimePrecision::Microseconds),
        );
        let score_field = sb.add_f64_field("score", INDEXED | FAST);
        let constant_field = sb.add_f64_field("constant", INDEXED | FAST);
        let nonfinite_field = sb.add_f64_field("nonfinite", INDEXED | FAST);
        let skewed_field = sb.add_i64_field("skewed", INDEXED | FAST);
        let nullable_field = sb.add_i64_field("nullable", INDEXED | FAST);
        let name_field = sb.add_text_field("name", STRING | FAST);

        let index = crate::index::stats::create_in_ram(sb.build());
        let mut writer = index
            .writer_with_num_threads::<TantivyDocument>(1, 50_000_000)
            .unwrap();

        let total = 10_000i64;
        for g in 1..=total {
            let skewed = if g <= 9_000 {
                1
            } else if g < total {
                100 + (g - 9_000)
            } else {
                1_400_000
            };
            let mut d = doc!(
                id_field => g,
                unsigned_field => (i64::MAX as u64) + (g as u64),
                wide_field => i64::MAX - total + g,
                date_field => DateTime::from_timestamp_micros(g),
                score_field => (g - 1) as f64 * 1000.0 / (total - 1) as f64,
                constant_field => 100.0f64,
                nonfinite_field => if g == total { f64::NAN } else { g as f64 },
                skewed_field => skewed,
                name_field => format!("s{g:05}"),
            );
            if g > 1 {
                d.add_i64(nullable_field, g);
            }
            writer.add_document(d).unwrap();
        }
        writer.commit().unwrap();

        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let segment = searcher.segment_reader(0);
        assert_eq!(segment.num_docs(), total as u32);
        assert!(SegmentStats::of_reader(segment).unwrap().is_some());
        let snapshot = SegmentStatsSnapshot::for_test(&searcher);
        let row_ctx = EstimateContext::for_single_segment(&snapshot);
        let vec_ctx = EstimateContext::for_vector_match_fraction(&snapshot, 1);

        let live_with = |ctx: EstimateContext<'_>, lo: Bound<Term>, hi: Bound<Term>| -> u64 {
            let rq: Box<dyn Query> = Box::new(RangeQuery::new(lo.clone(), hi.clone()));
            let ffrq: Box<dyn Query> = Box::new(FastFieldRangeQuery::new(lo, hi));
            let est_rq = rq
                .estimate_docs_with_context(segment, ctx)
                .unwrap()
                .unwrap();
            let est_ff = ffrq
                .estimate_docs_with_context(segment, ctx)
                .unwrap()
                .unwrap();
            assert!(!est_rq.matches.is_nan());
            assert_eq!(est_rq.live_docs(segment), est_ff.live_docs(segment));
            est_rq.live_docs(segment)
        };
        let live_row = |lo: Bound<Term>, hi: Bound<Term>| -> u64 {
            let default_est = RangeQuery::new(lo.clone(), hi.clone())
                .estimate_docs(segment)
                .unwrap()
                .unwrap()
                .live_docs(segment);
            let ctx_est = live_with(row_ctx, lo, hi);
            assert_eq!(default_est, ctx_est);
            ctx_est
        };
        let live_vec = |lo: Bound<Term>, hi: Bound<Term>| live_with(vec_ctx, lo, hi);

        let term_builders: &[(&str, &dyn Fn(i64) -> Term)] = &[
            ("id", &|v| Term::from_field_i64(id_field, v)),
            ("unsigned", &|v| {
                Term::from_field_u64(unsigned_field, (i64::MAX as u64) + (v as u64))
            }),
            ("wide", &|v| {
                Term::from_field_i64(wide_field, i64::MAX - total + v)
            }),
            ("date", &|v| {
                Term::from_field_date(date_field, DateTime::from_timestamp_micros(v))
            }),
        ];
        for (label, mk_term) in term_builders {
            for (lo, hi, expected_row, expected_vec) in [
                // Narrow ranges (<= 512 terms): exact term-dictionary counts under both policies.
                (Bound::Included(9_901), Bound::Unbounded, 100, 100),
                (Bound::Excluded(9_900), Bound::Unbounded, 100, 100),
                (Bound::Unbounded, Bound::Included(100), 100, 100),
                (Bound::Unbounded, Bound::Excluded(101), 100, 100),
                (Bound::Included(10_000), Bound::Included(10_000), 1, 1),
                (Bound::Included(20), Bound::Excluded(20), 0, 0),
                (Bound::Included(90), Bound::Included(10), 0, 0),
                (Bound::Excluded(10_000), Bound::Unbounded, 0, 0),
                (Bound::Unbounded, Bound::Excluded(1), 0, 0),
                // Full non-null coverage of `[min, max]`: exact `10_000` under both policies.
                (Bound::Included(1), Bound::Unbounded, 10_000, 10_000),
                (Bound::Unbounded, Bound::Included(10_000), 10_000, 10_000),
                (Bound::Unbounded, Bound::Unbounded, 10_000, 10_000),
                // Partial wide ranges (> 512 terms): conservative 10% (`1_000`) for row counts,
                // uniform `.stats` interpolation (`600` / `8_000`) for vector routing.
                (Bound::Included(9_401), Bound::Unbounded, 1_000, 600),
                (Bound::Excluded(9_400), Bound::Unbounded, 1_000, 600),
                (Bound::Unbounded, Bound::Included(600), 1_000, 600),
                (Bound::Unbounded, Bound::Excluded(601), 1_000, 600),
                (Bound::Included(2_001), Bound::Unbounded, 1_000, 8_000),
                (Bound::Excluded(2_000), Bound::Unbounded, 1_000, 8_000),
                (Bound::Unbounded, Bound::Included(8_000), 1_000, 8_000),
                (Bound::Unbounded, Bound::Excluded(8_001), 1_000, 8_000),
                (Bound::Included(1_001), Bound::Included(9_000), 1_000, 8_000),
            ] {
                assert_eq!(
                    live_row(lo.map(mk_term), hi.map(mk_term)),
                    expected_row,
                    "{label} row"
                );
                assert_eq!(
                    live_vec(lo.map(mk_term), hi.map(mk_term)),
                    expected_vec,
                    "{label} vec"
                );
            }
        }

        // Power-law skewed column (`min = 1`, `max = 1_400_000`, `1_000` docs > 100 across 1,000
        // distinct terms): conservative row estimation returns `1_000` (10%) instead of ~`9_999`.
        assert_eq!(
            live_row(
                Bound::Excluded(Term::from_field_i64(skewed_field, 100)),
                Bound::Unbounded
            ),
            1_000
        );

        // Float ranges: conservative `1_000` for row counts, uniform `600` / `8_000` for vector
        // routing, plus exact constant and `NaN`-safe fallback.
        let f_term = |f, v| Term::from_field_f64(f, v);
        assert_eq!(
            live_row(
                Bound::Included(f_term(score_field, 940.0)),
                Bound::Unbounded
            ),
            1_000
        );
        assert_eq!(
            live_vec(
                Bound::Included(f_term(score_field, 940.0)),
                Bound::Unbounded
            ),
            600
        );
        assert_eq!(
            live_row(
                Bound::Included(f_term(score_field, 200.0)),
                Bound::Unbounded
            ),
            1_000
        );
        assert_eq!(
            live_vec(
                Bound::Included(f_term(score_field, 200.0)),
                Bound::Unbounded
            ),
            8_000
        );
        for (lo, hi, expected) in [
            (
                Bound::Included(f_term(constant_field, 90.0)),
                Bound::Unbounded,
                10_000,
            ),
            (
                Bound::Included(f_term(constant_field, 100.0)),
                Bound::Included(f_term(constant_field, 100.0)),
                10_000,
            ),
            (
                Bound::Excluded(f_term(constant_field, 100.0)),
                Bound::Unbounded,
                0,
            ),
            (
                Bound::Unbounded,
                Bound::Excluded(f_term(constant_field, 100.0)),
                0,
            ),
        ] {
            assert_eq!(live_row(lo.clone(), hi.clone()), expected);
            assert_eq!(live_vec(lo, hi), expected);
        }
        // Non-finite `.stats` (`NaN`) falls back cleanly without producing `NaN` estimates.
        assert_eq!(
            live_row(
                Bound::Included(f_term(nonfinite_field, 1.0)),
                Bound::Unbounded
            ),
            1_000
        );
        assert_eq!(
            live_vec(
                Bound::Included(f_term(nonfinite_field, 1.0)),
                Bound::Unbounded
            ),
            1_000
        );

        // Nullable and string fields: exact below 512 terms, heuristic 10% floor above 512 terms.
        for live in [&live_row as &dyn Fn(_, _) -> u64, &live_vec] {
            assert_eq!(
                live(
                    Bound::Included(Term::from_field_i64(nullable_field, 9_901)),
                    Bound::Unbounded
                ),
                100
            );
            assert_eq!(
                live(
                    Bound::Included(Term::from_field_i64(nullable_field, 2_001)),
                    Bound::Unbounded
                ),
                1_000
            );
            assert_eq!(
                live(
                    Bound::Included(Term::from_field_text(name_field, "s09901")),
                    Bound::Unbounded
                ),
                100
            );
            assert_eq!(
                live(
                    Bound::Included(Term::from_field_text(name_field, "s02001")),
                    Bound::Unbounded
                ),
                1_000
            );
        }

        // Cached `SegmentStatsSnapshot` opens `.stats` once across repeated range estimates, and
        // `EstimateContext::for_vector_match_fraction(..., 32)` caps per-segment FST enumeration at 16 terms.
        use crate::index::segment_pruning::STATS_OPENS;
        use std::sync::atomic::Ordering::Relaxed;
        let fresh_snapshot = SegmentStatsSnapshot::for_test(&searcher);
        let ctx_32 = EstimateContext::for_vector_match_fraction(&fresh_snapshot, 32);
        assert_eq!(
            ctx_32.max_range_terms,
            MIN_ESTIMATED_RANGE_TERMS_PER_SEGMENT
        );
        STATS_OPENS.store(0, Relaxed);
        let narrow_16: Box<dyn Query> = Box::new(RangeQuery::new(
            Bound::Included(Term::from_field_text(name_field, "s09985")),
            Bound::Unbounded,
        ));
        let medium_100: Box<dyn Query> = Box::new(RangeQuery::new(
            Bound::Included(Term::from_field_text(name_field, "s09901")),
            Bound::Unbounded,
        ));
        let medium_id_100: Box<dyn Query> = Box::new(RangeQuery::new(
            Bound::Included(Term::from_field_i64(id_field, 9_901)),
            Bound::Unbounded,
        ));
        assert_eq!(
            narrow_16
                .estimate_docs_with_context(segment, ctx_32)
                .unwrap()
                .unwrap()
                .live_docs(segment),
            16
        );
        assert_eq!(STATS_OPENS.load(Relaxed), 0);
        // Exceeds the 32-segment per-segment FST cap (16 < 100 <= 512): string falls back to 10%,
        // while numeric `id` interpolates cached `.stats` bounds (1% = 100) opening `.stats` once.
        assert_eq!(
            medium_100
                .estimate_docs_with_context(segment, ctx_32)
                .unwrap()
                .unwrap()
                .live_docs(segment),
            1_000
        );
        assert_eq!(STATS_OPENS.load(Relaxed), 1);
        assert_eq!(
            medium_id_100
                .estimate_docs_with_context(segment, ctx_32)
                .unwrap()
                .unwrap()
                .live_docs(segment),
            100
        );
        assert_eq!(STATS_OPENS.load(Relaxed), 1);
    }
}

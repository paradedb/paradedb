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

use std::ops::Bound;

use super::SearchQueryInput;
use super::numeric::convert_value_for_field;
use super::pdb_query::{canonicalize_range_bounds_for_field, pdb};
use crate::api::version::Version;
use crate::api::{FieldName, HashMap};
use crate::index::segment_pruning::SegmentStatsSnapshot;
use crate::index::segment_pruning::predicate::{
    boolean_can_match, boolean_matches_all, range_can_match, range_matches_all, term_can_match,
    term_matches_all,
};
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::schema::{SearchField, SearchIndexSchema};
use tantivy::SegmentOrdinal;
use tantivy::schema::FieldType;

/// Answers pruning questions directly for one segment of the execution snapshot.
///
/// A positive `can_match` answer retains the segment; it is not evidence that a match exists.
/// `matches_all` requires a guarantee for every document, including NULLs. Unsupported query
/// shapes provide neither an exclusion nor a match-all guarantee. Statistics errors propagate
/// through the snapshot as query errors.
pub(crate) struct SegmentPruner<'a> {
    snapshot: &'a SegmentStatsSnapshot,
    schema: &'a SearchIndexSchema,
    index_created_by_version: Option<Version>,
}

impl<'a> SegmentPruner<'a> {
    pub(crate) fn new(
        snapshot: &'a SegmentStatsSnapshot,
        schema: &'a SearchIndexSchema,
        index_created_by_version: Option<Version>,
    ) -> Self {
        Self {
            snapshot,
            schema,
            index_created_by_version,
        }
    }

    fn eligible_field(&self, field: &FieldName) -> Option<SearchField> {
        if field.path().is_some() {
            return None;
        }
        self.schema
            .search_field(field.root())
            .filter(SearchField::stats_order_matches_values)
    }

    fn eligible_value_field(&self, field: &FieldName) -> Option<SearchField> {
        self.eligible_field(field)
            .filter(SearchField::stats_describe_terms)
    }

    fn term_value(&self, field: &SearchField, value: &PdbOwnedValue) -> Option<PdbOwnedValue> {
        let value = convert_value_for_field(
            value.clone(),
            &field.field_type(),
            self.index_created_by_version,
        )
        .ok()?;
        value_preserves_term_order(field, &value).then_some(value)
    }

    fn range_bounds(
        &self,
        field: &SearchField,
        lower: &Bound<PdbOwnedValue>,
        upper: &Bound<PdbOwnedValue>,
    ) -> Option<(Bound<PdbOwnedValue>, Bound<PdbOwnedValue>)> {
        let (lower, upper) = canonicalize_range_bounds_for_field(
            field,
            self.index_created_by_version,
            lower.clone(),
            upper.clone(),
        )
        .ok()?;
        let preserves_order = [&lower, &upper].into_iter().all(|bound| match bound {
            Bound::Included(value) | Bound::Excluded(value) => {
                value_preserves_term_order(field, value)
            }
            Bound::Unbounded => true,
        });
        preserves_order.then_some((lower, upper))
    }

    // Grouped so each field is decoded once per segment check, not once per term.
    fn terms_by_field(terms: &[super::TermInput]) -> HashMap<&FieldName, Vec<&PdbOwnedValue>> {
        let mut grouped: HashMap<&FieldName, Vec<&PdbOwnedValue>> = HashMap::default();
        for term in terms {
            grouped.entry(&term.field).or_default().push(&term.value);
        }
        grouped
    }

    fn terms_can_match<'v>(
        &self,
        segment: SegmentOrdinal,
        field: &FieldName,
        mut terms: impl Iterator<Item = &'v PdbOwnedValue>,
    ) -> bool {
        let Some(field) = self.eligible_value_field(field) else {
            return true;
        };
        let stats = self.snapshot.empirical(segment as usize, &field);
        terms.any(|value| {
            self.term_value(&field, value)
                .is_none_or(|value| term_can_match(stats.as_ref(), &value))
        })
    }

    fn terms_match_all<'v>(
        &self,
        segment: SegmentOrdinal,
        field: &FieldName,
        mut terms: impl Iterator<Item = &'v PdbOwnedValue>,
    ) -> bool {
        let Some(field) = self.eligible_value_field(field) else {
            return false;
        };
        let stats = self.snapshot.empirical(segment as usize, &field);
        terms.any(|value| {
            self.term_value(&field, value)
                .is_some_and(|value| term_matches_all(stats.as_ref(), &value))
        })
    }

    fn field_can_match(
        &self,
        segment: SegmentOrdinal,
        field: &FieldName,
        query: &pdb::Query,
    ) -> bool {
        match query {
            pdb::Query::All => true,
            pdb::Query::Empty => false,
            pdb::Query::ScoreAdjusted { query, .. } => self.field_can_match(segment, field, query),
            pdb::Query::Term { value } => {
                self.terms_can_match(segment, field, std::iter::once(value))
            }
            pdb::Query::TermSet { terms } => {
                !terms.is_empty() && self.terms_can_match(segment, field, terms.iter())
            }
            pdb::Query::Range {
                lower_bound,
                upper_bound,
            } => {
                let Some(field) = self.eligible_value_field(field) else {
                    return true;
                };
                let Some((lower, upper)) = self.range_bounds(&field, lower_bound, upper_bound)
                else {
                    return true;
                };
                range_can_match(
                    self.snapshot.empirical(segment as usize, &field).as_ref(),
                    &lower,
                    &upper,
                )
            }
            _ => true,
        }
    }

    fn field_matches_all(
        &self,
        segment: SegmentOrdinal,
        field: &FieldName,
        query: &pdb::Query,
    ) -> bool {
        match query {
            pdb::Query::All => true,
            pdb::Query::Empty => false,
            pdb::Query::ScoreAdjusted { query, .. } => {
                self.field_matches_all(segment, field, query)
            }
            pdb::Query::Exists => self
                .eligible_field(field)
                .and_then(|field| self.snapshot.empirical(segment as usize, &field))
                .is_some_and(|stats| !stats.nullable),
            pdb::Query::Term { value } => {
                self.terms_match_all(segment, field, std::iter::once(value))
            }
            pdb::Query::TermSet { terms } => {
                !terms.is_empty() && self.terms_match_all(segment, field, terms.iter())
            }
            pdb::Query::Range {
                lower_bound,
                upper_bound,
            } => {
                let Some(field) = self.eligible_value_field(field) else {
                    return false;
                };
                let Some((lower, upper)) = self.range_bounds(&field, lower_bound, upper_bound)
                else {
                    return false;
                };
                range_matches_all(
                    self.snapshot.empirical(segment as usize, &field).as_ref(),
                    &lower,
                    &upper,
                )
            }
            _ => false,
        }
    }

    /// False proves that this segment cannot contribute a matching document.
    pub(crate) fn can_match(&self, segment: SegmentOrdinal, query: &SearchQueryInput) -> bool {
        match query {
            SearchQueryInput::All => true,
            SearchQueryInput::Empty => false,
            SearchQueryInput::FieldedQuery { field, query } => {
                self.field_can_match(segment, field, query)
            }
            SearchQueryInput::TermSet { terms } => Self::terms_by_field(terms)
                .into_iter()
                .any(|(field, values)| self.terms_can_match(segment, field, values.into_iter())),
            SearchQueryInput::Boolean {
                must,
                should,
                must_not,
                minimum_should_match,
            } => boolean_can_match(
                must.iter().map(|q| self.can_match(segment, q)),
                should.iter().map(|q| self.can_match(segment, q)),
                must_not.iter().map(|q| self.matches_all(segment, q)),
                *minimum_should_match,
            ),
            SearchQueryInput::Boost { query, .. }
            | SearchQueryInput::ConstScore { query, .. }
            | SearchQueryInput::WithIndex { query, .. } => self.can_match(segment, query),
            SearchQueryInput::DisjunctionMax { disjuncts, .. } => {
                disjuncts.iter().any(|q| self.can_match(segment, q))
            }
            SearchQueryInput::ScoreFilter { query, .. } => {
                query.as_deref().is_none_or(|q| self.can_match(segment, q))
            }
            SearchQueryInput::HeapFilter { indexed_query, .. } => {
                self.can_match(segment, indexed_query)
            }
            _ => true,
        }
    }

    /// True proves every document satisfies the query. Used for negation and redundant internal
    /// filters; ordinary user clauses still run through the original compiled query.
    pub(crate) fn matches_all(&self, segment: SegmentOrdinal, query: &SearchQueryInput) -> bool {
        match query {
            SearchQueryInput::All => true,
            SearchQueryInput::Empty => false,
            SearchQueryInput::FieldedQuery { field, query } => {
                self.field_matches_all(segment, field, query)
            }
            SearchQueryInput::TermSet { terms } => Self::terms_by_field(terms)
                .into_iter()
                .any(|(field, values)| self.terms_match_all(segment, field, values.into_iter())),
            SearchQueryInput::Boolean {
                must,
                should,
                must_not,
                minimum_should_match,
            } => boolean_matches_all(
                must.iter().map(|q| self.matches_all(segment, q)),
                should.iter().map(|q| self.matches_all(segment, q)),
                must_not.iter().map(|q| self.can_match(segment, q)),
                *minimum_should_match,
            ),
            SearchQueryInput::Boost { query, .. }
            | SearchQueryInput::ConstScore { query, .. }
            | SearchQueryInput::WithIndex { query, .. } => self.matches_all(segment, query),
            SearchQueryInput::DisjunctionMax { disjuncts, .. } => {
                disjuncts.iter().any(|q| self.matches_all(segment, q))
            }
            // Score and heap filters may reject any row that matches their indexed child.
            _ => false,
        }
    }
}

/// `value_to_term` casts unsigned inputs on I64 fields with `as i64`; overflowing values wrap
/// into negative terms. Numerical bounds cannot prove what those terms match.
fn value_preserves_term_order(field: &SearchField, value: &PdbOwnedValue) -> bool {
    !matches!((field.field_entry().field_type(), value),
        (FieldType::I64(_), PdbOwnedValue::U64(value)) if i64::try_from(*value).is_err())
}

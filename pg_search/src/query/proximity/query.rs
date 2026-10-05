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

use crate::query::proximity::weight::ProximityWeight;
use crate::query::proximity::{ProximityClause, ProximityDistance, WhichTerms};
use tantivy::query::{
    Bm25Weight, EnableScoring, Query, QueryEstimate, RegexQuery, TermQuery, Weight,
};
use tantivy::schema::{Field, IndexRecordOption};
use tantivy::{SegmentReader, TantivyError, Term};

#[derive(Debug, Clone)]
pub struct ProximityQuery {
    field: Field,
    left: ProximityClause,
    distance: ProximityDistance,
    right: ProximityClause,
}

impl ProximityQuery {
    pub fn new(
        field: Field,
        left: ProximityClause,
        distance: ProximityDistance,
        right: ProximityClause,
    ) -> Self {
        Self {
            field,
            left,
            distance,
            right,
        }
    }

    pub fn field(&self) -> Field {
        self.field
    }

    pub fn left(&self) -> &ProximityClause {
        &self.left
    }

    pub fn right(&self) -> &ProximityClause {
        &self.right
    }

    pub fn terms<'a>(
        &'a self,
        field: Field,
        segment_reader: Option<&'a SegmentReader>,
    ) -> tantivy::Result<impl Iterator<Item = Term> + 'a> {
        Ok(self
            .left
            .terms(field, segment_reader, WhichTerms::All)?
            .chain(self.right.terms(field, segment_reader, WhichTerms::All)?)
            .map(|t| Term::from_field_text(self.field, t.as_str())))
    }

    pub fn distance(&self) -> ProximityDistance {
        self.distance
    }
}

impl QueryEstimate for ProximityQuery {
    /// Use the rarer side's document count, ignoring distance and order. Read only term metadata.
    fn estimate_docs(&self, reader: &SegmentReader) -> tantivy::Result<Option<(u32, u64)>> {
        self.left
            .estimate_proximity(&self.right, self.field, reader)
    }
}

impl ProximityClause {
    fn estimate_proximity(
        &self,
        right: &Self,
        field: Field,
        reader: &SegmentReader,
    ) -> tantivy::Result<Option<(u32, u64)>> {
        let left = self.estimate_docs(field, reader)?;
        let right = right.estimate_docs(field, reader)?;
        Ok(left
            .zip(right)
            .map(|((left, left_cost), (right, right_cost))| {
                let count = left.min(right);
                (
                    count,
                    left_cost
                        .saturating_add(right_cost)
                        .saturating_add(u64::from(count)),
                )
            }))
    }

    /// Add alternative counts, capped at the segment size; nested proximity uses its rarer side.
    fn estimate_docs(
        &self,
        field: Field,
        reader: &SegmentReader,
    ) -> tantivy::Result<Option<(u32, u64)>> {
        match self {
            Self::Uninitialized => Ok(Some((0, 0))),
            Self::Term(term) => {
                TermQuery::new(Term::from_field_text(field, term), IndexRecordOption::Basic)
                    .estimate_docs(reader)
            }
            Self::Regex { pattern, .. } => {
                RegexQuery::from_pattern(pattern.as_str(), field)?.estimate_docs(reader)
            }
            Self::Clauses(clauses) => {
                let (mut count, mut cost) = (0u32, 0u64);
                for clause in clauses {
                    let Some((child_count, child_cost)) = clause.estimate_docs(field, reader)?
                    else {
                        return Ok(None);
                    };
                    count = count.saturating_add(child_count).min(reader.max_doc());
                    cost = cost.saturating_add(child_cost);
                }
                Ok(Some((count, cost)))
            }
            Self::Proximity { left, right, .. } => left.estimate_proximity(right, field, reader),
        }
    }
}

impl Query for ProximityQuery {
    fn weight(&self, enable_scoring: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
        let schema = enable_scoring.schema();
        let field_entry = schema.get_field_entry(self.field);
        let has_positions = field_entry
            .field_type()
            .get_index_record_option()
            .map(IndexRecordOption::has_positions)
            .unwrap_or(false);
        if !has_positions {
            let field_name = field_entry.name();
            return Err(TantivyError::SchemaError(format!(
                "proximity queries require fields indexed with positions.  `{field_name:?}` does not have positions."
            )));
        }

        let terms = self.terms(self.field, None)?.collect::<Vec<_>>();
        let bm25_weight_opt = match enable_scoring {
            EnableScoring::Enabled {
                statistics_provider,
                ..
            } => Some(Bm25Weight::for_terms(statistics_provider, &terms)?),
            EnableScoring::Disabled { .. } => None,
        };

        let weight = ProximityWeight::new(self.clone(), bm25_weight_opt);
        Ok(Box::new(weight))
    }

    fn query_terms(
        &self,
        _field: Field,
        segment_reader: &SegmentReader,
        visitor: &mut dyn FnMut(&Term, bool),
    ) {
        // These terms live on the field this query was built for, the one `weight()` resolves
        // against. Taking the caller's field instead would stream a regex clause over an
        // unrelated dictionary and hand back terms that belong to nobody.
        let Ok(terms) = self.terms(self.field, Some(segment_reader)) else {
            return;
        };
        for term in terms {
            visitor(&term, true)
        }
    }
}

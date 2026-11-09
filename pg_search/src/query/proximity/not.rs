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

use crate::query::proximity::spans::SpanSource;
use crate::query::proximity::{ProximityClause, WhichTerms};
use tantivy::fieldnorm::FieldNormReader;
use tantivy::query::{
    Bm25Weight, EmptyScorer, EnableScoring, Explanation, Query, Scorer, Weight, does_not_match,
};
use tantivy::schema::Field;
use tantivy::{DocId, DocSet, Score, SegmentReader, TERMINATED, Term};

/// A proximity clause containing a `ProximityClause::Not`, evaluated on spans end to end: the
/// flattened positions of `ProximityQuery` lose where a nested match starts and ends.
#[derive(Debug, Clone)]
pub struct ProximityNotQuery {
    field: Field,
    clause: ProximityClause,
}

impl ProximityNotQuery {
    pub fn new(field: Field, clause: ProximityClause) -> Self {
        Self { field, clause }
    }

    /// Only the `include` side of a `Not` is scored and highlighted; see `ProximityClause::terms`.
    fn terms<'a>(
        &'a self,
        segment_reader: Option<&'a SegmentReader>,
    ) -> tantivy::Result<impl Iterator<Item = Term> + 'a> {
        Ok(self
            .clause
            .terms(self.field, segment_reader, WhichTerms::All)?
            .map(|t| Term::from_field_text(self.field, t.as_str())))
    }
}

impl Query for ProximityNotQuery {
    fn weight(&self, enable_scoring: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
        let bm25_weight_opt = match enable_scoring {
            EnableScoring::Enabled {
                statistics_provider,
                ..
            } => {
                let terms = self.terms(None)?.collect::<Vec<_>>();
                Some(Bm25Weight::for_terms(statistics_provider, &terms)?)
            }
            EnableScoring::Disabled { .. } => None,
        };
        Ok(Box::new(ProximityNotWeight {
            query: self.clone(),
            weight_opt: bm25_weight_opt,
        }))
    }

    fn query_terms(
        &self,
        _field: Field,
        segment_reader: &SegmentReader,
        visitor: &mut dyn FnMut(&Term, bool),
    ) {
        let Ok(terms) = self.terms(Some(segment_reader)) else {
            return;
        };
        for term in terms {
            visitor(&term, true)
        }
    }
}

struct ProximityNotWeight {
    query: ProximityNotQuery,
    weight_opt: Option<Bm25Weight>,
}

impl ProximityNotWeight {
    fn not_scorer(
        &self,
        reader: &SegmentReader,
        boost: Score,
    ) -> tantivy::Result<Option<ProximityNotScorer>> {
        let field = self.query.field;
        let source = SpanSource::build(field, reader, &self.query.clause)?;
        if source.doc() == TERMINATED {
            return Ok(None);
        }
        let fieldnorm_reader = if self.weight_opt.is_some() {
            reader.scoring_fieldnorm_reader(field)?
        } else {
            FieldNormReader::constant(reader.max_doc(), 1)
        };
        Ok(Some(ProximityNotScorer {
            source,
            fieldnorm_reader,
            weight_opt: self
                .weight_opt
                .as_ref()
                .map(|bm25_weight| bm25_weight.boost_by(boost)),
        }))
    }
}

impl Weight for ProximityNotWeight {
    fn scorer(&self, reader: &SegmentReader, boost: Score) -> tantivy::Result<Box<dyn Scorer>> {
        Ok(self
            .not_scorer(reader, boost)?
            .map(|scorer| Box::new(scorer) as Box<dyn Scorer>)
            .unwrap_or_else(|| Box::new(EmptyScorer) as Box<dyn Scorer>))
    }

    fn explain(&self, reader: &SegmentReader, doc: DocId) -> tantivy::Result<Explanation> {
        let Some(mut scorer) = self.not_scorer(reader, 1.0)? else {
            return Err(does_not_match(doc));
        };
        if scorer.seek(doc) != doc {
            return Err(does_not_match(doc));
        }
        let mut explanation = Explanation::new("Proximity Not Scorer", scorer.score());
        if let Some(similarity_weight) = self.weight_opt.as_ref() {
            let nmatches = scorer.source.spans().len() as u32;
            explanation.add_detail(similarity_weight.explain(scorer.fieldnorm_id(), nmatches));
        }
        Ok(explanation)
    }
}

struct ProximityNotScorer {
    source: SpanSource,
    fieldnorm_reader: FieldNormReader,
    weight_opt: Option<Bm25Weight>,
}

impl ProximityNotScorer {
    fn fieldnorm_id(&self) -> u8 {
        self.source
            .fieldnorm_id()
            .unwrap_or_else(|| self.fieldnorm_reader.fieldnorm_id(self.doc()))
    }
}

impl DocSet for ProximityNotScorer {
    fn advance(&mut self) -> DocId {
        self.source.advance()
    }

    fn seek(&mut self, target: DocId) -> DocId {
        self.source.seek(target)
    }

    fn doc(&self) -> DocId {
        self.source.doc()
    }

    fn size_hint(&self) -> u32 {
        self.source.size_hint()
    }
}

impl Scorer for ProximityNotScorer {
    fn score(&mut self) -> Score {
        match self.weight_opt.as_ref() {
            Some(similarity_weight) => {
                let nmatches = self.source.spans().len() as u32;
                similarity_weight.score(self.fieldnorm_id(), nmatches)
            }
            None => 1.0,
        }
    }
}

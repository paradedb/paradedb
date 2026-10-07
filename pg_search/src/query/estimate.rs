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

use super::{SearchQueryInput, pdb};
use tantivy::SegmentReader;
use tantivy::query::{BooleanQuery, EmptyQuery, Occur, PhraseQuery, Query, TermQuery};

pub(crate) fn supports(query: &SearchQueryInput) -> bool {
    match query {
        SearchQueryInput::Boolean {
            must,
            should,
            must_not,
            ..
        } => must.iter().chain(should).chain(must_not).all(supports),
        SearchQueryInput::WithIndex { query, .. } => supports(query),
        SearchQueryInput::FieldedQuery { query, .. } => matches!(
            query,
            pdb::Query::Term { .. }
                | pdb::Query::Match {
                    distance: None | Some(0),
                    ..
                }
                | pdb::Query::MatchArray {
                    distance: None | Some(0),
                    ..
                }
                | pdb::Query::Phrase { .. }
                | pdb::Query::PhraseArray { .. }
                | pdb::Query::TokenizedPhrase { .. }
        ),
        _ => false,
    }
}

/// Returns live-document and traversal estimates without opening a scorer.
pub(crate) fn estimate_docs(query: &dyn Query, reader: &SegmentReader) -> Option<(u64, u64)> {
    let estimate = estimate(query, reader).ok()??;
    let live_fraction = reader.num_docs() as f64 / reader.max_doc().max(1) as f64;
    Some((
        (estimate.matches * live_fraction).ceil() as u64,
        estimate.cost,
    ))
}

struct Estimate {
    matches: f64,
    cost: u64,
}

fn estimate(query: &dyn Query, reader: &SegmentReader) -> tantivy::Result<Option<Estimate>> {
    let total = reader.max_doc().max(1) as f64;
    if let Some(query) = query.downcast_ref::<TermQuery>() {
        let term = query.term();
        if !reader.schema().get_field_entry(term.field()).is_indexed() {
            return Ok(None);
        }
        let count = reader.inverted_index(term.field())?.doc_freq(term)?;
        return Ok(Some(Estimate {
            matches: count as f64,
            cost: count as u64,
        }));
    }
    if let Some(query) = query.downcast_ref::<PhraseQuery>() {
        let terms = query.phrase_terms();
        let mut fraction = 1.0;
        let mut shortest = u64::MAX;
        for term in &terms {
            let count = reader.inverted_index(term.field())?.doc_freq(term)?;
            fraction *= count as f64 / total;
            shortest = shortest.min(count as u64);
        }
        // Match Tantivy's positional discount and retain the posting-list cost floor.
        let checks = (10 * terms.len()) as f64;
        return Ok(Some(Estimate {
            matches: fraction * total / checks,
            cost: ((fraction * total * checks).ceil() as u64).max(shortest),
        }));
    }
    if let Some(query) = query.downcast_ref::<BooleanQuery>() {
        if let [(Occur::Must | Occur::Should, child)] = query.clauses() {
            return estimate(child.as_ref(), reader);
        }
        let mut required = 1.0;
        let mut excluded = 1.0;
        let mut optional = Vec::new();
        let mut required_cost: Option<u64> = None;
        let mut optional_cost = 0u64;
        for (occur, child) in query.clauses() {
            let Some(child) = estimate(child.as_ref(), reader)? else {
                return Ok(None);
            };
            match occur {
                Occur::Must => {
                    required *= child.matches / total;
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
        let minimum = query
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
        return Ok(Some(Estimate {
            matches: (required * optional_fraction * excluded).clamp(0.0, 1.0) * total,
            cost: required_cost.unwrap_or(optional_cost),
        }));
    }
    // Tokenization can turn an eligible match or phrase into an empty query.
    if query.is::<EmptyQuery>() {
        return Ok(Some(Estimate {
            matches: 0.0,
            cost: 0,
        }));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::query::{AllQuery, RegexQuery};
    use tantivy::schema::{IndexRecordOption, Schema, TEXT};
    use tantivy::{Index, TantivyDocument, Term, doc};

    #[test]
    fn statistics_estimates_preserve_term_document_frequency() {
        let mut schema = Schema::builder();
        let field = schema.add_text_field("body", TEXT);
        let index = Index::create_in_ram(schema.build());
        let mut writer = index
            .writer_with_num_threads::<TantivyDocument>(1, 50_000_000)
            .unwrap();
        for n in 0..100 {
            let value = if n < 7 { "rare" } else { "common" };
            writer.add_document(doc!(field => value)).unwrap();
        }
        writer.commit().unwrap();
        let searcher = index.reader().unwrap().searcher();
        let query = TermQuery::new(
            Term::from_field_text(field, "rare"),
            IndexRecordOption::Basic,
        );
        assert_eq!(
            estimate_docs(&query, searcher.segment_reader(0)),
            Some((7, 7))
        );
    }

    #[test]
    fn statistics_estimates_terms_phrases_and_booleans() {
        let mut schema = Schema::builder();
        let field = schema.add_text_field("body", TEXT);
        let index = Index::create_in_ram(schema.build());
        let mut writer = index
            .writer_with_num_threads::<TantivyDocument>(1, 50_000_000)
            .unwrap();
        for _ in 0..16 {
            for value in ["alpha beta", "alpha delta", "gamma beta", "gamma delta"] {
                writer.add_document(doc!(field => value)).unwrap();
            }
        }
        writer.commit().unwrap();
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let segment = searcher.segment_reader(0);
        let term = |value| -> Box<dyn Query> {
            Box::new(TermQuery::new(
                Term::from_field_text(field, value),
                IndexRecordOption::Basic,
            ))
        };
        let count = |query: &dyn Query| estimate_docs(query, segment).unwrap().0;
        assert_eq!(
            estimate_docs(term("alpha").as_ref(), segment),
            Some((32, 32))
        );
        assert_eq!(
            estimate_docs(term("absent").as_ref(), segment),
            Some((0, 0))
        );
        let phrase = PhraseQuery::new(vec![
            Term::from_field_text(field, "alpha"),
            Term::from_field_text(field, "beta"),
        ]);
        assert_eq!(estimate_docs(&phrase, segment), Some((1, 320)));
        assert_eq!(
            count(&BooleanQuery::intersection(vec![
                term("alpha"),
                term("beta")
            ])),
            16
        );
        assert_eq!(
            count(&BooleanQuery::union(vec![term("alpha"), term("beta")])),
            48
        );
        assert_eq!(
            count(&BooleanQuery::new(vec![
                (Occur::Must, term("alpha")),
                (Occur::Should, term("beta"))
            ])),
            32
        );
        assert_eq!(
            count(&BooleanQuery::new(vec![
                (Occur::Must, term("alpha")),
                (Occur::MustNot, term("beta"))
            ])),
            16
        );
        assert_eq!(
            count(&BooleanQuery::new(vec![(Occur::MustNot, term("alpha"))])),
            0
        );
        assert_eq!(count(&BooleanQuery::new(vec![])), 0);
        for (minimum, expected) in [(0, 56), (1, 56), (2, 32), (3, 8), (4, 0)] {
            assert_eq!(
                count(&BooleanQuery::union_with_minimum_required_clauses(
                    vec![term("alpha"), term("beta"), term("gamma")],
                    minimum
                )),
                expected
            );
        }
        for occur in [Occur::Must, Occur::Should] {
            assert_eq!(
                count(&BooleanQuery::with_minimum_required_clauses(
                    vec![(occur, term("alpha"))],
                    2
                )),
                32
            );
        }
        let nested = BooleanQuery::intersection(vec![
            Box::new(BooleanQuery::union(vec![term("alpha"), term("beta")])),
            term("gamma"),
        ]);
        assert_eq!(count(&nested), 24);
        let mixed = BooleanQuery::intersection(vec![
            term("alpha"),
            Box::new(RegexQuery::from_pattern("b.*", field).unwrap()),
        ]);
        assert_eq!(estimate_docs(&mixed, segment), None);
        assert_eq!(estimate_docs(&AllQuery, segment), None);
        assert_eq!(estimate_docs(&EmptyQuery, segment), Some((0, 0)));

        writer.delete_term(Term::from_field_text(field, "gamma"));
        writer.commit().unwrap();
        reader.reload().unwrap();
        let searcher = reader.searcher();
        let segment = searcher.segment_reader(0);
        assert_eq!(segment.num_docs(), 32);
        assert_eq!(
            estimate_docs(term("alpha").as_ref(), segment),
            Some((16, 32))
        );
    }
}

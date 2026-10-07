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

use tantivy::SegmentReader;
use tantivy::query::{
    AllQuery, BooleanQuery, BoostQuery, ConstScoreQuery, EmptyQuery, Occur, PhraseQuery, Query,
    TermQuery,
};

pub(crate) trait QueryEstimate {
    fn estimate_docs(&self, reader: &SegmentReader) -> tantivy::Result<Option<Estimate>>;
}

pub(crate) struct Estimate {
    matches: f64,
    pub(crate) cost: u64,
}

impl Estimate {
    pub(crate) fn live_docs(&self, reader: &SegmentReader) -> u64 {
        let live_fraction = reader.num_docs() as f64 / reader.max_doc().max(1) as f64;
        ((self.matches * live_fraction).ceil() as u64).min(reader.num_docs() as u64)
    }
}

impl QueryEstimate for dyn Query {
    fn estimate_docs(&self, reader: &SegmentReader) -> tantivy::Result<Option<Estimate>> {
        // Exact match queries become terms joined by AND or OR, so they use the same estimates.
        macro_rules! estimate_as {
            ($($query:ty),* $(,)?) => {$(
                if let Some(query) = self.downcast_ref::<$query>() {
                    return query.estimate_docs(reader);
                }
            )*};
        }
        estimate_as! {
            AllQuery, TermQuery, PhraseQuery, BooleanQuery, EmptyQuery, ConstScoreQuery, BoostQuery
        }
        if let Some(query) = self.downcast_ref::<ConstScoreQuery<AllQuery>>() {
            return query.query().estimate_docs(reader);
        }
        Ok(None)
    }
}

impl QueryEstimate for AllQuery {
    fn estimate_docs(&self, reader: &SegmentReader) -> tantivy::Result<Option<Estimate>> {
        Ok(Some(Estimate {
            matches: reader.max_doc() as f64,
            cost: reader.max_doc() as u64,
        }))
    }
}

impl QueryEstimate for ConstScoreQuery {
    fn estimate_docs(&self, reader: &SegmentReader) -> tantivy::Result<Option<Estimate>> {
        self.query().estimate_docs(reader)
    }
}

impl QueryEstimate for BoostQuery {
    fn estimate_docs(&self, reader: &SegmentReader) -> tantivy::Result<Option<Estimate>> {
        self.query().estimate_docs(reader)
    }
}

impl QueryEstimate for TermQuery {
    fn estimate_docs(&self, reader: &SegmentReader) -> tantivy::Result<Option<Estimate>> {
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

impl QueryEstimate for PhraseQuery {
    fn estimate_docs(&self, reader: &SegmentReader) -> tantivy::Result<Option<Estimate>> {
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

impl QueryEstimate for BooleanQuery {
    fn estimate_docs(&self, reader: &SegmentReader) -> tantivy::Result<Option<Estimate>> {
        // AND starts with the smallest estimate; each extra condition reduces it less.
        // OR estimates how many documents match enough conditions.
        // NOT removes the estimated share of documents matching the excluded conditions.
        if let [(Occur::Must | Occur::Should, child)] = self.clauses() {
            return child.estimate_docs(reader);
        }
        let total = reader.max_doc().max(1) as f64;
        let mut required = Vec::new();
        let mut excluded = 1.0;
        let mut optional = Vec::new();
        let mut required_cost: Option<u64> = None;
        let mut optional_cost = 0u64;
        for (occur, child) in self.clauses() {
            let Some(child) = child.estimate_docs(reader)? else {
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
    fn estimate_docs(&self, _reader: &SegmentReader) -> tantivy::Result<Option<Estimate>> {
        Ok(Some(Estimate {
            matches: 0.0,
            cost: 0,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::query::{QueryClone, RegexQuery};
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
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let query = TermQuery::new(
            Term::from_field_text(field, "rare"),
            IndexRecordOption::Basic,
        );
        let segment = searcher.segment_reader(0);
        let estimate = query.estimate_docs(segment).unwrap().unwrap();
        assert_eq!(estimate.live_docs(segment), 7);
        assert_eq!(estimate.cost, 7);
        for clauses in [
            vec![query.box_clone(), Box::new(AllQuery)],
            vec![Box::new(AllQuery), query.box_clone()],
        ] {
            let estimate = BooleanQuery::intersection(clauses)
                .estimate_docs(segment)
                .unwrap()
                .unwrap();
            assert_eq!(estimate.matches, 7.0);
            assert_eq!(estimate.live_docs(segment), 7);
            assert_eq!(estimate.cost, 7);
        }

        writer.delete_term(Term::from_field_text(field, "common"));
        writer.commit().unwrap();
        reader.reload().unwrap();
        let searcher = reader.searcher();
        let segment = searcher.segment_reader(0);
        let estimate = AllQuery.estimate_docs(segment).unwrap().unwrap();
        assert_eq!(estimate.live_docs(segment), 7);
        assert_eq!(estimate.cost, 100);
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
        let estimate = |query: &dyn Query, segment: &SegmentReader| {
            query
                .estimate_docs(segment)
                .unwrap()
                .map(|estimate| (estimate.live_docs(segment), estimate.cost))
        };
        let count = |query: &dyn Query| estimate(query, segment).unwrap().0;
        assert_eq!(estimate(term("alpha").as_ref(), segment), Some((32, 32)));
        assert_eq!(estimate(term("absent").as_ref(), segment), Some((0, 0)));
        let phrase = PhraseQuery::new(vec![
            Term::from_field_text(field, "alpha"),
            Term::from_field_text(field, "beta"),
        ]);
        assert_eq!(estimate(&phrase, segment), Some((1, 320)));
        for (terms, expected) in [
            (vec!["alpha", "beta"], 23),
            (vec!["alpha", "beta", "gamma"], 20),
            (vec!["alpha", "beta", "gamma", "delta"], 18),
        ] {
            let query = BooleanQuery::intersection(terms.into_iter().map(term).collect());
            assert_eq!(estimate(&query, segment), Some((expected, 32)));
        }
        for terms in [["alpha", "absent"], ["absent", "alpha"]] {
            let query = BooleanQuery::intersection(terms.into_iter().map(term).collect());
            assert_eq!(estimate(&query, segment), Some((0, 0)));
        }
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
        let mut nested = vec![
            Box::new(BooleanQuery::union(vec![term("alpha"), term("beta")])),
            term("gamma"),
        ];
        assert_eq!(
            estimate(&BooleanQuery::intersection(nested.clone()), segment),
            Some((28, 32))
        );
        nested.reverse();
        assert_eq!(
            estimate(&BooleanQuery::intersection(nested), segment),
            Some((28, 32))
        );
        assert_eq!(
            count(&BooleanQuery::with_minimum_required_clauses(
                vec![
                    (Occur::Must, term("alpha")),
                    (Occur::Must, term("beta")),
                    (Occur::Should, term("gamma")),
                    (Occur::MustNot, term("delta")),
                ],
                1
            )),
            6
        );
        let mixed = BooleanQuery::intersection(vec![
            term("alpha"),
            Box::new(RegexQuery::from_pattern("b.*", field).unwrap()),
        ]);
        assert_eq!(estimate(&mixed, segment), None);
        assert_eq!(estimate(&AllQuery, segment), Some((64, 64)));
        assert_eq!(
            estimate(&ConstScoreQuery::new(AllQuery, 0.0), segment),
            Some((64, 64))
        );
        assert_eq!(
            count(&BooleanQuery::intersection(vec![
                Box::new(AllQuery),
                term("alpha")
            ])),
            32
        );
        assert_eq!(estimate(&EmptyQuery, segment), Some((0, 0)));

        writer.delete_term(Term::from_field_text(field, "gamma"));
        writer.commit().unwrap();
        reader.reload().unwrap();
        let searcher = reader.searcher();
        let segment = searcher.segment_reader(0);
        assert_eq!(segment.num_docs(), 32);
        assert_eq!(estimate(&AllQuery, segment), Some((32, 64)));
        assert_eq!(estimate(term("alpha").as_ref(), segment), Some((16, 32)));
    }
}

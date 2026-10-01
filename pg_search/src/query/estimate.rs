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

use crate::index::reader::index::{DocsEstimate, SearchIndexReader};
use pgrx::{PgBox, PgList, PgMemoryContexts, pg_sys};
use std::ops::Bound;
use std::ptr::null_mut;
use tantivy::query::{
    BooleanQuery, BoostQuery, ConstScoreQuery, DisjunctionMaxQuery, Occur, Query, QueryEstimate,
    TermSetQuery,
};

pub(crate) trait EstimateDocs {
    fn estimate_docs(&self, reader: &SearchIndexReader) -> DocsEstimate;
}

#[derive(Clone, Debug)]
pub(crate) struct EstimatedQuery(pub f64);

impl QueryEstimate for EstimatedQuery {
    fn estimate_docs(&self, _: &tantivy::SegmentReader) -> tantivy::Result<Option<(u32, u64)>> {
        // This is a caller-provided selectivity, not a segment metadata estimate.
        Ok(None)
    }
}

impl Query for EstimatedQuery {
    fn weight(
        &self,
        _: tantivy::query::EnableScoring<'_>,
    ) -> tantivy::Result<Box<dyn tantivy::query::Weight>> {
        panic!("an estimation query must never be executed")
    }
}

fn result(reader: &SearchIndexReader, selectivity: f64, cost: f64) -> DocsEstimate {
    let total_docs = reader.total_docs();
    let selectivity = selectivity.clamp(0.0, 1.0);
    DocsEstimate {
        selectivity,
        matching_docs: (selectivity * total_docs as f64).ceil() as usize,
        total_docs,
        query_cost: cost.ceil() as u64,
    }
}

fn fallback(reader: &SearchIndexReader, selectivity: f64) -> DocsEstimate {
    result(reader, selectivity, reader.total_docs() as f64)
}

impl EstimateDocs for dyn Query + '_ {
    /// Uses Tantivy's text estimates and PostgreSQL's Boolean selectivity rules.
    fn estimate_docs(&self, reader: &SearchIndexReader) -> DocsEstimate {
        let recurse = |q: &dyn Query| EstimateDocs::estimate_docs(q, reader);
        if let Some(q) = self.downcast_ref::<EstimatedQuery>() {
            return fallback(reader, q.0);
        }
        if let Some(q) = self.downcast_ref::<super::score::ScoreFilter>() {
            let scores = combine(
                reader,
                pg_sys::BoolExprType::OR_EXPR,
                q.bounds
                    .iter()
                    .map(|(lo, hi)| {
                        result(
                            reader,
                            if matches!((lo, hi), (Bound::Unbounded, Bound::Unbounded)) {
                                1.0
                            } else {
                                range_prior(lo, hi)
                            },
                            0.0,
                        )
                    })
                    .collect(),
            );
            return combine(
                reader,
                pg_sys::BoolExprType::AND_EXPR,
                vec![recurse(q.query.as_ref()), scores],
            );
        }

        if let Some(q) = self.downcast_ref::<Box<dyn Query>>() {
            return recurse(q.as_ref());
        }
        if let Some(q) = self.downcast_ref::<BoostQuery>() {
            return recurse(q.query());
        }
        if let Some(q) = self.downcast_ref::<ConstScoreQuery>() {
            return recurse(q.query());
        }
        if let Some(q) = self.downcast_ref::<BooleanQuery>() {
            let mut must = vec![];
            let mut should = vec![];
            let mut must_not = vec![];
            for (occur, query) in q.clauses() {
                match occur {
                    Occur::Must => &mut must,
                    Occur::Should => &mut should,
                    Occur::MustNot => &mut must_not,
                }
                .push(recurse(query.as_ref()));
            }
            return boolean_estimate(
                reader,
                must,
                should,
                must_not,
                q.get_minimum_number_should_match(),
            );
        }
        if let Some(q) = self.downcast_ref::<DisjunctionMaxQuery>() {
            return combine(
                reader,
                pg_sys::BoolExprType::OR_EXPR,
                q.disjuncts().iter().map(|q| recurse(q.as_ref())).collect(),
            );
        }
        if let Some(q) = self.downcast_ref::<TermSetQuery>() {
            return combine(
                reader,
                pg_sys::BoolExprType::OR_EXPR,
                q.terms()
                    .map(|t| {
                        recurse(&tantivy::query::TermQuery::new(
                            t.clone(),
                            tantivy::schema::IndexRecordOption::Basic,
                        ))
                    })
                    .collect(),
            );
        }
        if self.is::<tantivy::query::AllQuery>() {
            return fallback(reader, 1.0);
        }
        if self.is::<tantivy::query::EmptyQuery>() {
            return result(reader, 0.0, 0.0);
        }
        if let Some(q) = self.downcast_ref::<tantivy::query::RangeQuery>() {
            let (lo, hi) = q.bounds();
            return fallback(reader, range_prior(lo, hi));
        }
        if self.is::<tantivy::query::ExistsQuery>() {
            return fallback(reader, pg_sys::DEFAULT_NOT_UNK_SEL);
        }
        segment_estimate(self, reader)
    }
}

fn segment_estimate(
    query: &(impl QueryEstimate + ?Sized),
    reader: &SearchIndexReader,
) -> DocsEstimate {
    let mut count = 0.0;
    let mut cost = 0.0;
    let mut covered = 0u64;
    for segment in reader.searcher().segment_readers() {
        let live = u64::from(segment.num_docs());
        covered += live;
        match query.estimate_docs(segment).ok().flatten() {
            Some((docs, work)) => {
                count += f64::from(docs) / f64::from(segment.max_doc().max(1)) * live as f64;
                cost += work as f64;
            }
            None => {
                count += live as f64 * pg_sys::DEFAULT_MATCH_SEL;
                cost += live as f64;
            }
        }
    }
    let missing = reader.total_docs().saturating_sub(covered) as f64;
    count += missing * pg_sys::DEFAULT_MATCH_SEL;
    cost += missing;
    let total = reader.total_docs().max(1) as f64;
    result(reader, count / total, cost)
}

fn combine(
    reader: &SearchIndexReader,
    op: pg_sys::BoolExprType::Type,
    parts: Vec<DocsEstimate>,
) -> DocsEstimate {
    let mut memory = PgMemoryContexts::new("pg_search selectivity");
    let sel = unsafe {
        memory.switch_to(|_| {
            let mut args = PgList::<pg_sys::Node>::new();
            for part in &parts {
                let mut info =
                    PgBox::<pg_sys::RestrictInfo>::alloc_node(pg_sys::NodeTag::T_RestrictInfo);
                info.clause = pg_sys::makeBoolConst(true, false).cast();
                info.norm_selec = part.selectivity;
                info.outer_selec = part.selectivity;
                args.push(info.into_pg().cast());
            }
            let clause = pg_sys::makeBoolExpr(op, args.into_pg(), -1);
            pg_sys::clause_selectivity(
                null_mut(),
                clause.cast(),
                0,
                pg_sys::JoinType::JOIN_INNER,
                null_mut(),
            )
        })
    };
    result(reader, sel, parts.iter().map(|e| e.query_cost as f64).sum())
}

fn boolean_estimate(
    reader: &SearchIndexReader,
    mut must: Vec<DocsEstimate>,
    should: Vec<DocsEstimate>,
    must_not: Vec<DocsEstimate>,
    minimum: usize,
) -> DocsEstimate {
    if must.is_empty() && should.is_empty() {
        return result(reader, 0.0, 0.0);
    }
    let minimum = if must.is_empty() {
        minimum.max(1)
    } else {
        minimum
    };
    let optional_cost = if minimum == 0 {
        should.iter().map(|e| e.query_cost).sum()
    } else {
        0
    };
    if minimum > 0 {
        let required = if minimum > should.len() {
            result(reader, 0.0, 0.0)
        } else if minimum == 1 {
            combine(reader, pg_sys::BoolExprType::OR_EXPR, should)
        } else if minimum == should.len() {
            combine(reader, pg_sys::BoolExprType::AND_EXPR, should)
        } else {
            let cost = should.iter().map(|e| e.query_cost as f64).sum();
            // PostgreSQL has no minimum-should-match operator; assume independent clauses.
            let sel = if should.len().saturating_mul(minimum) > 1_000_000 {
                (should.iter().map(|e| e.selectivity).sum::<f64>() / minimum as f64).min(1.0)
            } else {
                let mut counts = vec![0.0; minimum];
                counts[0] = 1.0;
                for part in &should {
                    let p = part.selectivity;
                    for j in (1..minimum).rev() {
                        counts[j] = counts[j] * (1.0 - p) + counts[j - 1] * p;
                    }
                    counts[0] *= 1.0 - p;
                }
                1.0 - counts.iter().sum::<f64>()
            };
            result(reader, sel, cost)
        };
        must.push(required);
    }
    must.extend(
        must_not
            .into_iter()
            .map(|e| combine(reader, pg_sys::BoolExprType::NOT_EXPR, vec![e])),
    );
    let mut result = combine(reader, pg_sys::BoolExprType::AND_EXPR, must);
    result.query_cost = result.query_cost.saturating_add(optional_cost);
    result
}

pub(crate) fn range_prior<T>(lo: &Bound<T>, hi: &Bound<T>) -> f64 {
    match (lo, hi) {
        (Bound::Unbounded, Bound::Unbounded) => pg_sys::DEFAULT_NOT_UNK_SEL,
        (Bound::Unbounded, _) | (_, Bound::Unbounded) => pg_sys::DEFAULT_INEQ_SEL,
        _ => pg_sys::DEFAULT_RANGE_INEQ_SEL,
    }
}
#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::postgres::pdb_owned_value::PdbOwnedValue;
    use crate::query::{SearchQueryInput, pdb_query::pdb};
    use crate::scan::info::RowEstimate;

    use crate::index::reader::index::test_support::segmented_index_fixture;
    use pgrx::{Spi, pg_test};

    fn term(field: &str, value: impl Into<PdbOwnedValue>) -> SearchQueryInput {
        SearchQueryInput::FieldedQuery {
            field: field.into(),
            query: pdb::Query::Term {
                value: value.into(),
            },
        }
    }

    fn boolean(
        must: Vec<SearchQueryInput>,
        should: Vec<SearchQueryInput>,
        must_not: Vec<SearchQueryInput>,
        minimum_should_match: Option<i64>,
    ) -> SearchQueryInput {
        SearchQueryInput::Boolean {
            must,
            should,
            must_not,
            minimum_should_match,
        }
    }

    #[pg_test]
    fn metadata_selectivity_uses_every_segment() {
        let (index, _) = segmented_index_fixture("metadata_every_segment", 4, false);
        let query = term("title", "silver".to_owned());
        let reader = SearchIndexReader::open_for_estimation(&index, &query).unwrap();
        assert_eq!(reader.searcher().segment_readers().len(), 4);
        let result = reader.estimate_docs(&query, RowEstimate::Unknown);
        assert_eq!(result.matching_docs, 10);
        assert_eq!(result.selectivity, 0.25);
        assert_eq!(result.query_cost, 10);
        let scaled = reader.estimate_docs(&query, RowEstimate::Known(400));
        assert_eq!(scaled.matching_docs, 100);
        assert_eq!(scaled.selectivity, result.selectivity);
    }

    #[pg_test]
    fn metadata_selectivity_combines_nested_queries() {
        let (index, _) = segmented_index_fixture("metadata_boolean", 4, false);
        let reader =
            SearchIndexReader::open_for_estimation(&index, &SearchQueryInput::All).unwrap();
        let silver = term("title", "silver".to_owned());
        let quiet = term("title", "quiet".to_owned());
        let cases = [
            (
                boolean(vec![silver.clone()], vec![quiet.clone()], vec![], None),
                0.25,
            ),
            (
                boolean(vec![], vec![silver.clone(), quiet.clone()], vec![], None),
                0.8125,
            ),
            (
                boolean(vec![], vec![silver.clone(), quiet.clone()], vec![], Some(2)),
                0.1875,
            ),
            (
                boolean(
                    vec![SearchQueryInput::All],
                    vec![],
                    vec![silver.clone()],
                    None,
                ),
                0.75,
            ),
            (boolean(vec![], vec![], vec![silver.clone()], None), 0.0),
            (
                SearchQueryInput::DisjunctionMax {
                    disjuncts: vec![silver, quiet],
                    tie_breaker: None,
                },
                0.8125,
            ),
        ];
        for (query, expected) in cases {
            let query = SearchQueryInput::Boost {
                factor: 2.0,
                query: Box::new(SearchQueryInput::ConstScore {
                    score: 3.0,
                    query: Box::new(query),
                }),
            };
            let result = reader.estimate_docs(&query, RowEstimate::Unknown);
            assert!(
                (result.selectivity - expected).abs() < 1e-9,
                "{query:?}: {} != {expected}",
                result.selectivity
            );
        }
        for (conjunction, expected) in [(false, 0.8125), (true, 0.1875)] {
            let input = SearchQueryInput::FieldedQuery {
                field: "title".into(),
                query: pdb::Query::MatchArray {
                    tokens: vec!["silver".to_owned(), "quiet".to_owned()],
                    distance: None,
                    transposition_cost_one: None,
                    prefix: None,
                    conjunction_mode: Some(conjunction),
                },
            };
            let result = reader.estimate_docs(&input, RowEstimate::Unknown);
            assert!((result.selectivity - expected).abs() < 1e-9);
        }
        let parsed = SearchQueryInput::Parse {
            query_string: "(title:silver AND title:quiet)^2".into(),
            lenient: None,
            conjunction_mode: None,
        };
        let parser_reader = SearchIndexReader::open_for_estimation(&index, &parsed).unwrap();
        assert!(
            (parser_reader
                .estimate_docs(&parsed, RowEstimate::Unknown)
                .selectivity
                - 0.1875)
                .abs()
                < 1e-9
        );
    }

    #[pg_test]
    fn metadata_selectivity_never_builds_a_weight() {
        #[derive(Clone, Debug)]
        struct MetadataOnly(bool);
        impl tantivy::query::QueryEstimate for MetadataOnly {
            fn estimate_docs(
                &self,
                _: &tantivy::SegmentReader,
            ) -> tantivy::Result<Option<(u32, u64)>> {
                Ok(self.0.then_some((2, 7)))
            }
        }
        impl Query for MetadataOnly {
            fn weight(
                &self,
                _: tantivy::query::EnableScoring<'_>,
            ) -> tantivy::Result<Box<dyn tantivy::query::Weight>> {
                panic!("estimation must not construct a weight")
            }
        }
        let (index, _) = segmented_index_fixture("metadata_no_scorer", 2, false);
        let reader =
            SearchIndexReader::open_for_estimation(&index, &SearchQueryInput::All).unwrap();
        for (query, expected) in [
            (MetadataOnly(true), 0.2),
            (MetadataOnly(false), pg_sys::DEFAULT_MATCH_SEL),
        ] {
            assert!((segment_estimate(&query, &reader).selectivity - expected).abs() < 1e-9);
        }
        Spi::run("SET paradedb.enable_heuristic_selectivity = off").unwrap();
        let mlt = SearchQueryInput::FieldedQuery {
            field: "id".into(),
            query: pdb::Query::MoreLikeThis {
                key_value: PdbOwnedValue::I64(-999),
                fields: None,
                options: Default::default(),
            },
        };
        assert_eq!(
            crate::api::operator::estimate_selectivity_and_cost(&index, mlt.clone()).0,
            Some(0.1)
        );
        assert!(
            reader
                .build_query_tree_with_estimates(mlt)
                .unwrap()
                .estimated_docs
                .is_some()
        );
    }

    #[pg_test]
    fn metadata_selectivity_nontext_defaults() {
        let (index, _) = segmented_index_fixture("metadata_defaults", 2, false);
        let reader =
            SearchIndexReader::open_for_estimation(&index, &SearchQueryInput::All).unwrap();
        let cases = [
            (
                pdb::Query::Term {
                    value: PdbOwnedValue::I64(3),
                },
                pg_sys::DEFAULT_EQ_SEL,
            ),
            (pdb::Query::Exists, pg_sys::DEFAULT_NOT_UNK_SEL),
            (
                pdb::Query::Range {
                    lower_bound: Bound::Included(PdbOwnedValue::I64(3)),
                    upper_bound: Bound::Unbounded,
                },
                pg_sys::DEFAULT_INEQ_SEL,
            ),
            (
                pdb::Query::Range {
                    lower_bound: Bound::Included(PdbOwnedValue::I64(3)),
                    upper_bound: Bound::Included(PdbOwnedValue::I64(6)),
                },
                pg_sys::DEFAULT_RANGE_INEQ_SEL,
            ),
        ];
        for (query, expected) in cases {
            let input = SearchQueryInput::FieldedQuery {
                field: "id".into(),
                query,
            };
            let estimate = reader.estimate_docs(&input, RowEstimate::Unknown);
            assert!(
                (estimate.selectivity - expected).abs() < 1e-9,
                "{input:?}: {estimate:?}"
            );
            assert!(
                reader
                    .build_query_tree_with_estimates(input)
                    .unwrap()
                    .estimated_docs
                    .is_some()
            );
        }
    }
}

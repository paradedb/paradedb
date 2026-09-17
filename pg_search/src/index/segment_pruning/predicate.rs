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

use std::cmp::Ordering;
use std::ops::Bound;

use crate::index::stats::{EmpiricalStats, comparable, ends_before};
use crate::postgres::pdb_owned_value::PdbOwnedValue;

// NaN does not provide a conventional closed interval for these checks.
fn pruning_comparable(a: &PdbOwnedValue, b: &PdbOwnedValue) -> bool {
    comparable(a, b)
        && !matches!(a, PdbOwnedValue::F64(value) if value.is_nan())
        && !matches!(b, PdbOwnedValue::F64(value) if value.is_nan())
}

fn lower_contains(bound: &Bound<PdbOwnedValue>, value: &PdbOwnedValue) -> bool {
    !ends_before(Bound::Included(value), bound.as_ref())
}

fn upper_contains(bound: &Bound<PdbOwnedValue>, value: &PdbOwnedValue) -> bool {
    !ends_before(bound.as_ref(), Bound::Included(value))
}

fn bounds_comparable(
    stats: &EmpiricalStats,
    lower: &Bound<PdbOwnedValue>,
    upper: &Bound<PdbOwnedValue>,
) -> bool {
    [lower, upper].into_iter().all(|bound| match bound {
        Bound::Unbounded => true,
        Bound::Included(value) | Bound::Excluded(value) => {
            pruning_comparable(value, &stats.min) && pruning_comparable(value, &stats.max)
        }
    })
}

/// False only when the observed bounds exclude every possible matching value.
pub(crate) fn range_can_match(
    stats: Option<&EmpiricalStats>,
    lower: &Bound<PdbOwnedValue>,
    upper: &Bound<PdbOwnedValue>,
) -> bool {
    stats.is_none_or(|stats| {
        !bounds_comparable(stats, lower, upper) || stats.intersects(lower, upper)
    })
}

/// A range covers every document only when it covers both extrema and the field has no NULLs.
pub(crate) fn range_matches_all(
    stats: Option<&EmpiricalStats>,
    lower: &Bound<PdbOwnedValue>,
    upper: &Bound<PdbOwnedValue>,
) -> bool {
    stats.is_some_and(|stats| {
        bounds_comparable(stats, lower, upper)
            && !stats.nullable
            && lower_contains(lower, &stats.min)
            && upper_contains(upper, &stats.max)
    })
}

/// False only when the observed bounds exclude the term.
pub(crate) fn term_can_match(stats: Option<&EmpiricalStats>, term: &PdbOwnedValue) -> bool {
    stats.is_none_or(|stats| {
        !(pruning_comparable(term, &stats.min) && pruning_comparable(term, &stats.max))
            || (term.total_cmp(&stats.min) != Ordering::Less
                && term.total_cmp(&stats.max) != Ordering::Greater)
    })
}

/// A term covers every document only when it equals both extrema and the field has no NULLs.
pub(crate) fn term_matches_all(stats: Option<&EmpiricalStats>, term: &PdbOwnedValue) -> bool {
    stats.is_some_and(|stats| {
        !stats.nullable
            && pruning_comparable(term, &stats.min)
            && pruning_comparable(term, &stats.max)
            && term.total_cmp(&stats.min) == Ordering::Equal
            && term.total_cmp(&stats.max) == Ordering::Equal
    })
}

/// Tantivy's scorer and pruning_scorer disagree on the minimum for a single positive clause.
/// Negative minima are cast to usize during query compilation. Neither case provides a safe
/// exclusion or match-all guarantee, including when nested under NOT.
fn required_should_matches(
    must: usize,
    should: usize,
    must_not: usize,
    minimum_should_match: Option<i64>,
) -> Option<usize> {
    let minimum = minimum_should_match.unwrap_or(0);
    if minimum < 0 || (must + should == 1 && must_not == 0 && minimum as usize > should) {
        return None;
    }
    Some((minimum as usize).max(usize::from(must == 0 && should > 0)))
}

/// Positive clauses supply can_match results; negative clauses supply matches_all results.
/// Iterators are evaluated lazily, so an impossible conjunct stops further statistics reads.
pub(crate) fn boolean_can_match(
    mut must: impl ExactSizeIterator<Item = bool>,
    should: impl ExactSizeIterator<Item = bool>,
    mut must_not: impl ExactSizeIterator<Item = bool>,
    minimum_should_match: Option<i64>,
) -> bool {
    // Tantivy has no implicit MatchAll for an empty or pure-negative Boolean query.
    if must.len() == 0 && should.len() == 0 {
        return false;
    }
    let Some(required) = required_should_matches(
        must.len(),
        should.len(),
        must_not.len(),
        minimum_should_match,
    ) else {
        return true;
    };
    must.all(|possible| possible)
        && !must_not.any(|guaranteed| guaranteed)
        && should.filter(|possible| *possible).take(required).count() == required
}

/// Positive clauses supply matches_all results; negative clauses supply can_match results.
pub(crate) fn boolean_matches_all(
    mut must: impl ExactSizeIterator<Item = bool>,
    should: impl ExactSizeIterator<Item = bool>,
    mut must_not: impl ExactSizeIterator<Item = bool>,
    minimum_should_match: Option<i64>,
) -> bool {
    if must.len() == 0 && should.len() == 0 {
        return false;
    }
    let Some(required) = required_should_matches(
        must.len(),
        should.len(),
        must_not.len(),
        minimum_should_match,
    ) else {
        return false;
    };
    must.all(|guaranteed| guaranteed)
        && !must_not.any(|possible| possible)
        && should
            .filter(|guaranteed| *guaranteed)
            .take(required)
            .count()
            == required
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rstest::rstest;
    use tantivy::schema::{INDEXED, Schema};
    use tantivy::{Index, TantivyDocument, doc};

    type Proof = (bool, bool);
    const NEVER: Proof = (false, false);
    const MAYBE: Proof = (true, false);
    const ALWAYS: Proof = (true, true);

    fn range_truth(
        stats: Option<&EmpiricalStats>,
        lower: &Bound<PdbOwnedValue>,
        upper: &Bound<PdbOwnedValue>,
    ) -> Proof {
        (
            range_can_match(stats, lower, upper),
            range_matches_all(stats, lower, upper),
        )
    }

    fn term_truth(stats: Option<&EmpiricalStats>, term: &PdbOwnedValue) -> Proof {
        (term_can_match(stats, term), term_matches_all(stats, term))
    }

    fn boolean_truth(
        must: impl IntoIterator<Item = Proof>,
        should: impl IntoIterator<Item = Proof>,
        must_not: impl IntoIterator<Item = Proof>,
        minimum: Option<i64>,
    ) -> Proof {
        let (must, should, must_not): (Vec<_>, Vec<_>, Vec<_>) = (
            must.into_iter().collect(),
            should.into_iter().collect(),
            must_not.into_iter().collect(),
        );
        (
            boolean_can_match(
                must.iter().map(|p| p.0),
                should.iter().map(|p| p.0),
                must_not.iter().map(|p| p.1),
                minimum,
            ),
            boolean_matches_all(
                must.iter().map(|p| p.1),
                should.iter().map(|p| p.1),
                must_not.iter().map(|p| p.0),
                minimum,
            ),
        )
    }

    #[test]
    fn impossible_conjunct_stops_evaluation() {
        let must = [false, true]
            .into_iter()
            .inspect(|possible| assert!(!possible, "the later conjunct must not be visited"));
        assert!(!boolean_can_match(
            must,
            [].into_iter(),
            [].into_iter(),
            None
        ));
    }

    fn stats(min: i64, max: i64, nullable: bool) -> EmpiricalStats {
        EmpiricalStats {
            min: PdbOwnedValue::I64(min),
            max: PdbOwnedValue::I64(max),
            nullable,
        }
    }

    #[rstest]
    #[case::touching_inclusive(20, true, 30, true, false, MAYBE)]
    #[case::touching_exclusive(20, false, 30, true, false, NEVER)]
    #[case::exact_segment(10, true, 20, true, false, ALWAYS)]
    #[case::nullable_covering(0, true, 30, true, true, MAYBE)]
    fn range_boundaries_are_exact(
        #[case] lower: i64,
        #[case] lower_included: bool,
        #[case] upper: i64,
        #[case] upper_included: bool,
        #[case] nullable: bool,
        #[case] expected: Proof,
    ) {
        let bound = |value, included| {
            if included {
                Bound::Included(PdbOwnedValue::I64(value))
            } else {
                Bound::Excluded(PdbOwnedValue::I64(value))
            }
        };
        assert_eq!(
            range_truth(
                Some(&stats(10, 20, nullable)),
                &bound(lower, lower_included),
                &bound(upper, upper_included),
            ),
            expected
        );
    }

    #[rstest]
    #[case::inside_gap(50, MAYBE)]
    #[case::outside_bounds(101, NEVER)]
    fn term_truth_respects_only_proven_bounds(#[case] term: i64, #[case] expected: Proof) {
        assert_eq!(
            term_truth(Some(&stats(1, 100, false)), &PdbOwnedValue::I64(term)),
            expected
        );
    }

    #[test]
    fn missing_nullable_and_constant_statistics() {
        let term = PdbOwnedValue::I64(10);
        assert_eq!(term_truth(None, &term), MAYBE);
        assert_eq!(term_truth(Some(&stats(10, 10, true)), &term), MAYBE);
        assert_eq!(term_truth(Some(&stats(10, 10, false)), &term), ALWAYS);
        assert_eq!(term_truth(Some(&stats(11, 20, false)), &term), NEVER);
    }

    #[test]
    fn nan_statistics_and_bounds_fail_open() {
        let stats = EmpiricalStats {
            min: PdbOwnedValue::F64(f64::NAN),
            max: PdbOwnedValue::F64(10.0),
            nullable: false,
        };
        assert_eq!(term_truth(Some(&stats), &PdbOwnedValue::F64(20.0)), MAYBE);
        assert_eq!(
            range_truth(
                Some(&stats),
                &Bound::Included(PdbOwnedValue::F64(20.0)),
                &Bound::Unbounded,
            ),
            MAYBE
        );

        let finite_stats = EmpiricalStats {
            min: PdbOwnedValue::F64(1.0),
            max: PdbOwnedValue::F64(10.0),
            nullable: false,
        };
        assert_eq!(
            term_truth(Some(&finite_stats), &PdbOwnedValue::F64(f64::NAN)),
            MAYBE
        );
    }

    #[rstest]
    #[case::must_is_required(
        vec![NEVER], vec![ALWAYS], vec![], None, NEVER
    )]
    #[case::should_cannot_override_must(
        vec![ALWAYS], vec![NEVER], vec![], None, ALWAYS
    )]
    #[case::must_not_excludes_guaranteed_match(
        vec![ALWAYS], vec![], vec![ALWAYS], None, NEVER
    )]
    #[case::minimum_exceeds_possible(
        vec![], vec![ALWAYS, NEVER], vec![], Some(2), NEVER
    )]
    #[case::minimum_may_be_met(
        vec![], vec![ALWAYS, MAYBE], vec![], Some(2), MAYBE
    )]
    #[case::minimum_is_guaranteed(
        vec![], vec![ALWAYS, ALWAYS], vec![], Some(2), ALWAYS
    )]
    #[case::explicit_zero_still_unions_should(vec![], vec![NEVER], vec![], Some(0), NEVER)]
    #[case::negative_minimum_fails_open(vec![], vec![ALWAYS], vec![], Some(-1), MAYBE)]
    #[case::negative_minimum_fails_open_with_must(vec![ALWAYS], vec![], vec![], Some(-1), MAYBE)]
    #[case::single_should_oversized_minimum(vec![], vec![ALWAYS], vec![], Some(2), MAYBE)]
    #[case::single_must_positive_minimum(vec![ALWAYS], vec![], vec![], Some(1), MAYBE)]
    fn boolean_truth_cases(
        #[case] must: Vec<Proof>,
        #[case] should: Vec<Proof>,
        #[case] must_not: Vec<Proof>,
        #[case] minimum_should_match: Option<i64>,
        #[case] expected: Proof,
    ) {
        assert_eq!(
            boolean_truth(must, should, must_not, minimum_should_match),
            expected
        );
    }

    #[test]
    fn single_clause_boolean_proofs_agree_with_tantivy_execution() {
        use tantivy::collector::{Count, TopDocs};
        use tantivy::query::{AllQuery, BooleanQuery, EmptyQuery, EnableScoring, Occur, Query};
        use tantivy::{DocSet, TERMINATED};

        let mut schema = Schema::builder();
        let id = schema.add_u64_field("id", INDEXED);
        let index = Index::create_in_ram(schema.build());
        let mut writer: tantivy::IndexWriter<TantivyDocument> = index.writer(50_000_000).unwrap();
        writer.add_document(doc!(id => 1u64)).unwrap();
        writer.commit().unwrap();
        let searcher = index.reader().unwrap().searcher();
        let segment = searcher.segment_reader(0);

        for occur in [Occur::Must, Occur::Should] {
            for minimum in [0, 1, 2, -1] {
                for child_truth in [NEVER, ALWAYS] {
                    let child: Box<dyn Query> = match child_truth {
                        NEVER => Box::new(EmptyQuery),
                        ALWAYS => Box::new(AllQuery),
                        _ => unreachable!(),
                    };
                    let query = BooleanQuery::with_minimum_required_clauses(
                        vec![(occur, child)],
                        minimum as usize,
                    );
                    let proof = boolean_truth(
                        (occur == Occur::Must).then_some(child_truth),
                        (occur == Occur::Should).then_some(child_truth),
                        [],
                        Some(minimum),
                    );
                    let nested = BooleanQuery::new(vec![
                        (Occur::Must, Box::new(query.clone())),
                        (Occur::Must, Box::new(AllQuery)),
                    ]);
                    let negated = BooleanQuery::new(vec![
                        (Occur::Must, Box::new(AllQuery)),
                        (Occur::MustNot, Box::new(query.clone())),
                    ]);
                    let negated_proof = boolean_truth([ALWAYS], [], [proof], None);

                    for (query, proof) in [
                        (&query as &dyn Query, proof),
                        (&nested as &dyn Query, proof),
                        (&negated as &dyn Query, negated_proof),
                    ] {
                        let check = |count: usize| match proof {
                            NEVER => assert_eq!(count, 0, "{query:?}"),
                            ALWAYS => assert_eq!(count, 1, "{query:?}"),
                            _ => {}
                        };
                        check(searcher.search(query, &Count).unwrap());
                        check(
                            searcher
                                .search(query, &TopDocs::with_limit(1).order_by_score())
                                .unwrap()
                                .len(),
                        );
                        for scoring in [false, true] {
                            let weight = query
                                .weight(if scoring {
                                    EnableScoring::enabled_from_searcher(&searcher)
                                } else {
                                    EnableScoring::disabled_from_searcher(&searcher)
                                })
                                .unwrap();
                            let mut scorer = weight.scorer(segment, 1.0).unwrap();
                            check(scorer.count_including_deleted() as usize);
                            let mut scorer = weight
                                .pruning_scorer(segment, 1.0, f32::NEG_INFINITY)
                                .unwrap();
                            let mut count = 0;
                            while scorer.doc() != TERMINATED {
                                count += 1;
                                scorer.advance();
                            }
                            check(count);
                        }
                    }
                }
            }
        }
    }

    fn truth_from_mask(mask: u8) -> Proof {
        match mask & 0b1111 {
            0 => NEVER,
            0b1111 => ALWAYS,
            _ => MAYBE,
        }
    }

    proptest! {
        #[test]
        fn range_proof_is_sound_for_observed_values(
            values in prop::collection::vec(-100i64..=100, 1..32),
            nullable in any::<bool>(),
            lower in -120i64..=120,
            upper in -120i64..=120,
            lower_included in any::<bool>(),
            upper_included in any::<bool>(),
        ) {
            let min = *values.iter().min().unwrap();
            let max = *values.iter().max().unwrap();
            let stats = stats(min, max, nullable);
            let lower_bound = if lower_included {
                Bound::Included(PdbOwnedValue::I64(lower))
            } else {
                Bound::Excluded(PdbOwnedValue::I64(lower))
            };
            let upper_bound = if upper_included {
                Bound::Included(PdbOwnedValue::I64(upper))
            } else {
                Bound::Excluded(PdbOwnedValue::I64(upper))
            };
            let proof = range_truth(Some(&stats), &lower_bound, &upper_bound);
            let matches = values.iter().map(|value| {
                lower_contains(&lower_bound, &PdbOwnedValue::I64(*value))
                    && upper_contains(&upper_bound, &PdbOwnedValue::I64(*value))
            }).collect::<Vec<_>>();

            if proof == NEVER {
                prop_assert!(matches.iter().all(|matched| !matched));
            }
            if proof == ALWAYS {
                prop_assert!(!nullable);
                prop_assert!(matches.iter().all(|matched| *matched));
            }
        }

        #[test]
        fn boolean_proof_is_sound_for_four_documents(
            must in prop::collection::vec(0u8..16, 0..5),
            should in prop::collection::vec(0u8..16, 0..5),
            must_not in prop::collection::vec(0u8..16, 0..5),
            minimum_should_match in prop::option::of(-3i64..6),
        ) {
            let proof = boolean_truth(
                must.iter().copied().map(truth_from_mask),
                should.iter().copied().map(truth_from_mask),
                must_not.iter().copied().map(truth_from_mask),
                minimum_should_match,
            );
            if minimum_should_match.is_some_and(|minimum| minimum < 0)
                && !(must.is_empty() && should.is_empty())
            {
                prop_assert_eq!(proof, MAYBE);
            } else {
                let required = minimum_should_match.unwrap_or(0)
                    .max(i64::from(must.is_empty() && !should.is_empty())) as usize;
                let match_mask = if must.is_empty() && should.is_empty() {
                    0
                } else {
                    (0..4).fold(0u8, |result, doc| {
                        let bit = 1u8 << doc;
                        let matches = must.iter().all(|mask| mask & bit != 0)
                            && must_not.iter().all(|mask| mask & bit == 0)
                            && should.iter().filter(|mask| **mask & bit != 0).count() >= required;
                        result | if matches { bit } else { 0 }
                    })
                };

                if proof == NEVER {
                    prop_assert_eq!(match_mask, 0);
                }
                if proof == ALWAYS {
                    prop_assert_eq!(match_mask, 0b1111);
                }
            }
        }
    }
}

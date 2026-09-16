// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use std::cmp::Ordering;
use std::ops::Bound;
use std::sync::Arc;

use super::snapshot::SegmentStatsSnapshot;
use crate::index::stats::{EmpiricalStats, comparable};
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::schema::SearchField;
use tantivy::index::SegmentId;

/// Statistics involving NaN do not provide a conventional closed interval. Keep those segments
/// until the exact PostgreSQL/Tantivy NaN ordering is represented explicitly in the proof model.
fn pruning_comparable(a: &PdbOwnedValue, b: &PdbOwnedValue) -> bool {
    comparable(a, b)
        && !matches!(a, PdbOwnedValue::F64(value) if value.is_nan())
        && !matches!(b, PdbOwnedValue::F64(value) if value.is_nan())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SegmentTruth {
    /// The predicate cannot match any live row in this segment. This is the only state that may
    /// authorize skipping a segment; `Maybe` and `Always` are optimization information only.
    Never,
    Maybe,
    Always,
}

impl SegmentTruth {
    pub(crate) fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::Never, _) | (_, Self::Never) => Self::Never,
            (Self::Always, Self::Always) => Self::Always,
            _ => Self::Maybe,
        }
    }

    pub(crate) fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::Always, _) | (_, Self::Always) => Self::Always,
            (Self::Never, Self::Never) => Self::Never,
            _ => Self::Maybe,
        }
    }
}

#[derive(Debug)]
pub(crate) struct SegmentTruthTable {
    snapshot: Arc<SegmentStatsSnapshot>,
    values: Box<[SegmentTruth]>,
}

impl SegmentTruthTable {
    pub(crate) fn new(
        snapshot: Arc<SegmentStatsSnapshot>,
        values: impl IntoIterator<Item = SegmentTruth>,
    ) -> Arc<Self> {
        let values = values.into_iter().collect::<Box<[_]>>();
        assert_eq!(
            values.len(),
            snapshot.len(),
            "one table value is required for every snapshot segment"
        );
        Arc::new(Self { snapshot, values })
    }

    pub(crate) fn snapshot(&self) -> &Arc<SegmentStatsSnapshot> {
        &self.snapshot
    }

    pub(crate) fn at(&self, segment_idx: usize) -> SegmentTruth {
        self.values[segment_idx]
    }

    pub(crate) fn for_segment(&self, segment_id: SegmentId) -> SegmentTruth {
        self.snapshot
            .segment_index(segment_id)
            .map(|idx| self.at(idx))
            .unwrap_or(SegmentTruth::Maybe)
    }

    pub(crate) fn contains(&self, truth: SegmentTruth) -> bool {
        self.values.contains(&truth)
    }

    pub(crate) fn is_candidate(&self, segment_id: SegmentId) -> bool {
        self.snapshot.segment_index(segment_id).is_none_or(|idx| {
            self.snapshot.doc_count(idx) > 0 && self.at(idx) != SegmentTruth::Never
        })
    }

    pub(crate) fn uniform(snapshot: Arc<SegmentStatsSnapshot>, truth: SegmentTruth) -> Arc<Self> {
        let len = snapshot.len();
        Self::new(snapshot, std::iter::repeat_n(truth, len))
    }

    /// Rebase onto `additional`'s snapshot and conjoin a newly resolved predicate.
    pub(crate) fn conjunction(&self, additional: &Self) -> Arc<Self> {
        assert!(
            Arc::ptr_eq(self.snapshot(), additional.snapshot()),
            "truth tables can only be combined when they share one snapshot"
        );
        let snapshot = Arc::clone(additional.snapshot());
        Self::new(
            Arc::clone(&snapshot),
            (0..snapshot.len()).map(|idx| self.at(idx).and(additional.at(idx))),
        )
    }
}

fn lower_contains(bound: &Bound<PdbOwnedValue>, value: &PdbOwnedValue) -> bool {
    match bound {
        Bound::Unbounded => true,
        Bound::Included(lower) => lower.total_cmp(value) != Ordering::Greater,
        Bound::Excluded(lower) => lower.total_cmp(value) == Ordering::Less,
    }
}

fn upper_contains(bound: &Bound<PdbOwnedValue>, value: &PdbOwnedValue) -> bool {
    match bound {
        Bound::Unbounded => true,
        Bound::Included(upper) => upper.total_cmp(value) != Ordering::Less,
        Bound::Excluded(upper) => upper.total_cmp(value) == Ordering::Greater,
    }
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

pub(crate) fn range_truth(
    stats: Option<&EmpiricalStats>,
    lower: &Bound<PdbOwnedValue>,
    upper: &Bound<PdbOwnedValue>,
) -> SegmentTruth {
    let Some(stats) = stats else {
        return SegmentTruth::Maybe;
    };
    if !bounds_comparable(stats, lower, upper) {
        return SegmentTruth::Maybe;
    }
    if !stats.intersects(lower, upper) {
        return SegmentTruth::Never;
    }
    if !stats.nullable && lower_contains(lower, &stats.min) && upper_contains(upper, &stats.max) {
        SegmentTruth::Always
    } else {
        SegmentTruth::Maybe
    }
}

pub(crate) fn term_truth(stats: Option<&EmpiricalStats>, term: &PdbOwnedValue) -> SegmentTruth {
    let Some(stats) = stats else {
        return SegmentTruth::Maybe;
    };
    if !(pruning_comparable(term, &stats.min) && pruning_comparable(term, &stats.max)) {
        return SegmentTruth::Maybe;
    }
    if term.total_cmp(&stats.min) == Ordering::Less
        || term.total_cmp(&stats.max) == Ordering::Greater
    {
        SegmentTruth::Never
    } else if !stats.nullable
        && term.total_cmp(&stats.min) == Ordering::Equal
        && term.total_cmp(&stats.max) == Ordering::Equal
    {
        SegmentTruth::Always
    } else {
        SegmentTruth::Maybe
    }
}

/// A term set sorted once so each segment's proof costs two binary searches instead of a pass
/// over every term. Join InList pushdowns can carry tens of thousands of terms.
#[derive(Debug)]
pub(crate) struct SortedTerms {
    terms: Vec<PdbOwnedValue>,
    /// Every term is comparable with every other and none is NaN, so `total_cmp` is a total
    /// order over the set. A mixed set is proven term by term.
    homogeneous: bool,
}

impl SortedTerms {
    pub(crate) fn new(mut terms: Vec<PdbOwnedValue>) -> Self {
        let homogeneous = terms
            .first()
            .is_some_and(|first| terms.iter().all(|term| pruning_comparable(term, first)));
        if homogeneous {
            terms.sort_by(|a, b| a.total_cmp(b));
        }
        Self { terms, homogeneous }
    }
}

pub(crate) fn terms_truth(stats: Option<&EmpiricalStats>, terms: &SortedTerms) -> SegmentTruth {
    let Some(stats) = stats else {
        return SegmentTruth::Maybe;
    };
    let Some(first) = terms.terms.first() else {
        return SegmentTruth::Never;
    };
    if !terms.homogeneous {
        return disjunction_truth(terms.terms.iter().map(|term| term_truth(Some(stats), term)));
    }
    if !(pruning_comparable(first, &stats.min) && pruning_comparable(first, &stats.max)) {
        return SegmentTruth::Maybe;
    }
    let start = terms
        .terms
        .partition_point(|term| term.total_cmp(&stats.min) == Ordering::Less);
    let inside = terms
        .terms
        .get(start)
        .is_some_and(|term| term.total_cmp(&stats.max) != Ordering::Greater);
    if !inside {
        SegmentTruth::Never
    } else if !stats.nullable && stats.min.total_cmp(&stats.max) == Ordering::Equal {
        SegmentTruth::Always
    } else {
        SegmentTruth::Maybe
    }
}

pub(crate) fn exists_truth(stats: Option<&EmpiricalStats>) -> SegmentTruth {
    match stats {
        Some(stats) if !stats.nullable => SegmentTruth::Always,
        Some(_) | None => SegmentTruth::Maybe,
    }
}

pub(crate) fn boolean_truth(
    must: impl IntoIterator<Item = SegmentTruth>,
    should: impl IntoIterator<Item = SegmentTruth>,
    must_not: impl IntoIterator<Item = SegmentTruth>,
    minimum_should_match: Option<i64>,
) -> SegmentTruth {
    let (mut must_count, mut must_has_never, mut must_all_always) = (0, false, true);
    for truth in must {
        must_count += 1;
        must_has_never |= truth == SegmentTruth::Never;
        must_all_always &= truth == SegmentTruth::Always;
    }
    let (mut should_count, mut possible_should, mut guaranteed_should) = (0, 0, 0);
    for truth in should {
        should_count += 1;
        possible_should += usize::from(truth != SegmentTruth::Never);
        guaranteed_should += usize::from(truth == SegmentTruth::Always);
    }
    let (mut must_not_count, mut must_not_has_always, mut must_not_all_never) = (0, false, true);
    for truth in must_not {
        must_not_count += 1;
        must_not_has_always |= truth == SegmentTruth::Always;
        must_not_all_never &= truth == SegmentTruth::Never;
    }
    // Tantivy has no implicit MatchAll for a pure-negative or empty Boolean query.
    if must_count == 0 && should_count == 0 {
        return SegmentTruth::Never;
    }
    // Execution casts a negative value to a huge `usize`, which Tantivy's single-clause
    // shortcut ignores and its general path treats as unsatisfiable. Neither reading is safe
    // to prove against, and under `must_not` a wrong `Always` becomes a wrong `Never`.
    let minimum_should_match = minimum_should_match.unwrap_or(0);
    if minimum_should_match < 0 {
        return SegmentTruth::Maybe;
    }
    // `scorer()` bypasses the minimum for one positive child, whereas
    // `pruning_scorer()` takes the general Boolean path. Nested Booleans can reach
    // either path, so an oversized minimum cannot prove Never (or Always under NOT).
    if must_count + should_count == 1
        && must_not_count == 0
        && minimum_should_match as usize > should_count
    {
        return SegmentTruth::Maybe;
    }
    let required =
        minimum_should_match.max(i64::from(must_count == 0 && should_count > 0)) as usize;

    if must_has_never || must_not_has_always || possible_should < required {
        return SegmentTruth::Never;
    }

    if must_all_always && must_not_all_never && guaranteed_should >= required {
        SegmentTruth::Always
    } else {
        SegmentTruth::Maybe
    }
}

pub(crate) fn disjunction_truth(children: impl IntoIterator<Item = SegmentTruth>) -> SegmentTruth {
    children
        .into_iter()
        .fold(SegmentTruth::Never, SegmentTruth::or)
}

/// One truth per snapshot segment for a predicate whose field and values are already resolved.
pub(crate) fn truths_for_field(
    snapshot: &SegmentStatsSnapshot,
    field: &SearchField,
    truth: impl Fn(Option<&EmpiricalStats>) -> SegmentTruth,
) -> Box<[SegmentTruth]> {
    (0..snapshot.len())
        .map(|idx| {
            let stats = snapshot.empirical(idx, field);
            truth(stats.as_ref())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use SegmentTruth::{Always, Maybe, Never};
    use proptest::prelude::*;
    use rstest::rstest;
    use tantivy::schema::{INDEXED, Schema};
    use tantivy::{Index, TantivyDocument, doc};

    fn one_segment_snapshot() -> Arc<SegmentStatsSnapshot> {
        let mut schema = Schema::builder();
        let field = schema.add_u64_field("id", INDEXED);
        let index = Index::create_in_ram(schema.build());
        let mut writer: tantivy::IndexWriter<TantivyDocument> = index.writer(50_000_000).unwrap();
        writer.add_document(doc!(field => 1u64)).unwrap();
        writer.commit().unwrap();
        SegmentStatsSnapshot::capture(&index.reader().unwrap().searcher())
    }

    fn stats(min: i64, max: i64, nullable: bool) -> EmpiricalStats {
        EmpiricalStats {
            min: PdbOwnedValue::I64(min),
            max: PdbOwnedValue::I64(max),
            nullable,
        }
    }

    #[rstest]
    #[case::touching_inclusive(20, true, 30, true, false, Maybe)]
    #[case::touching_exclusive(20, false, 30, true, false, Never)]
    #[case::exact_segment(10, true, 20, true, false, Always)]
    #[case::nullable_covering(0, true, 30, true, true, Maybe)]
    fn range_boundaries_are_exact(
        #[case] lower: i64,
        #[case] lower_included: bool,
        #[case] upper: i64,
        #[case] upper_included: bool,
        #[case] nullable: bool,
        #[case] expected: SegmentTruth,
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
    #[case::inside_gap(50, Maybe)]
    #[case::outside_bounds(101, Never)]
    fn term_truth_respects_only_proven_bounds(#[case] term: i64, #[case] expected: SegmentTruth) {
        assert_eq!(
            term_truth(Some(&stats(1, 100, false)), &PdbOwnedValue::I64(term)),
            expected
        );
    }

    fn sorted(terms: impl IntoIterator<Item = i64>) -> SortedTerms {
        SortedTerms::new(terms.into_iter().map(PdbOwnedValue::I64).collect())
    }

    #[test]
    fn term_sets_and_presence_obey_min_max_and_nullability() {
        let constant = stats(10, 10, false);
        assert_eq!(
            terms_truth(Some(&constant), &sorted([9, 11])),
            SegmentTruth::Never
        );
        assert_eq!(
            terms_truth(Some(&constant), &sorted([9, 10])),
            SegmentTruth::Always
        );
        assert_eq!(
            terms_truth(Some(&constant), &sorted([])),
            SegmentTruth::Never
        );
        let mixed = SortedTerms::new(vec![
            PdbOwnedValue::I64(10),
            PdbOwnedValue::Str("10".to_string()),
        ]);
        assert_eq!(terms_truth(Some(&constant), &mixed), SegmentTruth::Always);
        assert_eq!(
            terms_truth(
                Some(&stats(1, 5, false)),
                &SortedTerms::new(vec![PdbOwnedValue::Str("a".to_string())])
            ),
            SegmentTruth::Maybe
        );
        assert_eq!(exists_truth(Some(&constant)), SegmentTruth::Always);
        assert_eq!(
            exists_truth(Some(&stats(10, 10, true))),
            SegmentTruth::Maybe
        );
        assert_eq!(exists_truth(None), SegmentTruth::Maybe);
        assert_eq!(
            term_truth(None, &PdbOwnedValue::I64(10)),
            SegmentTruth::Maybe
        );
    }

    #[test]
    fn nan_statistics_and_bounds_fail_open() {
        let stats = EmpiricalStats {
            min: PdbOwnedValue::F64(f64::NAN),
            max: PdbOwnedValue::F64(10.0),
            nullable: false,
        };
        assert_eq!(
            term_truth(Some(&stats), &PdbOwnedValue::F64(20.0)),
            SegmentTruth::Maybe
        );
        assert_eq!(
            range_truth(
                Some(&stats),
                &Bound::Included(PdbOwnedValue::F64(20.0)),
                &Bound::Unbounded,
            ),
            SegmentTruth::Maybe
        );

        let finite_stats = EmpiricalStats {
            min: PdbOwnedValue::F64(1.0),
            max: PdbOwnedValue::F64(10.0),
            nullable: false,
        };
        assert_eq!(
            term_truth(Some(&finite_stats), &PdbOwnedValue::F64(f64::NAN)),
            SegmentTruth::Maybe
        );
    }

    #[rstest]
    #[case::must_is_required(
        vec![Never], vec![Always], vec![], None, Never
    )]
    #[case::should_cannot_override_must(
        vec![Always], vec![Never], vec![], None, Always
    )]
    #[case::must_not_excludes_guaranteed_match(
        vec![Always], vec![], vec![Always], None, Never
    )]
    #[case::minimum_exceeds_possible(
        vec![], vec![Always, Never], vec![], Some(2), Never
    )]
    #[case::minimum_may_be_met(
        vec![], vec![Always, Maybe], vec![], Some(2), Maybe
    )]
    #[case::minimum_is_guaranteed(
        vec![], vec![Always, Always], vec![], Some(2), Always
    )]
    #[case::explicit_zero_still_unions_should(vec![], vec![Never], vec![], Some(0), Never)]
    #[case::negative_minimum_fails_open(vec![], vec![Always], vec![], Some(-1), Maybe)]
    #[case::negative_minimum_fails_open_with_must(vec![Always], vec![], vec![], Some(-1), Maybe)]
    #[case::single_should_oversized_minimum(vec![], vec![Always], vec![], Some(2), Maybe)]
    #[case::single_must_positive_minimum(vec![Always], vec![], vec![], Some(1), Maybe)]
    fn boolean_truth_cases(
        #[case] must: Vec<SegmentTruth>,
        #[case] should: Vec<SegmentTruth>,
        #[case] must_not: Vec<SegmentTruth>,
        #[case] minimum_should_match: Option<i64>,
        #[case] expected: SegmentTruth,
    ) {
        assert_eq!(
            boolean_truth(must, should, must_not, minimum_should_match),
            expected
        );
    }

    #[test]
    fn conjunction_uses_shared_snapshot() {
        let snapshot = one_segment_snapshot();
        let additional = SegmentTruthTable::uniform(Arc::clone(&snapshot), SegmentTruth::Never);
        let possible = SegmentTruthTable::uniform(snapshot, SegmentTruth::Maybe);
        assert_eq!(possible.conjunction(&additional).at(0), SegmentTruth::Never);
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
                for child_truth in [Never, Always] {
                    let child: Box<dyn Query> = match child_truth {
                        Never => Box::new(EmptyQuery),
                        Always => Box::new(AllQuery),
                        Maybe => unreachable!(),
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
                    let negated_proof = boolean_truth([Always], [], [proof], None);

                    for (query, proof) in [
                        (&query as &dyn Query, proof),
                        (&nested as &dyn Query, proof),
                        (&negated as &dyn Query, negated_proof),
                    ] {
                        let check = |count: usize| match proof {
                            Never => assert_eq!(count, 0, "{query:?}"),
                            Always => assert_eq!(count, 1, "{query:?}"),
                            Maybe => {}
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

    fn truth_from_mask(mask: u8) -> SegmentTruth {
        match mask & 0b1111 {
            0 => SegmentTruth::Never,
            0b1111 => SegmentTruth::Always,
            _ => SegmentTruth::Maybe,
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

            if proof == SegmentTruth::Never {
                prop_assert!(matches.iter().all(|matched| !matched));
            }
            if proof == SegmentTruth::Always {
                prop_assert!(!nullable);
                prop_assert!(matches.iter().all(|matched| *matched));
            }
        }

        #[test]
        fn sorted_terms_agree_with_term_by_term_proofs(
            values in prop::collection::vec(-20i64..=20, 1..8),
            nullable in any::<bool>(),
            terms in prop::collection::vec(-25i64..=25, 0..12),
        ) {
            let min = *values.iter().min().unwrap();
            let max = *values.iter().max().unwrap();
            let stats = stats(min, max, nullable);
            let expected = disjunction_truth(
                terms.iter().map(|term| term_truth(Some(&stats), &PdbOwnedValue::I64(*term))),
            );
            prop_assert_eq!(terms_truth(Some(&stats), &sorted(terms)), expected);
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
                prop_assert_eq!(proof, SegmentTruth::Maybe);
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

                if proof == SegmentTruth::Never {
                    prop_assert_eq!(match_mask, 0);
                }
                if proof == SegmentTruth::Always {
                    prop_assert_eq!(match_mask, 0b1111);
                }
            }
        }
    }
}

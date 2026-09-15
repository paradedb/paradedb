// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::SearchQueryInput;
use super::numeric::convert_value_for_field;
use super::pdb_query::{canonicalize_range_bounds_for_field, pdb};
use crate::api::FieldName;
use crate::api::version::Version;
use crate::index::segment_pruning::SegmentStatsSnapshot;
use crate::index::segment_pruning::predicate::{
    SegmentTruth, SegmentTruthTable, SortedTerms, boolean_truth, exists_truth, range_truth,
    term_truth, terms_truth, truths_for_field,
};
use crate::index::stats::EmpiricalStats;
use crate::schema::{SearchField, SearchFieldType, SearchIndexSchema};
use tantivy::index::SegmentReader;
use tantivy::query::{
    AllScorer, ConstScorer, EmptyScorer, EmptyWeight, EnableScoring, Explanation, Query,
    QueryClone, Scorer, Weight,
};
use tantivy::schema::{Field, FieldType};
use tantivy::{DocId, Score, Term};

/// A range-partition predicate whose result can be decided from the current segment's bounds.
///
/// The exact query remains authoritative for `Maybe`. `Always` avoids opening its range scorer,
/// while retaining the constant score produced by Tantivy range queries.
#[derive(Debug)]
struct SegmentRangeQuery {
    inner: Box<dyn Query>,
    truth: Arc<SegmentTruthTable>,
    removed: Arc<RangeFilterRemovalCounter>,
}

impl QueryClone for SegmentRangeQuery {
    fn box_clone(&self) -> Box<dyn Query> {
        Box::new(Self {
            inner: self.inner.box_clone(),
            truth: Arc::clone(&self.truth),
            removed: Arc::clone(&self.removed),
        })
    }
}

impl Query for SegmentRangeQuery {
    fn weight(&self, enable_scoring: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
        let scoring_enabled = enable_scoring.is_scoring_enabled();
        Ok(Box::new(SegmentRangeWeight {
            inner: self.inner.weight(enable_scoring)?,
            truth: Arc::clone(&self.truth),
            removed: Arc::clone(&self.removed),
            scoring_enabled,
        }))
    }

    fn query_terms(
        &self,
        field: Field,
        segment_reader: &SegmentReader,
        visitor: &mut dyn FnMut(&Term, bool),
    ) {
        self.inner.query_terms(field, segment_reader, visitor);
    }
}

struct SegmentRangeWeight {
    inner: Box<dyn Weight>,
    truth: Arc<SegmentTruthTable>,
    removed: Arc<RangeFilterRemovalCounter>,
    scoring_enabled: bool,
}

/// Per-derived-reader observer for the range-partition optimization. Keeping it next to the
/// wrapper makes the end-to-end DataFusion test fail if execution stops installing that wrapper.
#[derive(Debug, Default)]
pub(crate) struct RangeFilterRemovalCounter(AtomicUsize);

impl RangeFilterRemovalCounter {
    pub(crate) fn get(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }

    fn increment(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

impl SegmentRangeWeight {
    fn segment_scorer(
        &self,
        reader: &SegmentReader,
        boost: Score,
    ) -> tantivy::Result<Box<dyn Scorer>> {
        match self.truth.for_segment(reader.segment_id()) {
            SegmentTruth::Never => Ok(Box::new(EmptyScorer)),
            SegmentTruth::Maybe => self.inner.scorer(reader, boost),
            SegmentTruth::Always => {
                self.removed.increment();
                if self.scoring_enabled {
                    Ok(Box::new(ConstScorer::new(
                        AllScorer::new(reader.max_doc()),
                        boost,
                    )))
                } else {
                    // Tantivy recognizes and removes a bare AllScorer from Boolean intersections.
                    Ok(Box::new(AllScorer::new(reader.max_doc())))
                }
            }
        }
    }
}

impl Weight for SegmentRangeWeight {
    fn scorer(&self, reader: &SegmentReader, boost: Score) -> tantivy::Result<Box<dyn Scorer>> {
        self.segment_scorer(reader, boost)
    }

    fn explain(&self, reader: &SegmentReader, doc: DocId) -> tantivy::Result<Explanation> {
        match self.truth.for_segment(reader.segment_id()) {
            SegmentTruth::Never => EmptyWeight.explain(reader, doc),
            SegmentTruth::Maybe => self.inner.explain(reader, doc),
            SegmentTruth::Always if doc < reader.max_doc() => Ok(Explanation::new(
                "range predicate implied by segment bounds",
                1.0,
            )),
            SegmentTruth::Always => EmptyWeight.explain(reader, doc),
        }
    }

    fn count(&self, reader: &SegmentReader) -> tantivy::Result<u32> {
        match self.truth.for_segment(reader.segment_id()) {
            SegmentTruth::Never => Ok(0),
            SegmentTruth::Maybe => self.inner.count(reader),
            SegmentTruth::Always => Ok(reader.num_docs()),
        }
    }
}

pub(crate) fn wrap_range_partition_filter(
    query: Box<dyn Query>,
    input: &SearchQueryInput,
    truth: Arc<SegmentTruthTable>,
    removed: Arc<RangeFilterRemovalCounter>,
) -> Box<dyn Query> {
    let mut contains_range = false;
    input.visit_ref(&mut |query| {
        contains_range |= matches!(
            query,
            SearchQueryInput::FieldedQuery {
                query: pdb::Query::Range { .. },
                ..
            }
        );
    });

    if contains_range && truth.contains(SegmentTruth::Always) {
        Box::new(SegmentRangeQuery {
            inner: query,
            truth,
            removed,
        })
    } else {
        query
    }
}

pub(crate) struct PruningQueryBuilder<'a> {
    snapshot: Arc<SegmentStatsSnapshot>,
    schema: &'a SearchIndexSchema,
    index_created_by_version: Option<Version>,
}

impl<'a> PruningQueryBuilder<'a> {
    fn stats_order_compatible(field: &crate::schema::SearchField) -> bool {
        field.is_raw_sortable()
            // `SearchField::is_sortable` does not currently advertise IP fields, but their
            // Tantivy fast-field and query representations are both `IpAddr` and therefore
            // share the exact ordering recorded by `.stats`.
            || (matches!(field.field_type(), SearchFieldType::Inet(_)) && field.is_fast())
    }

    fn value_stats_order_compatible(field: &crate::schema::SearchField) -> bool {
        Self::stats_order_compatible(field)
            // String statistics describe complete columnar values. They can prove the absence of
            // an inverted-index term only when indexing emits that same complete value as one
            // term; analyzed tokenizers must fail open. Gate on the Tantivy field type because
            // uuid columns also accept `text_fields` tokenizer configurations.
            && (!matches!(field.field_entry().field_type(), FieldType::Str(_))
                || field.is_keyword())
    }

    pub(crate) fn new(
        snapshot: Arc<SegmentStatsSnapshot>,
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
        let search_field = self.schema.search_field(field.root())?;
        Self::stats_order_compatible(&search_field).then_some(search_field)
    }

    fn eligible_value_field(&self, field: &FieldName) -> Option<SearchField> {
        self.eligible_field(field)
            .filter(Self::value_stats_order_compatible)
    }

    fn uniform(&self, truth: SegmentTruth) -> Truths {
        std::iter::repeat_n(truth, self.snapshot.len()).collect()
    }

    fn for_field(
        &self,
        field: &SearchField,
        truth: impl Fn(Option<&EmpiricalStats>) -> SegmentTruth,
    ) -> Truths {
        truths_for_field(&self.snapshot, field, truth)
    }

    fn disjunction(&self, children: impl Iterator<Item = Truths>) -> Truths {
        children.fold(self.uniform(SegmentTruth::Never), |acc, child| {
            elementwise(&acc, &child, SegmentTruth::or)
        })
    }

    /// Each leaf resolves its field and canonical values once, then proves every segment.
    fn fielded_truth(&self, field: &FieldName, query: &pdb::Query) -> Truths {
        match query {
            pdb::Query::All => self.uniform(SegmentTruth::Always),
            pdb::Query::Empty => self.uniform(SegmentTruth::Never),
            pdb::Query::ScoreAdjusted { query, .. } => self.fielded_truth(field, query),
            pdb::Query::Exists => match self.eligible_field(field) {
                Some(search_field) => self.for_field(&search_field, exists_truth),
                None => self.uniform(SegmentTruth::Maybe),
            },
            pdb::Query::Term { value } => {
                let Some(search_field) = self.eligible_value_field(field) else {
                    return self.uniform(SegmentTruth::Maybe);
                };
                let Ok(value) = convert_value_for_field(
                    value.clone(),
                    &search_field.field_type(),
                    self.index_created_by_version,
                ) else {
                    return self.uniform(SegmentTruth::Maybe);
                };
                self.for_field(&search_field, |stats| term_truth(stats, &value))
            }
            pdb::Query::TermSet { terms } => {
                let Some(search_field) = self.eligible_value_field(field) else {
                    return self.uniform(SegmentTruth::Maybe);
                };
                let Some(terms) = terms
                    .iter()
                    .cloned()
                    .map(|value| {
                        convert_value_for_field(
                            value,
                            &search_field.field_type(),
                            self.index_created_by_version,
                        )
                        .ok()
                    })
                    .collect::<Option<Vec<_>>>()
                else {
                    return self.uniform(SegmentTruth::Maybe);
                };
                let terms = SortedTerms::new(terms);
                self.for_field(&search_field, |stats| terms_truth(stats, &terms))
            }
            pdb::Query::Range {
                lower_bound,
                upper_bound,
            } => {
                let Some(search_field) = self.eligible_value_field(field) else {
                    return self.uniform(SegmentTruth::Maybe);
                };
                let Ok((lower, upper)) = canonicalize_range_bounds_for_field(
                    &search_field,
                    self.index_created_by_version,
                    lower_bound.clone(),
                    upper_bound.clone(),
                ) else {
                    return self.uniform(SegmentTruth::Maybe);
                };
                self.for_field(&search_field, |stats| range_truth(stats, &lower, &upper))
            }
            _ => self.uniform(SegmentTruth::Maybe),
        }
    }

    fn truths(&self, input: &SearchQueryInput) -> Truths {
        match input {
            SearchQueryInput::All => self.uniform(SegmentTruth::Always),
            SearchQueryInput::Empty => self.uniform(SegmentTruth::Never),
            SearchQueryInput::TermSet { terms } => self.disjunction(terms.iter().map(|term| {
                self.fielded_truth(
                    &term.field,
                    &pdb::Query::Term {
                        value: term.value.clone(),
                    },
                )
            })),
            SearchQueryInput::FieldedQuery { field, query } => self.fielded_truth(field, query),
            SearchQueryInput::Boolean {
                must,
                should,
                must_not,
                minimum_should_match,
            } => {
                let lower = |queries: &[SearchQueryInput]| {
                    queries
                        .iter()
                        .map(|query| self.truths(query))
                        .collect::<Vec<_>>()
                };
                let (must, should, must_not) = (lower(must), lower(should), lower(must_not));
                (0..self.snapshot.len())
                    .map(|idx| {
                        boolean_truth(
                            must.iter().map(|truths| truths[idx]),
                            should.iter().map(|truths| truths[idx]),
                            must_not.iter().map(|truths| truths[idx]),
                            *minimum_should_match,
                        )
                    })
                    .collect()
            }
            SearchQueryInput::Boost { query, .. }
            | SearchQueryInput::ConstScore { query, .. }
            | SearchQueryInput::WithIndex { query, .. } => self.truths(query),
            SearchQueryInput::DisjunctionMax { disjuncts, .. } => {
                self.disjunction(disjuncts.iter().map(|query| self.truths(query)))
            }
            SearchQueryInput::ScoreFilter { query, .. } => match query.as_deref() {
                Some(query) => rejections_only(&self.truths(query)),
                None => self.uniform(SegmentTruth::Maybe),
            },
            SearchQueryInput::HeapFilter { indexed_query, .. } => {
                rejections_only(&self.truths(indexed_query))
            }
            _ => self.uniform(SegmentTruth::Maybe),
        }
    }

    pub(crate) fn truth_for_query(&self, input: &SearchQueryInput) -> Arc<SegmentTruthTable> {
        SegmentTruthTable::new(Arc::clone(&self.snapshot), self.truths(input))
    }
}

type Truths = Box<[SegmentTruth]>;

fn elementwise(
    left: &[SegmentTruth],
    right: &[SegmentTruth],
    combine: fn(SegmentTruth, SegmentTruth) -> SegmentTruth,
) -> Truths {
    left.iter()
        .zip(right)
        .map(|(left, right)| combine(*left, *right))
        .collect()
}

/// A filter that re-checks rows outside the index can only inherit impossibility.
fn rejections_only(truths: &[SegmentTruth]) -> Truths {
    truths
        .iter()
        .map(|truth| match truth {
            SegmentTruth::Never => SegmentTruth::Never,
            SegmentTruth::Maybe | SegmentTruth::Always => SegmentTruth::Maybe,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tantivy::schema::{INDEXED, Schema};
    use tantivy::{DocSet, Index, TantivyDocument, doc};

    #[derive(Debug, Default)]
    struct Calls {
        scorers: AtomicUsize,
        counts: AtomicUsize,
        explains: AtomicUsize,
    }

    #[derive(Debug, Clone)]
    struct CountingQuery(Arc<Calls>);

    impl Query for CountingQuery {
        fn weight(&self, _scoring: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
            Ok(Box::new(CountingWeight(Arc::clone(&self.0))))
        }
    }

    struct CountingWeight(Arc<Calls>);

    impl Weight for CountingWeight {
        fn scorer(&self, reader: &SegmentReader, boost: Score) -> tantivy::Result<Box<dyn Scorer>> {
            self.0.scorers.fetch_add(1, Ordering::Relaxed);
            Ok(Box::new(ConstScorer::new(
                AllScorer::new(reader.max_doc()),
                boost,
            )))
        }

        fn explain(&self, _reader: &SegmentReader, _doc: DocId) -> tantivy::Result<Explanation> {
            self.0.explains.fetch_add(1, Ordering::Relaxed);
            Ok(Explanation::new("inner range", 1.0))
        }

        fn count(&self, reader: &SegmentReader) -> tantivy::Result<u32> {
            self.0.counts.fetch_add(1, Ordering::Relaxed);
            Ok(reader.num_docs())
        }
    }

    fn query_with_truth(
        truth: SegmentTruth,
        scoring: bool,
    ) -> (Box<dyn Weight>, Arc<Calls>, SegmentReader) {
        let mut schema = Schema::builder();
        let id = schema.add_u64_field("id", INDEXED);
        let index = Index::create_in_ram(schema.build());
        let mut writer: tantivy::IndexWriter<TantivyDocument> = index.writer(50_000_000).unwrap();
        writer.add_document(doc!(id => 1u64)).unwrap();
        writer.commit().unwrap();
        let searcher = index.reader().unwrap().searcher();
        let segment = searcher.segment_reader(0).clone();
        let snapshot = SegmentStatsSnapshot::capture(&searcher);
        let calls = Arc::new(Calls::default());
        let query = SegmentRangeQuery {
            inner: Box::new(CountingQuery(Arc::clone(&calls))),
            truth: SegmentTruthTable::uniform(snapshot, truth),
            removed: Arc::new(RangeFilterRemovalCounter::default()),
        };
        let weight = query
            .weight(if scoring {
                EnableScoring::enabled_from_searcher(&searcher)
            } else {
                EnableScoring::disabled_from_searcher(&searcher)
            })
            .unwrap();
        (weight, calls, segment)
    }

    #[test]
    fn contained_range_does_not_enter_inner_scorer_and_preserves_score() {
        let (weight, calls, segment) = query_with_truth(SegmentTruth::Always, true);
        let mut scorer = weight.scorer(&segment, 2.5).unwrap();
        assert_eq!(scorer.doc(), 0);
        assert_eq!(scorer.score(), 2.5);
        assert_eq!(weight.count(&segment).unwrap(), 1);
        assert_eq!(weight.explain(&segment, 0).unwrap().value(), 1.0);
        assert_eq!(calls.scorers.load(Ordering::Relaxed), 0);
        assert_eq!(calls.counts.load(Ordering::Relaxed), 0);
        assert_eq!(calls.explains.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn unscored_contained_range_becomes_a_removable_all_scorer() {
        let (weight, calls, segment) = query_with_truth(SegmentTruth::Always, false);
        let scorer = weight.scorer(&segment, 1.0).unwrap();
        assert!(scorer.is::<AllScorer>());
        assert_eq!(calls.scorers.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn partially_overlapping_range_delegates_to_exact_query() {
        let (weight, calls, segment) = query_with_truth(SegmentTruth::Maybe, false);
        assert_eq!(weight.scorer(&segment, 1.0).unwrap().doc(), 0);
        assert_eq!(weight.count(&segment).unwrap(), 1);
        assert_eq!(weight.explain(&segment, 0).unwrap().value(), 1.0);
        assert_eq!(calls.scorers.load(Ordering::Relaxed), 1);
        assert_eq!(calls.counts.load(Ordering::Relaxed), 1);
        assert_eq!(calls.explains.load(Ordering::Relaxed), 1);
    }
}

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

use crate::index::reader::index::enable_scoring;
use std::sync::{Arc, OnceLock};
use tantivy::query::{BooleanQuery, Occur, PruningScorer, Query, Scorer, Weight};
use tantivy::{DocAddress, DocId, DocSet, Score, Searcher, SegmentOrdinal, SegmentReader};

#[cfg(any(test, feature = "pg_test"))]
pub mod test_support {
    use std::sync::atomic::AtomicUsize;

    /// Number of deferred per-segment scorers that crossed the actual Tantivy open boundary.
    pub static SCORERS_OPENED: AtomicUsize = AtomicUsize::new(0);
}

/// Lazily builds one [`Weight`] and shares it across a search's segments.
///
/// A scored weight aggregates corpus-level term statistics: `doc_freq` walks every
/// segment's term dictionary. Building the weight per segment therefore costs
/// segments² dictionary lookups per query, which dominates scored scans on
/// many-segment indexes.
pub struct LazyWeight {
    query: Arc<dyn Query>,
    need_scores: bool,
    searcher: Searcher,
    weight: OnceLock<Box<dyn Weight>>,
}

impl LazyWeight {
    pub fn new(query: Arc<dyn Query>, need_scores: bool, searcher: Searcher) -> Self {
        Self {
            query,
            need_scores,
            searcher,
            weight: Default::default(),
        }
    }

    /// Conjoin another query using the same searcher and scoring mode.
    /// Each segment still receives its own scorer.
    pub(super) fn and_query(self: &Arc<Self>, query: Box<dyn Query>) -> Self {
        Self::new(
            Arc::new(BooleanQuery::new(vec![
                (Occur::Must, self.query.box_clone()),
                (Occur::Must, query),
            ])),
            self.need_scores,
            self.searcher.clone(),
        )
    }

    pub(super) fn get(&self) -> &dyn Weight {
        self.weight
            .get_or_init(|| {
                self.query
                    .weight(enable_scoring(self.need_scores, &self.searcher))
                    .expect("weight should be constructable")
            })
            .as_ref()
    }
}

pub struct DeferredScorer {
    weight: Arc<LazyWeight>,
    segment_reader: SegmentReader,
    scorer: OnceLock<Box<dyn PruningScorer>>,
    /// Pruning threshold recorded via [`Self::set_threshold`] before the underlying scorer is
    /// opened, so `pruning_scorer` receives it before its initial `advance()`.
    initial_threshold: Score,
}

impl DeferredScorer {
    pub fn new(weight: Arc<LazyWeight>, segment_reader: SegmentReader) -> Self {
        Self {
            weight,
            segment_reader,
            scorer: Default::default(),
            initial_threshold: Score::MIN,
        }
    }

    #[track_caller]
    #[inline(always)]
    fn scorer_mut(&mut self) -> &mut dyn PruningScorer {
        self.scorer();
        self.scorer
            .get_mut()
            .expect("deferred scorer should have been initialized")
    }

    #[track_caller]
    #[inline(always)]
    fn scorer(&self) -> &dyn PruningScorer {
        self.scorer.get_or_init(|| {
            #[cfg(any(test, feature = "pg_test"))]
            test_support::SCORERS_OPENED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.weight
                .get()
                .pruning_scorer(&self.segment_reader, 1.0, self.initial_threshold)
                .expect("pruning scorer should be constructable")
        })
    }

    fn set_threshold(&mut self, threshold: Score) {
        if let Some(scorer) = self.scorer.get_mut() {
            scorer.set_threshold(threshold);
        } else {
            // Avoid opening the underlying scorer just to record a threshold: opening with
            // `Score::MIN` runs an initial unpruned `advance()` (evaluating filters on a full
            // warmup batch) before `set_threshold` takes effect.
            self.initial_threshold = threshold;
        }
    }
}

impl DocSet for DeferredScorer {
    #[inline(always)]
    fn advance(&mut self) -> DocId {
        self.scorer_mut().advance()
    }

    #[inline(always)]
    fn doc(&self) -> DocId {
        self.scorer().doc()
    }

    fn size_hint(&self) -> u32 {
        self.scorer().size_hint()
    }
}

impl Scorer for DeferredScorer {
    #[inline(always)]
    fn score(&mut self) -> Score {
        self.scorer_mut().score()
    }
}

pub struct ScorerIter {
    deferred: DeferredScorer,
    segment_ord: SegmentOrdinal,
    segment_reader: SegmentReader,
}

impl ScorerIter {
    pub fn new(
        scorer: DeferredScorer,
        segment_ord: SegmentOrdinal,
        segment_reader: SegmentReader,
    ) -> Self {
        Self {
            deferred: scorer,
            segment_ord,
            segment_reader,
        }
    }

    pub fn segment_ord(&self) -> SegmentOrdinal {
        self.segment_ord
    }

    pub fn segment_id(&self) -> tantivy::index::SegmentId {
        self.segment_reader.segment_id()
    }

    /// Returns the estimated number of documents that will be yielded by this iterator.
    ///
    /// This is used for query planning statistics and uses Tantivy's `size_hint`.
    pub fn estimated_doc_count(&self) -> u32 {
        self.deferred.size_hint()
    }

    pub fn set_threshold(&mut self, threshold: Score) {
        self.deferred.set_threshold(threshold);
    }

    /// Returns an upper bound on the number of documents this iterator can yield starting from
    /// `from_doc` (inclusive) through the end of the segment.
    pub fn remaining_docs_upper_bound(&self, from_doc: DocId) -> usize {
        self.segment_reader
            .max_doc()
            .saturating_sub(from_doc)
            .min(self.segment_reader.num_docs()) as usize
    }
}

impl Iterator for ScorerIter {
    type Item = (Score, DocAddress);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let doc_id = self.deferred.doc();

            if doc_id == tantivy::TERMINATED {
                // we've read all the docs
                return None;
            } else if self
                .segment_reader
                .alive_bitset()
                .map(|alive_bitset| alive_bitset.is_alive(doc_id))
                // if there's no alive_bitset, the doc is alive
                .unwrap_or(true)
            {
                // this doc is alive
                let score = self.deferred.score();
                let this = (score, DocAddress::new(self.segment_ord, doc_id));

                // move to the next doc for the next iteration
                self.deferred.advance();

                // return the live doc
                return Some(this);
            }

            // this doc isn't alive, move to the next doc and loop around
            self.deferred.advance();
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // NOTE: We do not implement size_hint for `ScorerIter`, because the implementation of
        // `Scorer::size_hint` can take a lot longer to execute than is usually expected from
        // `Iterator::size_hint`. We also never consume a `ScorerIter` in a way that requires an
        // accurate size: when consuming for Top K, we consume a precise amount, and in all other
        // cases the iterator is consumed as streaming.
        (0, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tantivy::columnar::ColumnType;
    use tantivy::query::{TermPredicate, TermPredicateQuery, TermQuery, TinySet};
    use tantivy::schema::{FAST, IndexRecordOption, STRING, Schema, TEXT};
    use tantivy::{Index, ProbeCost, Term, doc};

    #[derive(Debug)]
    struct CountingMatchAllPredicate {
        calls: Arc<AtomicUsize>,
    }

    impl TermPredicate for CountingMatchAllPredicate {
        fn filter_bytes(&self, _data: &[u8], offsets: &[u32], out: &mut [TinySet]) {
            self.calls.fetch_add(1, Ordering::Relaxed);
            let count = offsets.len().saturating_sub(1);
            for i in 0..count {
                out[i / 64].insert_mut((i % 64) as u32);
            }
        }

        fn filter_monotonic_u64s(
            &self,
            _col_type: ColumnType,
            values: &[u64],
            out: &mut [TinySet],
        ) {
            self.calls.fetch_add(1, Ordering::Relaxed);
            for i in 0..values.len() {
                out[i / 64].insert_mut((i % 64) as u32);
            }
        }

        fn probe_cost(&self) -> ProbeCost {
            ProbeCost::per_doc(5)
        }
    }

    #[test]
    fn deferred_scorer_applies_threshold_before_opening() {
        let mut schema_builder = Schema::builder();
        let title = schema_builder.add_text_field("title", TEXT);
        let key = schema_builder.add_text_field("key", STRING | FAST);
        let schema = schema_builder.build();

        let index = Index::create_in_ram(schema);
        let mut writer = index.writer_with_num_threads(1, 15_000_000).unwrap();
        // Enough distinct high-entropy keys (no common delta prefix) to span > 2 SSTable blocks
        // so `TermSetWeight` chooses lazy `CachedDictPredicateDocSet` under `ExecutionMode::Pruning`.
        for i in 0..500u64 {
            let hi = i.wrapping_mul(0x9e37_79b9_7f4a_7c15);
            let lo = i.wrapping_mul(0xbf58_476d_1ce4_e5b9);
            let k = format!("{hi:016x}-{lo:016x}-{i:08}");
            writer
                .add_document(doc!(title => "alpha beta", key => k))
                .unwrap();
        }
        writer.commit().unwrap();

        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let segment_reader = searcher.segment_readers()[0].clone();

        let calls = Arc::new(AtomicUsize::new(0));
        let predicate = Arc::new(CountingMatchAllPredicate {
            calls: Arc::clone(&calls),
        });
        let disjunction = Box::new(BooleanQuery::new(vec![
            (
                Occur::Should,
                Box::new(TermQuery::new(
                    Term::from_field_text(title, "alpha"),
                    IndexRecordOption::WithFreqs,
                )),
            ),
            (
                Occur::Should,
                Box::new(TermQuery::new(
                    Term::from_field_text(title, "beta"),
                    IndexRecordOption::WithFreqs,
                )),
            ),
        ]));
        let query: Arc<dyn Query> = Arc::new(BooleanQuery::new(vec![
            (Occur::Must, disjunction),
            (
                Occur::Must,
                Box::new(TermPredicateQuery::new(key, predicate)),
            ),
        ]));
        let lazy_weight = Arc::new(LazyWeight::new(query, true, searcher));

        // Setting a high threshold before opening must not initialize the scorer, and once
        // opened the scorer must prune immediately without probing the predicate.
        let mut pruned = DeferredScorer::new(Arc::clone(&lazy_weight), segment_reader.clone());
        pruned.set_threshold(1000.0);
        assert!(pruned.scorer.get().is_none());
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        assert_eq!(pruned.doc(), tantivy::TERMINATED);
        assert_eq!(calls.load(Ordering::Relaxed), 0);

        // Once opened at `Score::MIN`, `set_threshold` still forwards to the live scorer.
        let mut live = DeferredScorer::new(lazy_weight, segment_reader);
        assert_ne!(live.doc(), tantivy::TERMINATED);
        assert!(calls.load(Ordering::Relaxed) > 0);
        live.set_threshold(1000.0);
        assert_eq!(live.advance(), tantivy::TERMINATED);
    }
}

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
use std::sync::OnceLock;
use tantivy::query::{PruningScorer, Query, Scorer};
use tantivy::{DocAddress, DocId, DocSet, Score, Searcher, SegmentOrdinal, SegmentReader};

enum ScorerType {
    Regular(Box<dyn Scorer>),
    Pruning(Box<dyn PruningScorer>),
}

pub struct DeferredScorer {
    query: Box<dyn Query>,
    need_scores: bool,
    pruning: bool,
    segment_reader: SegmentReader,
    searcher: Searcher,
    scorer: OnceLock<ScorerType>,
}

impl DeferredScorer {
    pub fn new(
        query: Box<dyn Query>,
        need_scores: bool,
        segment_reader: SegmentReader,
        searcher: Searcher,
    ) -> Self {
        Self {
            query,
            need_scores,
            pruning: false,
            segment_reader,
            searcher,
            scorer: Default::default(),
        }
    }

    #[track_caller]
    #[inline(always)]
    fn scorer_mut(&mut self) -> &mut dyn Scorer {
        self.scorer();
        match self
            .scorer
            .get_mut()
            .expect("deferred scorer should have been initialized")
        {
            ScorerType::Regular(scorer) => scorer,
            ScorerType::Pruning(pruning) => pruning,
        }
    }

    #[track_caller]
    #[inline(always)]
    fn pruning_scorer_mut(&mut self) -> Option<&mut dyn PruningScorer> {
        self.scorer();
        match self
            .scorer
            .get_mut()
            .expect("deferred scorer should have been initialized")
        {
            ScorerType::Regular(_) => None,
            ScorerType::Pruning(pruning) => Some(pruning),
        }
    }

    #[track_caller]
    #[inline(always)]
    fn scorer(&self) -> &dyn Scorer {
        let scorer = self.scorer.get_or_init(|| {
            let weight = self
                .query
                .weight(enable_scoring(self.need_scores, &self.searcher))
                .expect("weight should be constructable");

            if self.pruning {
                let pruning_scorer = weight
                    .pruning_scorer(&self.segment_reader, 1.0, Score::MIN)
                    .expect("pruning scorer should be constructable");
                ScorerType::Pruning(pruning_scorer)
            } else {
                let scorer = weight
                    .scorer(&self.segment_reader, 1.0)
                    .expect("scorer should be constructable");
                ScorerType::Regular(scorer)
            }
        });
        match scorer {
            ScorerType::Regular(scorer) => scorer,
            ScorerType::Pruning(pruning) => pruning,
        }
    }

    /// Should only be called before `self.scorer` is initialized
    pub fn set_pruning(&mut self, pruning: bool) {
        self.pruning = pruning;
    }

    pub fn set_threshold(&mut self, threshold: Score) {
        let scorer = self
            .pruning_scorer_mut()
            .expect("should only call set_threshold when `self.scorer` is a `PruningScorer`");
        scorer.set_threshold(threshold);
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

    /// Update the pruning field on `deferred`. This should only be called before `self.deferred.scorer`
    /// is initialized
    pub fn set_pruning(&mut self, pruning: bool) {
        self.deferred.set_pruning(pruning);
    }

    pub fn set_threshold(&mut self, threshold: Score) {
        self.deferred.set_threshold(threshold);
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

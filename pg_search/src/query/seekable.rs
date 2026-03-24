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

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};
use tantivy::index::SegmentId;
use tantivy::query::{Explanation, Query, Scorer, Weight};
use tantivy::DocSet;
use tantivy::TERMINATED;
use tantivy::{DocId, Result, Score, SegmentReader};

// We don't import SeekDangerResult, we just use whatever the underlying scorer returns since the DocSet trait requires it
use tantivy::SeekDangerResult;

pub struct Thresholds {
    pub min_doc: AtomicU32,
    pub max_doc: AtomicU32,
    pub docs_seeked_past: std::sync::atomic::AtomicU64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            min_doc: AtomicU32::new(0),
            max_doc: AtomicU32::new(u32::MAX),
            docs_seeked_past: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

/// A shared handle that maps each segment to dynamically updated `DocId` thresholds.
#[derive(Default, Clone)]
pub struct SeekHandle {
    thresholds: Arc<RwLock<HashMap<SegmentId, Arc<Thresholds>>>>,
}

impl SeekHandle {
    /// Returns the atomic thresholds for the given segment.
    pub fn get_threshold(&self, segment_id: SegmentId) -> Arc<Thresholds> {
        let mut lock = self.thresholds.write().unwrap();
        lock.entry(segment_id)
            .or_insert_with(|| Arc::new(Thresholds::default()))
            .clone()
    }

    /// Returns the total number of documents seeked past across all segments.
    pub fn docs_seeked_past(&self) -> u64 {
        let lock = self.thresholds.read().unwrap();
        lock.values()
            .map(|t| t.docs_seeked_past.load(Ordering::Relaxed))
            .sum()
    }
}

/// A query wrapper that intercepts `Scorer` iterations to apply dynamic `DocId` thresholds.
pub struct SeekableQuery {
    underlying: Box<dyn Query>,
    seek_handle: SeekHandle,
}

impl Clone for SeekableQuery {
    fn clone(&self) -> Self {
        Self {
            underlying: self.underlying.box_clone(),
            seek_handle: self.seek_handle.clone(),
        }
    }
}

impl std::fmt::Debug for SeekableQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SeekableQuery").finish()
    }
}

impl SeekableQuery {
    pub fn new(underlying: Box<dyn Query>, seek_handle: SeekHandle) -> Self {
        Self {
            underlying,
            seek_handle,
        }
    }
}

impl Query for SeekableQuery {
    fn weight(&self, enable_scoring: tantivy::query::EnableScoring<'_>) -> Result<Box<dyn Weight>> {
        let underlying_weight = self.underlying.weight(enable_scoring)?;
        Ok(Box::new(SeekableWeight {
            underlying: underlying_weight,
            seek_handle: self.seek_handle.clone(),
        }))
    }

    fn query_terms<'a>(
        &'a self,
        field: tantivy::schema::Field,
        segment_reader: &SegmentReader,
        visitor: &mut dyn FnMut(&tantivy::Term, bool),
    ) {
        // Need to transmute the visitor to have the longer lifetime expected by query_terms
        // Since we are not storing the term, this is safe in practice.
        // Or we can just skip delegating `query_terms` since it's an optional optimization for highlighting
        // Actually, Tantivy's `query_terms` is just:
        // fn query_terms<'a>(&'a self, _visitor: &mut dyn FnMut(&Term, bool)) {}
        // So we can implement it if needed, or leave it empty as the default impl does.
        // Wait, Tantivy `query_terms` takes:
        // `fn query_terms(&self, _field: Field, _segment_reader: &SegmentReader, _visitor: &mut dyn FnMut(&Term, bool))`

        // Actually the issue was a generic lifetime 'a. We can just call it with the right types.
        let mut v = |term: &tantivy::Term, b: bool| {
            visitor(term, b);
        };
        self.underlying.query_terms(field, segment_reader, &mut v);
    }
}

pub struct SeekableWeight {
    underlying: Box<dyn Weight>,
    seek_handle: SeekHandle,
}

impl Weight for SeekableWeight {
    fn scorer(&self, reader: &SegmentReader, boost: Score) -> Result<Box<dyn Scorer>> {
        let underlying_scorer = self.underlying.scorer(reader, boost)?;
        let thresholds = self.seek_handle.get_threshold(reader.segment_id());
        Ok(Box::new(SeekableScorer {
            underlying: underlying_scorer,
            thresholds,
        }))
    }

    fn explain(&self, reader: &SegmentReader, doc: DocId) -> Result<Explanation> {
        self.underlying.explain(reader, doc)
    }

    fn count(&self, reader: &SegmentReader) -> Result<u32> {
        self.underlying.count(reader)
    }

    fn for_each(
        &self,
        reader: &SegmentReader,
        callback: &mut dyn FnMut(DocId, Score),
    ) -> Result<()> {
        let mut scorer = self.scorer(reader, 1.0)?;
        let mut doc = scorer.doc();
        while doc != TERMINATED {
            callback(doc, scorer.score());
            doc = scorer.advance();
        }
        Ok(())
    }
}

/// A `Scorer` wrapper that lazily evaluates dynamic `DocId` thresholds published by the query engine
/// (e.g. from DataFusion's `SortMergeJoinExec`) and uses `seek()` to efficiently skip large
/// swaths of irrelevant documents.
pub struct SeekableScorer {
    underlying: Box<dyn Scorer>,
    thresholds: Arc<Thresholds>,
}

impl DocSet for SeekableScorer {
    fn advance(&mut self) -> DocId {
        // We evaluate the threshold *after* advancing the underlying scorer to ensure we are
        // comparing the dynamically updated threshold against the *next* document we intend to read,
        // avoiding off-by-one errors where a threshold update happens mid-batch and gets ignored.
        let mut next_doc = self.underlying.advance();
        if next_doc == TERMINATED {
            return TERMINATED;
        }

        let min_doc = self.thresholds.min_doc.load(Ordering::Relaxed);
        let max_doc = self.thresholds.max_doc.load(Ordering::Relaxed);

        if next_doc < min_doc {
            let skipped = (min_doc - next_doc) as u64;
            self.thresholds
                .docs_seeked_past
                .fetch_add(skipped, Ordering::Relaxed);
            next_doc = self.underlying.seek(min_doc);
            if next_doc == TERMINATED {
                return TERMINATED;
            }
        }

        if next_doc >= max_doc {
            return TERMINATED;
        }

        next_doc
    }

    fn seek(&mut self, target: DocId) -> DocId {
        let min_doc = self.thresholds.min_doc.load(Ordering::Relaxed);
        let max_doc = self.thresholds.max_doc.load(Ordering::Relaxed);

        let actual_target = target.max(min_doc);

        if actual_target > target {
            self.thresholds
                .docs_seeked_past
                .fetch_add((actual_target - target) as u64, Ordering::Relaxed);
        }
        if actual_target >= max_doc {
            return TERMINATED;
        }

        let next_doc = self.underlying.seek(actual_target);
        if next_doc >= max_doc {
            return TERMINATED;
        }
        next_doc
    }

    fn seek_danger(&mut self, target: DocId) -> SeekDangerResult {
        let min_doc = self.thresholds.min_doc.load(Ordering::Relaxed);
        let max_doc = self.thresholds.max_doc.load(Ordering::Relaxed);

        let actual_target = target.max(min_doc);

        if actual_target > target {
            self.thresholds
                .docs_seeked_past
                .fetch_add((actual_target - target) as u64, Ordering::Relaxed);
        }
        if actual_target >= max_doc {
            return SeekDangerResult::SeekLowerBound(TERMINATED);
        }

        let result = self.underlying.seek_danger(actual_target);
        match result {
            SeekDangerResult::Found => SeekDangerResult::Found,
            SeekDangerResult::SeekLowerBound(doc) => {
                if doc >= max_doc {
                    SeekDangerResult::SeekLowerBound(TERMINATED)
                } else {
                    SeekDangerResult::SeekLowerBound(doc)
                }
            }
        }
    }

    fn doc(&self) -> DocId {
        self.underlying.doc()
    }

    fn size_hint(&self) -> u32 {
        self.underlying.size_hint()
    }
}

impl Scorer for SeekableScorer {
    fn score(&mut self) -> Score {
        self.underlying.score()
    }
}

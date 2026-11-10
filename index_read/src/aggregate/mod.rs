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

// Batch size used by both the MVCC visibility collector and the interrupt-check collector.
//
// This is significantly larger than COLLECT_BLOCK_BUFFER_LEN (64) to reduce the overhead
// of visibility checks. Specifically:
// 1. It amortizes the dynamic dispatch overhead of looking up ctids from the fast field.
// 2. It allows `VisibilityChecker` to process ctids in sorted order, which is critical because
//    checking visibility requires acquiring locks on the visibility map (VM) pages and potentially
//    tuple locks on the heap. Accessing these in a sorted, batched manner reduces lock contention
//    and random I/O.
pub const COLLECTOR_BATCH_SIZE: usize = 2048;

pub mod interrupt_collector {
    use tantivy::collector::{Collector, SegmentCollector};
    use tantivy::{DocId, Score, SegmentOrdinal, SegmentReader};

    /// A collector wrapper that periodically checks for Postgres query cancellation
    /// during Tantivy's collection loop, which otherwise runs without yielding control.
    pub struct InterruptableCollector<C: Collector> {
        inner: C,
    }

    impl<C: Collector> InterruptableCollector<C> {
        pub fn new(inner: C) -> Self {
            Self { inner }
        }
    }

    impl<C: Collector> Collector for InterruptableCollector<C> {
        type Fruit = C::Fruit;
        type Child = InterruptableSegmentCollector<C::Child>;

        fn for_segment(
            &self,
            segment_local_id: SegmentOrdinal,
            segment: &SegmentReader,
        ) -> tantivy::Result<Self::Child> {
            Ok(InterruptableSegmentCollector {
                inner: self.inner.for_segment(segment_local_id, segment)?,
                docs_since_check: 0,
            })
        }

        fn requires_scoring(&self) -> bool {
            self.inner.requires_scoring()
        }

        fn supports_bitmap_collection(&self) -> bool {
            self.inner.supports_bitmap_collection()
        }

        fn merge_fruits(
            &self,
            segment_fruits: Vec<<Self::Child as SegmentCollector>::Fruit>,
        ) -> tantivy::Result<Self::Fruit> {
            self.inner.merge_fruits(segment_fruits)
        }
    }

    pub struct InterruptableSegmentCollector<SC: SegmentCollector> {
        inner: SC,
        docs_since_check: usize,
    }

    impl<SC: SegmentCollector> InterruptableSegmentCollector<SC> {
        #[inline]
        fn maybe_check_interrupt(&mut self, docs: usize) {
            self.docs_since_check += docs;
            if self.docs_since_check >= super::COLLECTOR_BATCH_SIZE {
                self.docs_since_check = 0;
                pgrx::check_for_interrupts!();
            }
        }
    }

    impl<SC: SegmentCollector> SegmentCollector for InterruptableSegmentCollector<SC> {
        type Fruit = SC::Fruit;

        fn collect(&mut self, doc: DocId, score: Score) {
            self.maybe_check_interrupt(1);
            self.inner.collect(doc, score);
        }

        fn collect_block(&mut self, docs: &[DocId]) {
            self.maybe_check_interrupt(docs.len());
            self.inner.collect_block(docs);
        }

        fn supports_bitmap_collection(&self) -> bool {
            self.inner.supports_bitmap_collection()
        }

        fn collect_bitmap(&mut self, base: DocId, mask: &tantivy::DocIdBitmap) {
            self.maybe_check_interrupt(tantivy::BLOCK_WINDOW as usize);
            self.inner.collect_bitmap(base, mask);
        }

        fn harvest(self) -> Self::Fruit {
            self.inner.harvest()
        }
    }
}

pub mod mvcc_collector {
    use parking_lot::Mutex;
    use std::sync::Arc;
    use tantivy::collector::{Collector, SegmentCollector};

    use crate::postgres::heap::{VisibilityChecker, VisibilityMask};
    use tantivy::{DocId, Score, SegmentOrdinal, SegmentReader};

    use super::COLLECTOR_BATCH_SIZE as BATCH_SIZE;

    pub struct MVCCFilterCollector<C: Collector> {
        inner: C,
        pub lock: Arc<Mutex<VisibilityChecker>>,
    }

    unsafe impl<C: Collector> Send for MVCCFilterCollector<C> {}
    unsafe impl<C: Collector> Sync for MVCCFilterCollector<C> {}

    impl<C: Collector> Collector for MVCCFilterCollector<C> {
        type Fruit = C::Fruit;
        type Child = MVCCFilterSegmentCollector<C::Child>;

        fn for_segment(
            &self,
            segment_local_id: SegmentOrdinal,
            segment: &SegmentReader,
        ) -> tantivy::Result<Self::Child> {
            let inner = self.inner.for_segment(segment_local_id, segment)?;
            let requires_scoring = self.inner.requires_scoring();

            Ok(MVCCFilterSegmentCollector {
                inner,
                visibility: SegmentVisibilityState::Unchecked(Arc::clone(&self.lock)),
                segment_ord: segment_local_id,
                doc_buffer: Vec::new(),
                score_buffer: Vec::new(),
                visibility_buffer: Vec::new(),
                requires_scoring,
            })
        }

        fn requires_scoring(&self) -> bool {
            self.inner.requires_scoring()
        }

        fn supports_bitmap_collection(&self) -> bool {
            self.inner.supports_bitmap_collection()
        }

        fn merge_fruits(
            &self,
            segment_fruits: Vec<<Self::Child as SegmentCollector>::Fruit>,
        ) -> tantivy::Result<Self::Fruit> {
            self.inner.merge_fruits(segment_fruits)
        }
    }

    #[allow(clippy::arc_with_non_send_sync)]
    impl<C: Collector> MVCCFilterCollector<C> {
        pub fn new(wrapped: C, vischeck: VisibilityChecker) -> Self {
            Self {
                inner: wrapped,
                lock: Arc::new(Mutex::new(vischeck)),
            }
        }
    }

    /// Per-segment visibility state for [`MVCCFilterSegmentCollector`].
    ///
    /// Tantivy calls [`Collector::for_segment`] before evaluating the segment's query scorer.
    /// Deferring [`VisibilityChecker::for_segment_arc`] and batch-buffer allocation until the
    /// first matching document arrives avoids reading `.fast` (`_per_block_ctid_ranges` / `__ctid`)
    /// or heap visibility map pages on segments where the query matches 0 documents.
    enum SegmentVisibilityState {
        Unchecked(Arc<Mutex<VisibilityChecker>>),
        AllVisible,
        NeedsChecks(Arc<Mutex<VisibilityChecker>>),
    }

    pub struct MVCCFilterSegmentCollector<SC: SegmentCollector> {
        inner: SC,
        visibility: SegmentVisibilityState,
        segment_ord: SegmentOrdinal,

        // Incoming buffers
        doc_buffer: Vec<DocId>,
        score_buffer: Vec<Score>,

        // Entry i records whether doc_buffer[i] is visible to the query snapshot.
        visibility_buffer: Vec<bool>,

        requires_scoring: bool,
    }
    unsafe impl<C: SegmentCollector> Send for MVCCFilterSegmentCollector<C> {}
    unsafe impl<C: SegmentCollector> Sync for MVCCFilterSegmentCollector<C> {}

    impl<SC: SegmentCollector> MVCCFilterSegmentCollector<SC> {
        fn resolve_lock(&mut self) -> Option<&Mutex<VisibilityChecker>> {
            if let SegmentVisibilityState::Unchecked(checker) = &self.visibility {
                let lock = VisibilityChecker::for_segment_arc(checker, self.segment_ord)
                    .expect("failed to check segment visibility");
                self.visibility = match lock {
                    Some(lock) => {
                        // All-visible segments forward documents directly and need no batch buffers.
                        self.doc_buffer.reserve(BATCH_SIZE);
                        if self.requires_scoring {
                            self.score_buffer.reserve(BATCH_SIZE);
                        }
                        self.visibility_buffer.reserve(BATCH_SIZE);
                        SegmentVisibilityState::NeedsChecks(lock)
                    }
                    None => SegmentVisibilityState::AllVisible,
                };
            }
            match &self.visibility {
                SegmentVisibilityState::AllVisible => None,
                SegmentVisibilityState::NeedsChecks(lock) => Some(lock.as_ref()),
                SegmentVisibilityState::Unchecked(_) => unreachable!(),
            }
        }

        fn flush(&mut self) {
            if self.doc_buffer.is_empty() {
                return;
            }

            // Determine which docs are visible.
            let SegmentVisibilityState::NeedsChecks(lock) = &self.visibility else {
                unreachable!("buffered docs need visibility checks");
            };
            let mut vischeck = lock.lock();
            let mask = vischeck.check_segment_docs_mask(
                self.segment_ord,
                &self.doc_buffer,
                &mut self.visibility_buffer,
            );
            drop(vischeck);

            match mask {
                VisibilityMask::All { .. } => {
                    // Forward directly without filtering when all buffered docs are visible.
                    if self.requires_scoring {
                        for (&doc, &score) in self.doc_buffer.iter().zip(self.score_buffer.iter()) {
                            self.inner.collect(doc, score);
                        }
                    } else {
                        self.inner.collect_block(&self.doc_buffer);
                    }
                }
                VisibilityMask::Some(mask) => {
                    // In-place compaction of visible docs.
                    let mut write_idx = 0;
                    for (read_idx, &is_vis) in mask.iter().enumerate() {
                        if is_vis {
                            self.doc_buffer[write_idx] = self.doc_buffer[read_idx];
                            if self.requires_scoring {
                                self.score_buffer[write_idx] = self.score_buffer[read_idx];
                            }
                            write_idx += 1;
                        }
                    }
                    self.doc_buffer.truncate(write_idx);
                    if self.requires_scoring {
                        self.score_buffer.truncate(write_idx);
                        for (&doc, &score) in self.doc_buffer.iter().zip(self.score_buffer.iter()) {
                            self.inner.collect(doc, score);
                        }
                    } else if !self.doc_buffer.is_empty() {
                        self.inner.collect_block(&self.doc_buffer);
                    }
                }
            }

            self.doc_buffer.clear();
            if self.requires_scoring {
                self.score_buffer.clear();
            }
        }
    }

    impl<SC: SegmentCollector> SegmentCollector for MVCCFilterSegmentCollector<SC> {
        type Fruit = SC::Fruit;

        fn collect(&mut self, doc: DocId, score: Score) {
            if self.resolve_lock().is_none() {
                self.inner.collect(doc, score);
                return;
            }
            self.doc_buffer.push(doc);
            if self.requires_scoring {
                self.score_buffer.push(score);
            }

            if self.doc_buffer.len() >= BATCH_SIZE {
                self.flush();
            }
        }

        fn collect_block(&mut self, docs: &[DocId]) {
            if self.resolve_lock().is_none() {
                self.inner.collect_block(docs);
                return;
            }
            self.doc_buffer.extend_from_slice(docs);
            if self.requires_scoring {
                // collect_block does not provide scores, but we must maintain score_buffer alignment.
                // We pad with 0.0 or equivalent.
                self.score_buffer.resize(self.doc_buffer.len(), 0.0);
            }

            if self.doc_buffer.len() >= BATCH_SIZE {
                self.flush();
            }
        }

        fn supports_bitmap_collection(&self) -> bool {
            self.inner.supports_bitmap_collection()
        }

        fn collect_bitmap(&mut self, base: DocId, mask: &tantivy::DocIdBitmap) {
            let segment_ord = self.segment_ord;
            let Some(lock) = self.resolve_lock() else {
                self.inner.collect_bitmap(base, mask);
                return;
            };
            let (visible, dirty) = lock
                .lock()
                .partition_bitmap_visibility(segment_ord, base, mask);
            if visible.iter().any(|word| !word.is_empty()) {
                // Bitmap collection is count-only, so buffered IDs can be counted later.
                self.inner.collect_bitmap(base, &visible);
            }
            tantivy::DocSetBatch::Bitmap(base, &dirty)
                .for_each_doc_block(|docs| self.collect_block(docs));
        }

        fn harvest(mut self) -> Self::Fruit {
            self.flush();
            self.inner.harvest()
        }
    }
    #[cfg(any(test, feature = "pg_test"))]
    #[pgrx::pg_schema]
    mod tests {
        use super::*;
        use crate::postgres::rel::PgSearchRelation;
        use pgrx::prelude::*;

        #[derive(Default)]
        struct BatchCount {
            count: usize,
            batches: Vec<usize>,
        }

        impl SegmentCollector for BatchCount {
            type Fruit = (usize, Vec<usize>);

            fn collect(&mut self, _: DocId, _: Score) {
                unreachable!();
            }

            fn collect_block(&mut self, docs: &[DocId]) {
                assert!(docs.is_sorted());
                assert!(docs.iter().all(|doc| doc % 1024 < 3));
                self.count += docs.len();
                self.batches.push(docs.len());
            }

            fn supports_bitmap_collection(&self) -> bool {
                true
            }

            fn collect_bitmap(&mut self, _: DocId, mask: &tantivy::DocIdBitmap) {
                self.count += mask.iter().map(|word| word.len() as usize).sum::<usize>();
            }

            fn harvest(self) -> Self::Fruit {
                (self.count, self.batches)
            }
        }

        #[pg_test]
        fn bitmap_collection_preserves_pending_visibility_batches() {
            Spi::run("CREATE TABLE bitmap_batch_visibility (id int)").unwrap();
            let oid =
                Spi::get_one::<pg_sys::Oid>("SELECT 'bitmap_batch_visibility'::regclass::oid")
                    .unwrap()
                    .unwrap();
            let relation = PgSearchRelation::open(oid);
            let checker = VisibilityChecker::with_rel_and_snap(&relation, unsafe {
                pg_sys::GetActiveSnapshot()
            })
            .with_check_visibility(false);
            let mut collector = MVCCFilterSegmentCollector {
                inner: BatchCount::default(),
                visibility: SegmentVisibilityState::NeedsChecks(
                    MVCCFilterCollector::new(tantivy::collector::Count, checker).lock,
                ),
                segment_ord: 0,
                doc_buffer: Vec::with_capacity(BATCH_SIZE),
                score_buffer: Vec::new(),
                visibility_buffer: Vec::with_capacity(BATCH_SIZE),
                requires_scoring: false,
            };
            let mut visible = [tantivy_common::TinySet::range_greater_or_equal(0); 16];
            visible[0] = tantivy_common::TinySet::range_greater_or_equal(3);
            for window in 0..1000 {
                let base = window * 1024;
                collector.collect_block(&[base, base + 1, base + 2]);
                collector.collect_bitmap(base, &visible);
            }
            assert_eq!(collector.inner.batches, [2049]);
            assert_eq!(collector.doc_buffer.len(), 951);
            let (count, batches) = collector.harvest();
            assert_eq!(count, 1000 * 1024);
            assert_eq!(batches, [2049, 951]);
        }
    }
}

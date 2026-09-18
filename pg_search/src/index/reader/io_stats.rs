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

//! Attributes Postgres buffer hits/reads to tantivy segment components, keyed
//! by [`tantivy::index::SegmentComponent`]. For vector search, `centroids`
//! reads are routing and `vec` reads are probing; the text components
//! (`term`, `idx`, `fast`, ...) are counted the same way.
//!
//! Like `block_tracker`, this is compiled out unless the `io_stats` feature is
//! enabled, in which case the per-segment counters are merged into the
//! `Segment Info` JSON shown by `EXPLAIN (ANALYZE, VERBOSE)`.

#[cfg(feature = "io_stats")]
mod imp {
    use crate::postgres::storage::block::bm25_max_free_space;
    use pgrx::pg_sys;
    use serde_json::json;
    use std::cell::RefCell;
    use std::collections::{BTreeMap, BTreeSet};
    use std::ops::Range;
    use tantivy::index::{SegmentComponent, SegmentId};
    use tantivy::postings::diagnostics::{self, ReadKind};

    #[derive(Debug, Default, serde::Serialize)]
    struct IoCounters {
        blks_hit: u64,
        blks_read: u64,
        read_calls: u64,
        requested_bytes: u64,
        distinct_buffer_pages: usize,
        #[serde(skip)]
        buffer_pages: BTreeSet<(u32, i32, u32)>,
    }

    #[derive(Default)]
    struct PageReads {
        requested_bytes: usize,
        calls: usize,
        pages: BTreeSet<(u32, usize)>,
    }

    impl PageReads {
        fn add(&mut self, file: u32, range: Range<usize>) {
            self.requested_bytes += range.len();
            self.calls += 1;
            if !range.is_empty() {
                let size = bm25_max_free_space();
                self.pages
                    .extend((range.start / size..=(range.end - 1) / size).map(|ord| (file, ord)));
            }
        }

        fn summary(&self) -> serde_json::Value {
            json!({"requested_bytes": self.requested_bytes, "read_calls": self.calls,
                "distinct_data_pages": self.pages.len()})
        }
    }

    type SegmentIo = BTreeMap<String, IoCounters>;
    type BankedIo = (SegmentId, SegmentIo, serde_json::Value);

    thread_local! {
        static CURRENT: RefCell<SegmentIo> = RefCell::default();
        static POSTINGS: RefCell<BTreeMap<&'static str, PageReads>> = RefCell::default();
        static ACTIVE: RefCell<Option<String>> = const { RefCell::new(None) };
        static PER_SEGMENT: RefCell<Vec<BankedIo>> = RefCell::default();
    }

    struct ActiveGuard(Option<String>);

    impl Drop for ActiveGuard {
        fn drop(&mut self) {
            ACTIVE.replace(self.0.take());
        }
    }

    pub fn record<R>(component: &SegmentComponent, bytes: usize, read: impl FnOnce() -> R) -> R {
        if !crate::gucs::experiment_io_stats() {
            return read();
        }
        let _guard = ActiveGuard(ACTIVE.replace(Some(component.to_string())));
        let (hit0, read0) = snapshot();
        let result = read();
        let (hit1, read1) = snapshot();
        CURRENT.with_borrow_mut(|current| {
            let slot = current.entry(component.to_string()).or_default();
            slot.blks_hit += hit1.saturating_sub(hit0) as u64;
            slot.blks_read += read1.saturating_sub(read0) as u64;
            slot.read_calls += 1;
            slot.requested_bytes += bytes as u64;
        });
        result
    }

    pub fn record_buffer_page(relation: pg_sys::Oid, fork: i32, block: u32) {
        ACTIVE.with_borrow(|active| {
            if let Some(component) = active {
                CURRENT.with_borrow_mut(|current| {
                    let slot = current.entry(component.clone()).or_default();
                    slot.buffer_pages.insert((relation.to_u32(), fork, block));
                    slot.distinct_buffer_pages = slot.buffer_pages.len();
                });
            }
        });
    }

    pub fn record_postings_range(
        component: &SegmentComponent,
        file: u32,
        start: usize,
        bytes: &[u8],
    ) {
        if !crate::gucs::experiment_io_stats() || *component != SegmentComponent::Postings {
            return;
        }
        let end = start + bytes.len();
        POSTINGS.with_borrow_mut(|reads| {
            reads.entry("all").or_default().add(file, start..end);
            match diagnostics::read_kind() {
                ReadKind::Eager { doc_freq } => {
                    let split = start
                        + diagnostics::metadata_len(doc_freq, bytes)
                            .expect("valid postings metadata");
                    if split != start {
                        reads.entry("metadata").or_default().add(file, start..split);
                    }
                    reads.entry("payload").or_default().add(file, split..end);
                }
                ReadKind::Header | ReadKind::Skips => {
                    reads.entry("metadata").or_default().add(file, start..end);
                }
                ReadKind::Payload => {
                    reads.entry("payload").or_default().add(file, start..end);
                }
                ReadKind::Other => {
                    reads
                        .entry("unclassified")
                        .or_default()
                        .add(file, start..end);
                }
            }
        });
    }

    fn snapshot() -> (i64, i64) {
        unsafe {
            let usage = std::ptr::addr_of!(pg_sys::pgBufferUsage).read();
            (usage.shared_blks_hit, usage.shared_blks_read)
        }
    }

    pub fn reset() {
        CURRENT.take();
        POSTINGS.take();
        PER_SEGMENT.take();
        diagnostics::reset(crate::gucs::experiment_io_stats());
    }

    pub fn end_segment(segment_id: SegmentId) {
        let current = CURRENT.take();
        let reads = POSTINGS.take();
        let overlap = reads
            .get("metadata")
            .zip(reads.get("payload"))
            .map_or(0, |(metadata, payload)| {
                metadata.pages.intersection(&payload.pages).count()
            });
        let reads: BTreeMap<_, _> = reads
            .iter()
            .map(|(name, value)| (*name, value.summary()))
            .collect();
        let postings = json!({"blocks": diagnostics::take(), "reads": reads,
            "metadata_payload_shared_pages": overlap, "data_bytes_per_page": bm25_max_free_space()});
        PER_SEGMENT.with_borrow_mut(|bank| bank.push((segment_id, current, postings)));
    }

    pub fn attach(segment_info: &mut BTreeMap<SegmentId, serde_json::Value>) {
        for (segment_id, io, postings) in PER_SEGMENT.take() {
            if io.is_empty() {
                continue;
            }
            if let serde_json::Value::Object(map) =
                segment_info.entry(segment_id).or_insert_with(|| json!({}))
            {
                map.insert("io".to_string(), json!(io));
                map.insert("postings_diagnostics".to_string(), postings);
            }
        }
    }
}

#[cfg(not(feature = "io_stats"))]
mod imp {
    use std::collections::BTreeMap;
    use tantivy::index::{SegmentComponent, SegmentId};

    #[inline(always)]
    pub fn record<R>(_component: &SegmentComponent, _bytes: usize, read: impl FnOnce() -> R) -> R {
        read()
    }

    #[inline(always)]
    pub fn reset() {}

    #[inline(always)]
    pub fn end_segment(_segment_id: SegmentId) {}

    #[inline(always)]
    pub fn attach(_segment_info: &mut BTreeMap<SegmentId, serde_json::Value>) {}
}

pub use imp::{attach, end_segment, record, reset};

#[cfg(feature = "io_stats")]
pub use imp::{record_buffer_page, record_postings_range};

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
//! by [`SegmentComponent`]. For vector search, `centroids` reads are routing
//! and `vec` reads are probing; the text components (`term`, `idx`, `fast`,
//! ...) are counted the same way.
//!
//! Like `block_tracker`, this is compiled out unless the `io_stats` feature is
//! enabled, in which case the per-segment counters are merged into the
//! `Segment Info` JSON shown by `EXPLAIN (ANALYZE, VERBOSE)`.

#[cfg(feature = "io_stats")]
mod imp {
    use pgrx::pg_sys;
    use serde_json::json;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use tantivy::index::{SegmentComponent, SegmentId};

    #[derive(Debug, Default, Clone, Copy, serde::Serialize)]
    struct IoCounters {
        blks_hit: u64,
        blks_read: u64,
    }

    type SegmentIo = BTreeMap<String, IoCounters>;

    thread_local! {
        static CURRENT: RefCell<SegmentIo> = RefCell::default();
        static PER_SEGMENT: RefCell<Vec<(SegmentId, SegmentIo)>> = RefCell::default();
    }

    pub fn record<R>(component: &SegmentComponent, read: impl FnOnce() -> R) -> R {
        let (hit0, read0) = snapshot();
        let result = read();
        let (hit1, read1) = snapshot();
        CURRENT.with_borrow_mut(|current| {
            let slot = current.entry(component.to_string()).or_default();
            slot.blks_hit += hit1.saturating_sub(hit0) as u64;
            slot.blks_read += read1.saturating_sub(read0) as u64;
        });
        result
    }

    fn snapshot() -> (i64, i64) {
        unsafe {
            let usage = std::ptr::addr_of!(pg_sys::pgBufferUsage).read();
            (usage.shared_blks_hit, usage.shared_blks_read)
        }
    }

    /// Forget any counts from outside a segment-collection window.
    pub fn reset() {
        CURRENT.take();
        PER_SEGMENT.take();
    }

    /// Close the current segment's collection window, banking its counters.
    pub fn end_segment(segment_id: SegmentId) {
        let current = CURRENT.take();
        PER_SEGMENT.with_borrow_mut(|per_segment| per_segment.push((segment_id, current)));
    }

    /// Merge the banked per-segment counters into the per-segment JSON built
    /// from tantivy's `ProbeStats`.
    pub fn attach(segment_info: &mut BTreeMap<SegmentId, serde_json::Value>) {
        for (segment_id, io) in PER_SEGMENT.take() {
            if io.is_empty() {
                continue;
            }
            if let Some(serde_json::Value::Object(map)) = segment_info.get_mut(&segment_id) {
                map.insert("io".to_string(), json!(io));
            }
        }
    }
}

#[cfg(not(feature = "io_stats"))]
mod imp {
    use std::collections::BTreeMap;
    use tantivy::index::{SegmentComponent, SegmentId};

    #[inline(always)]
    pub fn record<R>(_component: &SegmentComponent, read: impl FnOnce() -> R) -> R {
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

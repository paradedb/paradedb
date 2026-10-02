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

<<<<<<< HEAD
    thread_local! {
        static CURRENT: RefCell<SegmentIo> = RefCell::default();
        static PER_SEGMENT: RefCell<Vec<(SegmentId, SegmentIo)>> = RefCell::default();
=======
#[derive(Debug, Clone, Default)]
pub struct Trace(Arc<Mutex<Data>>);

#[derive(Debug, Clone)]
pub struct ComponentStats {
    trace: Trace,
    component: String,
}

pub struct Scope {
    trace: Trace,
    before: i64,
}

// NOTE: We intentionally do NOT use `impl_safe_drop!` here because the body only reads the
// `pgBufferUsage` global and updates a Rust side trace behind a `parking_lot` lock, neither of
// which can raise, and it has to run on a panic too or `depth` never comes back down.
impl Drop for Scope {
    fn drop(&mut self) {
        let mut data = self.trace.0.lock();
        data.depth -= 1;
        if data.depth == 0 {
            data.total += snapshot().0.saturating_sub(self.before) as u64;
        }
>>>>>>> 8f66bb1 (ci: require impl_safe_drop! or a stated reason on every impl Drop (#6572))
    }

<<<<<<< HEAD
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
=======
pub struct External {
    trace: Trace,
    before: i64,
    attributed: u64,
    name: &'static str,
}

// NOTE: We intentionally do NOT use `impl_safe_drop!` here because the body, like `Scope`'s, only
// reads `pgBufferUsage` and attributes the difference on the trace.
impl Drop for External {
    fn drop(&mut self) {
        let mut data = self.trace.0.lock();
        if data.depth > 0 {
            let attributed = data
                .components
                .values()
                .sum::<u64>()
                .saturating_sub(self.attributed);
            let hits = (snapshot().0.saturating_sub(self.before) as u64).saturating_sub(attributed);
            *data.components.entry(self.name.into()).or_default() += hits;
        }
>>>>>>> 8f66bb1 (ci: require impl_safe_drop! or a stated reason on every impl Drop (#6572))
    }

<<<<<<< HEAD
    fn snapshot() -> (i64, i64) {
        unsafe {
            let usage = std::ptr::addr_of!(pg_sys::pgBufferUsage).read();
            (usage.shared_blks_hit, usage.shared_blks_read)
=======
pub struct ScanInitGuard {
    trace: Trace,
    before: (i64, i64),
}

// NOTE: We intentionally do NOT use `impl_safe_drop!` here because the body, like `Scope`'s, only
// reads `pgBufferUsage` and records the scan init stage on the trace.
impl Drop for ScanInitGuard {
    fn drop(&mut self) {
        let after = snapshot();
        let mut data = self.trace.0.lock();
        let current = &mut data.current;
        let attributed = current.stages.get("scan_init").copied().unwrap_or_default();
        let direct = IoCounters {
            blks_hit: (after.0.saturating_sub(self.before.0) as u64)
                .saturating_sub(attributed.blks_hit),
            blks_read: (after.1.saturating_sub(self.before.1) as u64)
                .saturating_sub(attributed.blks_read),
        };
        let stage = current.stages.entry("scan_init".into()).or_default();
        stage.blks_hit += direct.blks_hit;
        stage.blks_read += direct.blks_read;
        let component = current
            .scan_init_components
            .entry("executor".into())
            .or_default();
        component.blks_hit += direct.blks_hit;
        component.blks_read += direct.blks_read;
        data.scan_init = false;
        data.preserve_next_reset = true;
    }
}

impl Trace {
    pub fn enter(&self) -> Scope {
        self.0.lock().depth += 1;
        Scope {
            trace: self.clone(),
            before: snapshot().0,
>>>>>>> 8f66bb1 (ci: require impl_safe_drop! or a stated reason on every impl Drop (#6572))
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

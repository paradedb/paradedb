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
    use std::cell::{Cell, RefCell};
    use std::collections::BTreeMap;
    use tantivy::index::{SegmentComponent, SegmentId};
    use tantivy::vector::{Stage, current_vector_stage};

    #[derive(Debug, Default, Clone, Copy, serde::Serialize)]
    struct IoCounters {
        blks_hit: u64,
        blks_read: u64,
    }

    #[derive(Debug, Default)]
    struct SegmentIo {
        components: BTreeMap<String, IoCounters>,
        stages: BTreeMap<String, IoCounters>,
        scan_init_components: BTreeMap<String, IoCounters>,
    }

    impl SegmentIo {
        fn is_empty(&self) -> bool {
            self.components.is_empty()
        }
    }

    thread_local! {
        static CURRENT: RefCell<SegmentIo> = RefCell::default();
        static PER_SEGMENT: RefCell<Vec<(SegmentId, SegmentIo)>> = RefCell::default();
        static PRE_SCAN_INIT: Cell<bool> = const { Cell::new(false) };
        static PRESERVE_NEXT_RESET: Cell<bool> = const { Cell::new(false) };
    }

    pub struct ScanInitGuard {
        hit0: i64,
        read0: i64,
    }

    impl Drop for ScanInitGuard {
        fn drop(&mut self) {
            let (hit1, read1) = snapshot();
            let total = IoCounters {
                blks_hit: hit1.saturating_sub(self.hit0) as u64,
                blks_read: read1.saturating_sub(self.read0) as u64,
            };
            CURRENT.with_borrow_mut(|current| {
                let attributed = current.stages.get("scan_init").copied().unwrap_or_default();
                let direct = IoCounters {
                    blks_hit: total.blks_hit.saturating_sub(attributed.blks_hit),
                    blks_read: total.blks_read.saturating_sub(attributed.blks_read),
                };
                let stage = current.stages.entry("scan_init".to_string()).or_default();
                stage.blks_hit += direct.blks_hit;
                stage.blks_read += direct.blks_read;
                let component = current
                    .scan_init_components
                    .entry("executor".to_string())
                    .or_default();
                component.blks_hit += direct.blks_hit;
                component.blks_read += direct.blks_read;
            });
            PRE_SCAN_INIT.set(false);
            PRESERVE_NEXT_RESET.set(true);
        }
    }

    /// Start the query-level reader-open window. The next collector reset is
    /// suppressed so these counters join the first collected segment.
    pub fn begin_scan_init() -> ScanInitGuard {
        CURRENT.take();
        PER_SEGMENT.take();
        PRESERVE_NEXT_RESET.set(false);
        PRE_SCAN_INIT.set(true);
        let (hit0, read0) = snapshot();
        ScanInitGuard { hit0, read0 }
    }

    pub fn record<R>(component: &SegmentComponent, read: impl FnOnce() -> R) -> R {
        let (hit0, read0) = snapshot();
        let result = read();
        let (hit1, read1) = snapshot();
        let delta = IoCounters {
            blks_hit: hit1.saturating_sub(hit0) as u64,
            blks_read: read1.saturating_sub(read0) as u64,
        };
        CURRENT.with_borrow_mut(|current| {
            let component = component.to_string();
            let slot = current.components.entry(component.clone()).or_default();
            slot.blks_hit += delta.blks_hit;
            slot.blks_read += delta.blks_read;
            let stage = if PRE_SCAN_INIT.get() {
                Some("scan_init".to_string())
            } else {
                stage_name(current_vector_stage())
            };
            if let Some(stage) = stage {
                let stage_slot = current.stages.entry(stage.clone()).or_default();
                stage_slot.blks_hit += delta.blks_hit;
                stage_slot.blks_read += delta.blks_read;
                if stage == "scan_init" {
                    let component_slot = current.scan_init_components.entry(component).or_default();
                    component_slot.blks_hit += delta.blks_hit;
                    component_slot.blks_read += delta.blks_read;
                }
            }
        });
        result
    }

    fn stage_name(stage: Stage) -> Option<String> {
        match stage {
            Stage::Other => None,
            Stage::ScanInit => Some("scan_init".to_string()),
            Stage::QueryPrep => Some("query_prep".to_string()),
            Stage::Routing => Some("routing".to_string()),
            Stage::LayerScan(layer) => Some(format!("layer{layer}_scan")),
            Stage::Boundary(layer) => Some(format!("boundary{layer}")),
            Stage::ExactScan => Some("exact_scan".to_string()),
            Stage::ResultAssembly => Some("result_assembly".to_string()),
            Stage::RerankFetch => Some("rerank_fetch".to_string()),
            Stage::RerankScore => Some("rerank_score".to_string()),
        }
    }

    fn snapshot() -> (i64, i64) {
        unsafe {
            let usage = std::ptr::addr_of!(pg_sys::pgBufferUsage).read();
            (usage.shared_blks_hit, usage.shared_blks_read)
        }
    }

    /// Forget any counts from outside a segment-collection window.
    pub fn reset() {
        if PRESERVE_NEXT_RESET.replace(false) {
            return;
        }
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
                let mut total = IoCounters::default();
                for (component, counters) in io.components {
                    total.blks_hit += counters.blks_hit;
                    total.blks_read += counters.blks_read;
                    let component = component
                        .chars()
                        .map(|character| {
                            if character.is_ascii_alphanumeric() {
                                character.to_ascii_lowercase()
                            } else {
                                '_'
                            }
                        })
                        .collect::<String>();
                    map.insert(
                        format!("io_{component}_buffer_hits"),
                        counters.blks_hit.into(),
                    );
                    map.insert(
                        format!("io_{component}_buffer_reads"),
                        counters.blks_read.into(),
                    );
                }
                map.insert("buffer_hits".to_string(), total.blks_hit.into());
                map.insert("buffer_reads".to_string(), total.blks_read.into());
                map.insert(
                    "blocks_fetched".to_string(),
                    (total.blks_hit + total.blks_read).into(),
                );
                for (name, counters) in io.stages {
                    map.insert(format!("{name}_buffer_hits"), counters.blks_hit.into());
                    map.insert(format!("{name}_buffer_reads"), counters.blks_read.into());
                    if name == "rerank_fetch" {
                        map.insert("rerank_buffer_hits".to_string(), counters.blks_hit.into());
                        map.insert("rerank_buffer_reads".to_string(), counters.blks_read.into());
                        map.insert(
                            "rerank_blocks_fetched".to_string(),
                            (counters.blks_hit + counters.blks_read).into(),
                        );
                    }
                }
                for (component, counters) in io.scan_init_components {
                    let component = component
                        .chars()
                        .map(|character| {
                            if character.is_ascii_alphanumeric() {
                                character.to_ascii_lowercase()
                            } else {
                                '_'
                            }
                        })
                        .collect::<String>();
                    map.insert(
                        format!("scan_init_io_{component}_buffer_hits"),
                        counters.blks_hit.into(),
                    );
                    map.insert(
                        format!("scan_init_io_{component}_buffer_reads"),
                        counters.blks_read.into(),
                    );
                }
            }
        }
    }
}

#[cfg(not(feature = "io_stats"))]
mod imp {
    use std::collections::BTreeMap;
    use tantivy::index::{SegmentComponent, SegmentId};

    pub struct ScanInitGuard;

    #[inline(always)]
    pub fn begin_scan_init() -> ScanInitGuard {
        ScanInitGuard
    }

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

pub use imp::{attach, begin_scan_init, end_segment, record, reset};

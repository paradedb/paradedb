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
//! by [`tantivy::index::SegmentComponent`], with an independent vector-stage axis.
//! Collection requires an active executor instrumentation request. Ordinary queries
//! bypass buffer snapshots, component-name allocation, and counter maps.
//!
//! Like `block_tracker`, this is compiled out unless the `io_stats` feature is
//! enabled, in which case the per-segment counters are merged into the
//! `Segment Info` JSON shown by `EXPLAIN (ANALYZE, VERBOSE)`.
//! Base scans also expose a `Buffer Hits` breakdown for component accesses during
//! `ExecCustomScan`, including reader setup and heap visibility checks.

#[cfg(feature = "io_stats")]
mod imp {
    use pgrx::pg_sys;
    use std::cell::{Cell, RefCell};
    use std::collections::BTreeMap;
    use std::rc::Rc;
    use tantivy::index::{SegmentComponent, SegmentId};
    use tantivy::vector::current_vector_stage;

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

    type SharedData = Rc<RefCell<Data>>;

    #[derive(Default)]
    struct Data {
        total: u64,
        components: BTreeMap<String, u64>,
    }

    #[derive(Clone, Default)]
    struct Context {
        data: Option<SharedData>,
        component: Option<String>,
    }

    thread_local! {
        static ACTIVE: Cell<bool> = const { Cell::new(false) };
        static CONTEXT: RefCell<Context> = RefCell::default();
        static CURRENT: RefCell<SegmentIo> = RefCell::default();
        static PER_SEGMENT: RefCell<Vec<(SegmentId, SegmentIo)>> = RefCell::default();
        static PRE_SCAN_INIT: Cell<bool> = const { Cell::new(false) };
        static PRESERVE_NEXT_RESET: Cell<bool> = const { Cell::new(false) };
    }

    #[derive(Default)]
    pub struct Trace(SharedData);

    pub struct Scope {
        previous: Context,
        total: Option<(SharedData, i64)>,
    }

    impl Drop for Scope {
        fn drop(&mut self) {
            if let Some((data, before)) = &self.total {
                data.borrow_mut().total += snapshot().0.saturating_sub(*before) as u64;
            }
            CONTEXT.set(std::mem::take(&mut self.previous));
        }
    }

    impl Trace {
        pub fn enter(&self) -> Scope {
            let previous = CONTEXT.replace(Context {
                data: Some(self.0.clone()),
                component: None,
            });
            Scope {
                previous,
                total: Some((self.0.clone(), snapshot().0)),
            }
        }

        pub fn hits(&self) -> Vec<(String, u64)> {
            let data = self.0.borrow();
            let mut hits: Vec<_> = data
                .components
                .iter()
                .filter(|(_, hits)| **hits > 0)
                .map(|(component, hits)| {
                    let name = match component.as_str() {
                        "idx" => "Postings",
                        "pos" => "Positions",
                        "term" => "Term Dictionary",
                        "fieldnorm" => "Field Norms",
                        "pnorm" => "Posting Norms",
                        "ctid_map" => "CTID Map",
                        "fast" => "Columnar Fields",
                        "store" => "Document Store",
                        "temp" => "Temporary Store",
                        "del" => "Liveness Bitmap",
                        "stats" => "Segment Statistics",
                        "vec" => "Vectors",
                        "centroids" => "Centroids",
                        other => other,
                    };
                    (name.to_owned(), *hits)
                })
                .collect();
            hits.sort_unstable_by(|left, right| left.0.cmp(&right.0));
            hits.insert(0, ("Total".into(), data.total));
            let other = data.total.saturating_sub(data.components.values().sum());
            if other > 0 {
                hits.push(("Other".into(), other));
            }
            hits
        }
    }

    pub struct External {
        data: Option<SharedData>,
        before: i64,
        name: &'static str,
    }

    impl Drop for External {
        fn drop(&mut self) {
            if let Some(data) = &self.data {
                *data
                    .borrow_mut()
                    .components
                    .entry(self.name.into())
                    .or_default() += snapshot().0.saturating_sub(self.before) as u64;
            }
        }
    }

    pub fn external(name: &'static str) -> External {
        let data = ACTIVE
            .get()
            .then(|| CONTEXT.with_borrow(|context| context.data.clone()))
            .flatten();
        let before = data.as_ref().map_or(0, |_| snapshot().0);
        External { data, before, name }
    }

    /// Scoped to executor callbacks so interleaved/nested plans cannot leave
    /// collection enabled for a subsequent ordinary query. Restored on unwind.
    pub struct InstrumentationGuard(bool);

    impl Drop for InstrumentationGuard {
        fn drop(&mut self) {
            ACTIVE.set(self.0);
        }
    }

    #[inline]
    pub fn instrumentation_request(requested: bool) -> InstrumentationGuard {
        InstrumentationGuard(ACTIVE.replace(requested))
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
    pub fn begin_scan_init() -> Option<ScanInitGuard> {
        if !ACTIVE.get() {
            return None;
        }
        CURRENT.take();
        PER_SEGMENT.take();
        PRESERVE_NEXT_RESET.set(false);
        PRE_SCAN_INIT.set(true);
        let (hit0, read0) = snapshot();
        Some(ScanInitGuard { hit0, read0 })
    }

    #[inline]
    pub fn record<R>(component: &SegmentComponent, read: impl FnOnce() -> R) -> R {
        if !ACTIVE.get() {
            return read();
        }
        let previous = CONTEXT.with_borrow_mut(|context| {
            let previous = context.clone();
            context.component = Some(component.to_string());
            previous
        });
        let _scope = Scope {
            previous,
            total: None,
        };
        read()
    }

    pub fn buffer<R>(read: impl FnOnce() -> R) -> R {
        if !ACTIVE.get() {
            return read();
        }
        let context = CONTEXT.with_borrow(Clone::clone);
        if context.data.is_none() && context.component.is_none() {
            return read();
        }
        let (hit0, read0) = snapshot();
        let result = read();
        let (hit1, read1) = snapshot();
        let delta = IoCounters {
            blks_hit: hit1.saturating_sub(hit0) as u64,
            blks_read: read1.saturating_sub(read0) as u64,
        };
        if let Some(component) = &context.component {
            CURRENT.with_borrow_mut(|current| {
                let slot = current.components.entry(component.clone()).or_default();
                slot.blks_hit += delta.blks_hit;
                slot.blks_read += delta.blks_read;
                let stage = if PRE_SCAN_INIT.get() {
                    Some("scan_init".to_string())
                } else {
                    current_vector_stage().name().map(Into::into)
                };
                if let Some(stage) = stage {
                    let stage_slot = current.stages.entry(stage.clone()).or_default();
                    stage_slot.blks_hit += delta.blks_hit;
                    stage_slot.blks_read += delta.blks_read;
                    if stage == "scan_init" {
                        let component_slot = current
                            .scan_init_components
                            .entry(component.clone())
                            .or_default();
                        component_slot.blks_hit += delta.blks_hit;
                        component_slot.blks_read += delta.blks_read;
                    }
                }
            });
        }
        if let Some(data) = context.data {
            *data
                .borrow_mut()
                .components
                .entry(context.component.unwrap_or_else(|| "Metadata".into()))
                .or_default() += delta.blks_hit;
        }
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
        if !ACTIVE.get() {
            return;
        }
        if PRESERVE_NEXT_RESET.replace(false) {
            return;
        }
        CURRENT.take();
        PER_SEGMENT.take();
    }

    /// Close the current segment's collection window, banking its counters.
    pub fn end_segment(segment_id: SegmentId) {
        if !ACTIVE.get() {
            return;
        }
        let current = CURRENT.take();
        PER_SEGMENT.with_borrow_mut(|per_segment| per_segment.push((segment_id, current)));
    }

    /// Merge the banked per-segment counters into the per-segment JSON built
    /// from tantivy's `ProbeStats`.
    pub fn attach(segment_info: &mut BTreeMap<SegmentId, serde_json::Value>) {
        if !ACTIVE.get() {
            return;
        }
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

    #[cfg(any(test, feature = "pg_test"))]
    #[pgrx::pg_schema]
    mod tests {
        use super::*;

        fn read_buffers(hits: i64, reads: i64) {
            buffer(|| unsafe {
                pg_sys::pgBufferUsage.shared_blks_hit += hits;
                pg_sys::pgBufferUsage.shared_blks_read += reads;
            });
        }

        #[pgrx::pg_test]
        fn nested_components_share_buffer_counts_without_double_counting() {
            let _request = instrumentation_request(true);
            reset();
            let trace = Trace::default();
            {
                let _scope = trace.enter();
                record(&SegmentComponent::Custom("vec".into()), || {
                    read_buffers(2, 1);
                    record(&SegmentComponent::Custom("centroids".into()), || {
                        read_buffers(3, 2);
                    });
                    read_buffers(5, 0);
                });
                record(&SegmentComponent::Custom("ctid_map".into()), || {
                    read_buffers(7, 0);
                });
                read_buffers(11, 0);
            }

            let hits: BTreeMap<_, _> = trace.hits().into_iter().collect();
            assert_eq!(hits["Total"], 28);
            assert_eq!(hits["Vectors"], 7);
            assert_eq!(hits["Centroids"], 3);
            assert_eq!(hits["CTID Map"], 7);
            assert_eq!(hits["Metadata"], 11);
            assert!(!hits.contains_key("Other"));

            let segment = SegmentId::generate_random();
            end_segment(segment);
            let mut info = BTreeMap::from([(segment, serde_json::json!({}))]);
            attach(&mut info);
            let counters = &info[&segment];
            assert_eq!(counters["io_vec_buffer_hits"], hits["Vectors"]);
            assert_eq!(counters["io_vec_buffer_reads"], 1);
            assert_eq!(counters["io_centroids_buffer_hits"], hits["Centroids"]);
            assert_eq!(counters["io_centroids_buffer_reads"], 2);
            assert_eq!(counters["io_ctid_map_buffer_hits"], hits["CTID Map"]);
            assert_eq!(counters["buffer_hits"], 17);
            assert_eq!(counters["buffer_reads"], 3);
            assert_eq!(counters["blocks_fetched"], 20);
        }

        #[pgrx::pg_test]
        fn reader_init_counts_survive_without_a_scan_trace() {
            let _request = instrumentation_request(true);
            {
                let _init = begin_scan_init().unwrap();
                record(&SegmentComponent::Custom("vec".into()), || {
                    read_buffers(2, 3);
                });
                read_buffers(4, 0);
            }
            reset();
            let segment = SegmentId::generate_random();
            end_segment(segment);
            let mut info = BTreeMap::from([(segment, serde_json::json!({}))]);
            attach(&mut info);
            let counters = &info[&segment];
            assert_eq!(counters["io_vec_buffer_hits"], 2);
            assert_eq!(counters["io_vec_buffer_reads"], 3);
            assert_eq!(counters["scan_init_buffer_hits"], 6);
            assert_eq!(counters["scan_init_buffer_reads"], 3);
            assert_eq!(counters["scan_init_io_vec_buffer_hits"], 2);
            assert_eq!(counters["scan_init_io_executor_buffer_hits"], 4);
        }

        #[pgrx::pg_test]
        fn component_context_restores_after_unwind_and_disabled_reads_bypass_collection() {
            let _request = instrumentation_request(true);
            reset();
            let _ = std::panic::catch_unwind(|| {
                record(&SegmentComponent::Custom("vec".into()), || {
                    panic!("test unwind");
                });
            });
            assert!(CONTEXT.with_borrow(|context| context.component.is_none()));
            record(&SegmentComponent::Custom("centroids".into()), || {
                {
                    let _disabled = instrumentation_request(false);
                    read_buffers(13, 7);
                }
                read_buffers(2, 1);
            });
            CURRENT.with_borrow(|current| {
                assert_eq!(current.components.len(), 1);
                assert_eq!(current.components["centroids"].blks_hit, 2);
                assert_eq!(current.components["centroids"].blks_read, 1);
            });
        }

        #[pgrx::pg_test]
        fn bm25_and_vector_explain_share_buffer_accounting() {
            pgrx::Spi::run(
                "SET max_parallel_workers_per_gather = 0;
                 SET enable_seqscan = off;
                 SET paradedb.global_mutable_segment_rows = 0;
                 SET paradedb.vector_stats = on;
                 CREATE TABLE io_accounting (id int PRIMARY KEY, body text, embedding vector(3));
                 INSERT INTO io_accounting
                 SELECT i, 'search engine ' || i, ARRAY[i::real, 1, 0]::vector
                 FROM generate_series(1, 1000) AS i;
                 CREATE INDEX io_accounting_idx ON io_accounting
                 USING paradedb (id, body, embedding vector_l2_ops);",
            )
            .unwrap();

            for (query, vector) in [
                (
                    "SELECT id FROM io_accounting WHERE body ||| 'search' ORDER BY pdb.score(id) DESC LIMIT 5",
                    false,
                ),
                (
                    "SELECT id FROM io_accounting WHERE id @@@ pdb.all() ORDER BY embedding <-> '[1,1,0]' LIMIT 5",
                    true,
                ),
            ] {
                let plan = pgrx::Spi::get_one::<pgrx::Json>(&format!(
                    "EXPLAIN (ANALYZE, VERBOSE, BUFFERS, FORMAT JSON) {query}"
                ))
                .unwrap()
                .unwrap()
                .0;
                let mut nodes = vec![&plan[0]["Plan"]];
                let scan = loop {
                    let node = nodes.pop().expect("ParadeDB scan should be present");
                    if node.get("Buffer Hits").is_some() {
                        break node;
                    }
                    if let Some(children) = node["Plans"].as_array() {
                        nodes.extend(children);
                    }
                };
                let hits = scan["Buffer Hits"].as_object().unwrap();
                let total = hits["Total"].as_u64().unwrap();
                assert!(total > 0);
                assert_eq!(
                    hits.iter()
                        .filter(|(name, _)| name.as_str() != "Total")
                        .map(|(_, value)| value.as_u64().unwrap())
                        .sum::<u64>(),
                    total,
                );
                if vector {
                    let segments: BTreeMap<String, serde_json::Value> =
                        serde_json::from_str(scan["Segment Info"].as_str().unwrap()).unwrap();
                    let vector_hits = segments
                        .values()
                        .map(|stats| stats["io_vec_buffer_hits"].as_u64().unwrap_or(0))
                        .sum::<u64>();
                    assert!(vector_hits > 0);
                    assert!(vector_hits <= hits["Vectors"].as_u64().unwrap());
                    assert!(
                        segments
                            .values()
                            .any(|stats| stats.as_object().unwrap().keys().any(|key| {
                                key != "scan_init_buffer_hits"
                                    && !key.starts_with("io_")
                                    && !key.starts_with("scan_init_io_")
                                    && key.ends_with("_buffer_hits")
                            }))
                    );
                }
            }
        }

        #[pgrx::pg_test]
        fn ordinary_reads_bypass_collection_and_reader_open_snapshots() {
            let _request = instrumentation_request(false);
            CURRENT.take();
            assert_eq!(record(&SegmentComponent::FastFields, || 42), 42);
            assert!(begin_scan_init().is_none());
            assert!(
                CURRENT.with_borrow(
                    |current| current.components.is_empty() && current.stages.is_empty()
                )
            );
        }

        #[pgrx::pg_test]
        fn instrumentation_request_restores_nested_state_and_unwinds() {
            assert!(!ACTIVE.get());
            {
                let _outer = instrumentation_request(true);
                assert!(ACTIVE.get());
                {
                    let _inner = instrumentation_request(false);
                    assert!(!ACTIVE.get());
                }
                assert!(ACTIVE.get());
            }
            assert!(!ACTIVE.get());
            let _ = std::panic::catch_unwind(|| {
                let _request = instrumentation_request(true);
                panic!("test unwind");
            });
            assert!(!ACTIVE.get());
        }
    }
}

#[cfg(not(feature = "io_stats"))]
mod imp {
    use std::collections::BTreeMap;
    use tantivy::index::{SegmentComponent, SegmentId};

    pub struct ScanInitGuard;
    pub struct InstrumentationGuard;

    #[inline(always)]
    pub fn instrumentation_request(_requested: bool) -> InstrumentationGuard {
        InstrumentationGuard
    }

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

pub use imp::{attach, begin_scan_init, end_segment, instrumentation_request, record, reset};

#[cfg(feature = "io_stats")]
pub use imp::{Trace, buffer, external};

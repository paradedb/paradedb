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

//! Per-scan buffer accounting shared by the reader and its storage handles.
//! A missing context bypasses collection. `BUFFERS` enables the context at the
//! executor, and `VERBOSE` additionally exposes per-segment/vector-stage counters.

use parking_lot::Mutex;
use pgrx::pg_sys;
use std::collections::BTreeMap;
use std::sync::Arc;
use tantivy::index::{SegmentComponent, SegmentId};
use tantivy::vector::current_vector_stage;

#[cfg(any(test, feature = "pg_test"))]
thread_local! {
    static VECTOR_READ_REQUESTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Counts vector requests independently of executor instrumentation in storage tests.
#[cfg(any(test, feature = "pg_test"))]
pub(crate) fn vector_read_requests() -> usize {
    VECTOR_READ_REQUESTS.get()
}

/// Counts requests while forwarding storage geometry and bytes unchanged.
#[cfg(any(test, feature = "pg_test"))]
#[derive(Debug)]
pub(crate) struct VectorReadCounter(pub Arc<dyn tantivy::directory::FileHandle>);

#[cfg(any(test, feature = "pg_test"))]
impl tantivy::HasLen for VectorReadCounter {
    fn len(&self) -> usize {
        self.0.len()
    }
}

#[cfg(any(test, feature = "pg_test"))]
impl tantivy::directory::FileHandle for VectorReadCounter {
    fn read_bytes(
        &self,
        range: std::ops::Range<usize>,
    ) -> std::io::Result<tantivy::directory::OwnedBytes> {
        VECTOR_READ_REQUESTS.set(VECTOR_READ_REQUESTS.get() + 1);
        self.0.read_bytes(range)
    }

    fn read_byte(&self, offset: usize) -> std::io::Result<u8> {
        VECTOR_READ_REQUESTS.set(VECTOR_READ_REQUESTS.get() + 1);
        self.0.read_byte(offset)
    }

    fn storage_block_len(&self) -> Option<usize> {
        self.0.storage_block_len()
    }
}

#[derive(Debug, Default, Clone, Copy)]
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

#[derive(Debug, Default)]
struct Data {
    total: u64,
    components: BTreeMap<String, u64>,
    current: SegmentIo,
    per_segment: Vec<(SegmentId, SegmentIo)>,
    scan_init: bool,
    preserve_next_reset: bool,
    depth: usize,
}

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

impl Drop for Scope {
    fn drop(&mut self) {
        let mut data = self.trace.0.lock();
        data.depth -= 1;
        if data.depth == 0 {
            data.total += snapshot().0.saturating_sub(self.before) as u64;
        }
    }
}

pub struct External {
    trace: Trace,
    before: i64,
    attributed: u64,
    name: &'static str,
}

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
    }
}

pub struct ScanInitGuard {
    trace: Trace,
    before: (i64, i64),
}

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
        }
    }

    pub fn component(&self, component: &SegmentComponent) -> ComponentStats {
        ComponentStats {
            trace: self.clone(),
            component: component.to_string(),
        }
    }

    pub fn external(&self, name: &'static str) -> External {
        External {
            trace: self.clone(),
            before: snapshot().0,
            attributed: self.0.lock().components.values().sum(),
            name,
        }
    }

    pub fn begin_scan_init(&self) -> ScanInitGuard {
        let mut data = self.0.lock();
        data.current = SegmentIo::default();
        data.per_segment.clear();
        data.scan_init = true;
        data.preserve_next_reset = false;
        ScanInitGuard {
            trace: self.clone(),
            before: snapshot(),
        }
    }

    pub fn reset(&self) {
        let mut data = self.0.lock();
        if std::mem::take(&mut data.preserve_next_reset) {
            return;
        }
        data.current = SegmentIo::default();
        data.per_segment.clear();
    }

    pub fn end_segment(&self, segment_id: SegmentId) {
        let mut data = self.0.lock();
        let current = std::mem::take(&mut data.current);
        data.per_segment.push((segment_id, current));
    }

    pub fn hits(&self) -> Vec<(String, u64)> {
        let data = self.0.lock();
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
        hits.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        hits.insert(0, ("Total".into(), data.total));
        let other = data.total.saturating_sub(data.components.values().sum());
        if other > 0 {
            hits.push(("Other".into(), other));
        }
        hits
    }

    pub fn attach(&self, segment_info: &mut BTreeMap<SegmentId, serde_json::Value>) {
        let per_segment = std::mem::take(&mut self.0.lock().per_segment);
        for (segment_id, io) in per_segment {
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

impl ComponentStats {
    pub fn buffer<R>(&self, read: impl FnOnce() -> R) -> R {
        let before = snapshot();
        let result = read();
        let after = snapshot();
        let delta = IoCounters {
            blks_hit: after.0.saturating_sub(before.0) as u64,
            blks_read: after.1.saturating_sub(before.1) as u64,
        };
        let mut data = self.trace.0.lock();
        let slot = data
            .current
            .components
            .entry(self.component.clone())
            .or_default();
        slot.blks_hit += delta.blks_hit;
        slot.blks_read += delta.blks_read;
        let stage = if data.scan_init {
            Some("scan_init".into())
        } else {
            current_vector_stage().name()
        };
        if let Some(stage) = stage {
            let slot = data.current.stages.entry(stage.into()).or_default();
            slot.blks_hit += delta.blks_hit;
            slot.blks_read += delta.blks_read;
            if data.scan_init {
                let slot = data
                    .current
                    .scan_init_components
                    .entry(self.component.clone())
                    .or_default();
                slot.blks_hit += delta.blks_hit;
                slot.blks_read += delta.blks_read;
            }
        }
        if data.depth > 0 {
            *data.components.entry(self.component.clone()).or_default() += delta.blks_hit;
        }
        result
    }
}

fn snapshot() -> (i64, i64) {
    unsafe {
        let usage = std::ptr::addr_of!(pg_sys::pgBufferUsage).read();
        (usage.shared_blks_hit, usage.shared_blks_read)
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;

    fn read_buffers(stats: Option<&ComponentStats>, hits: i64, reads: i64) {
        let read = || unsafe {
            pg_sys::pgBufferUsage.shared_blks_hit += hits;
            pg_sys::pgBufferUsage.shared_blks_read += reads;
        };
        match stats {
            Some(stats) => stats.buffer(read),
            None => read(),
        }
    }

    #[pgrx::pg_test]
    fn component_handles_share_buffer_counts_without_double_counting() {
        let trace = Trace::default();
        let vectors = trace.component(&SegmentComponent::Custom("vec".into()));
        let centroids = trace.component(&SegmentComponent::Custom("centroids".into()));
        let ctids = trace.component(&SegmentComponent::Custom("ctid_map".into()));
        {
            let _scope = trace.enter();
            let _metadata = trace.external("Metadata");
            read_buffers(Some(&vectors), 2, 1);
            read_buffers(Some(&centroids), 3, 2);
            read_buffers(Some(&vectors), 5, 0);
            read_buffers(Some(&ctids), 7, 0);
            read_buffers(None, 11, 0);
        }
        let hits: BTreeMap<_, _> = trace.hits().into_iter().collect();
        assert_eq!(hits["Total"], 28);
        assert_eq!(hits["Vectors"], 7);
        assert_eq!(hits["Centroids"], 3);
        assert_eq!(hits["CTID Map"], 7);
        assert_eq!(hits["Metadata"], 11);
        assert!(!hits.contains_key("Other"));
        let segment = SegmentId::generate_random();
        trace.end_segment(segment);
        let mut info = BTreeMap::from([(segment, serde_json::json!({}))]);
        trace.attach(&mut info);
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
    fn reader_init_counts_survive_without_an_execution_scope() {
        let trace = Trace::default();
        let vectors = trace.component(&SegmentComponent::Custom("vec".into()));
        {
            let _init = trace.begin_scan_init();
            read_buffers(Some(&vectors), 2, 3);
            read_buffers(None, 4, 0);
        }
        trace.reset();
        let segment = SegmentId::generate_random();
        trace.end_segment(segment);
        let mut info = BTreeMap::from([(segment, serde_json::json!({}))]);
        trace.attach(&mut info);
        let counters = &info[&segment];
        assert_eq!(counters["io_vec_buffer_hits"], 2);
        assert_eq!(counters["io_vec_buffer_reads"], 3);
        assert_eq!(counters["scan_init_buffer_hits"], 6);
        assert_eq!(counters["scan_init_buffer_reads"], 3);
        assert_eq!(counters["scan_init_io_vec_buffer_hits"], 2);
        assert_eq!(counters["scan_init_io_executor_buffer_hits"], 4);
        assert_eq!(trace.hits(), vec![("Total".into(), 0)]);
    }

    #[pgrx::pg_test]
    fn interleaved_scans_keep_components_and_segment_counters_separate() {
        let outer = Trace::default();
        let inner = Trace::default();
        let vectors = outer.component(&SegmentComponent::Custom("vec".into()));
        let centroids = inner.component(&SegmentComponent::Custom("centroids".into()));
        {
            let _outer = outer.enter();
            read_buffers(Some(&vectors), 2, 1);
            {
                let _inner = inner.enter();
                read_buffers(Some(&centroids), 3, 2);
                inner.reset();
                read_buffers(Some(&centroids), 4, 0);
            }
            read_buffers(Some(&vectors), 5, 0);
        }
        let hits: BTreeMap<_, _> = outer.hits().into_iter().collect();
        assert_eq!(hits["Total"], 14);
        assert_eq!(hits["Vectors"], 7);
        assert_eq!(hits["Other"], 7);
        assert!(!hits.contains_key("Centroids"));
        let hits: BTreeMap<_, _> = inner.hits().into_iter().collect();
        assert_eq!(hits["Total"], 7);
        assert_eq!(hits["Centroids"], 7);
        assert!(!hits.contains_key("Vectors"));
        let segment = SegmentId::generate_random();
        for (trace, component, hits) in [
            (outer, "io_vec_buffer_hits", 7),
            (inner, "io_centroids_buffer_hits", 4),
        ] {
            trace.end_segment(segment);
            let mut info = BTreeMap::from([(segment, serde_json::json!({}))]);
            trace.attach(&mut info);
            assert_eq!(info[&segment][component], hits);
            assert_eq!(info[&segment]["buffer_hits"], hits);
        }
    }

    #[pgrx::pg_test]
    fn execution_scope_restores_after_unwind() {
        let trace = Trace::default();
        let vectors = trace.component(&SegmentComponent::Custom("vec".into()));
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _scope = trace.enter();
            read_buffers(Some(&vectors), 2, 1);
            panic!("test unwind");
        }));
        assert_eq!(trace.0.lock().depth, 0);
        read_buffers(Some(&vectors), 7, 0);
        let hits: BTreeMap<_, _> = trace.hits().into_iter().collect();
        assert_eq!(hits["Total"], 2);
        assert_eq!(hits["Vectors"], 2);
    }

    #[pgrx::pg_test]
    fn repeated_and_nested_execution_scopes_count_once() {
        let trace = Trace::default();
        let vectors = trace.component(&SegmentComponent::Custom("vec".into()));
        {
            let _scope = trace.enter();
            read_buffers(Some(&vectors), 2, 0);
            {
                let _nested = trace.enter();
                read_buffers(Some(&vectors), 3, 0);
            }
        }
        {
            let _rescan = trace.enter();
            read_buffers(Some(&vectors), 5, 0);
        }
        let hits: BTreeMap<_, _> = trace.hits().into_iter().collect();
        assert_eq!(hits["Total"], 10);
        assert_eq!(hits["Vectors"], 10);
        assert!(!hits.contains_key("Other"));
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
            pgrx::Spi::run(query).unwrap();
            for (analyze, buffers, verbose) in [
                (true, true, true),
                (true, true, false),
                (true, false, true),
                (true, false, false),
                (false, true, false),
            ] {
                let plan = pgrx::Spi::get_one::<pgrx::Json>(&format!(
                        "EXPLAIN (ANALYZE {analyze}, VERBOSE {verbose}, BUFFERS {buffers}, FORMAT JSON) {query}"
                    ))
                    .unwrap()
                    .unwrap()
                    .0;
                let mut nodes = vec![&plan[0]["Plan"]];
                let scan = loop {
                    let node = nodes.pop().expect("ParadeDB scan should be present");
                    if node["Index"] == "io_accounting_idx" {
                        break node;
                    }
                    if let Some(children) = node["Plans"].as_array() {
                        nodes.extend(children);
                    }
                };
                let hits = scan
                    .get("Buffer Hits")
                    .map(|hits| hits.as_object().unwrap());
                assert_eq!(hits.is_some(), analyze && buffers);
                if let Some(hits) = hits {
                    let total = hits["Total"].as_u64().unwrap();
                    assert!(total > 0);
                    assert_eq!(
                        hits.iter()
                            .filter(|(name, _)| name.as_str() != "Total")
                            .map(|(_, value)| value.as_u64().unwrap())
                            .sum::<u64>(),
                        total,
                    );
                }
                if vector && analyze && verbose {
                    let segments: BTreeMap<String, serde_json::Value> =
                        serde_json::from_str(scan["Segment Info"].as_str().unwrap()).unwrap();
                    let vector_hits = segments
                        .values()
                        .map(|stats| stats["io_vec_buffer_hits"].as_u64().unwrap_or(0))
                        .sum::<u64>();
                    assert_eq!(vector_hits > 0, buffers);
                    if let Some(hits) = hits {
                        assert!(vector_hits <= hits["Vectors"].as_u64().unwrap());
                    }
                    assert_eq!(
                        segments.values().any(|stats| {
                            stats.as_object().unwrap().keys().any(|key| {
                                key != "scan_init_buffer_hits"
                                    && !key.starts_with("io_")
                                    && !key.starts_with("scan_init_io_")
                                    && key.ends_with("_buffer_hits")
                            })
                        }),
                        buffers
                    );
                }
            }
        }
        let plan = pgrx::Spi::get_one::<pgrx::Json>(
            "EXPLAIN (ANALYZE, VERBOSE, BUFFERS, FORMAT JSON)
             SELECT a.id, b.id
             FROM (SELECT id FROM io_accounting WHERE body ||| 'engine'
                   ORDER BY pdb.score(id) DESC LIMIT 3) a
             CROSS JOIN LATERAL (
                 SELECT id FROM io_accounting WHERE body ||| 'search' AND id > a.id
                 ORDER BY pdb.score(id) DESC LIMIT 2
             ) b",
        )
        .unwrap()
        .unwrap()
        .0;
        let mut nodes = vec![&plan[0]["Plan"]];
        let mut scans = Vec::new();
        while let Some(node) = nodes.pop() {
            if node["Index"] == "io_accounting_idx" {
                scans.push(node);
                let hits = node["Buffer Hits"].as_object().unwrap();
                let total = hits["Total"].as_u64().unwrap();
                assert!(total > 0);
                assert_eq!(
                    total,
                    hits.iter()
                        .filter(|(name, _)| name.as_str() != "Total")
                        .map(|(_, value)| value.as_u64().unwrap())
                        .sum::<u64>()
                );
            }
            if let Some(children) = node["Plans"].as_array() {
                nodes.extend(children);
            }
        }
        assert_eq!(scans.len(), 2);
        assert!(
            scans
                .iter()
                .any(|scan| scan["Actual Loops"].as_u64().unwrap() > 1)
        );
    }
}

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

//! Process-local scan telemetry for EXPLAIN.
//!
//! During a scan (serial or parallel), metrics are recorded only into
//! [`ScanTelemetry`]. Parallel workers publish that local state into DSM at
//! `EndCustomScan` via [`ParallelScanHandle`](super::parallel::ParallelScanHandle).
//! The leader assembles worker telemetry plus shared metadata (e.g. segment
//! claims) at `ShutdownCustomScan` for EXPLAIN.

use std::collections::BTreeMap;

use crate::postgres::ParallelExplainData;

use tantivy::index::SegmentId;

/// Process-local EXPLAIN metrics accumulated while the scan runs.
#[derive(Debug, Default)]
pub struct ScanTelemetry {
    query_count: usize,
    segment_info: BTreeMap<SegmentId, serde_json::Value>,
    /// Set on the leader at Shutdown after assembling DSM explain data.
    parallel_explain: Option<ParallelExplainData>,
}

impl ScanTelemetry {
    pub fn stage_elapsed_ns(&self) -> u64 {
        self.segment_info.values().fold(0u64, |total, value| {
            let Some(fields) = value.as_object() else {
                return total;
            };
            fields.iter().fold(total, |segment_total, (name, value)| {
                let is_stage = matches!(
                    name.as_str(),
                    "scan_init_ns"
                        | "query_prep_ns"
                        | "routing_ns"
                        | "exact_scan_ns"
                        | "result_assembly_ns"
                        | "rerank_fetch_ns"
                        | "rerank_score_ns"
                ) || (name.starts_with("layer") && name.ends_with("_scan_ns"))
                    || (name.starts_with("boundary") && name.ends_with("_ns"));
                if is_stage {
                    segment_total.saturating_add(value.as_u64().unwrap_or_default())
                } else {
                    segment_total
                }
            })
        })
    }

    pub fn add_result_assembly_ns(&mut self, elapsed_ns: u64) {
        let Some((_, value)) = self.segment_info.first_key_value() else {
            return;
        };
        let Some(fields) = value.as_object() else {
            return;
        };
        let current = fields
            .get("result_assembly_ns")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default();
        let first = self
            .segment_info
            .first_entry()
            .expect("the first segment was checked above");
        if let Some(fields) = first.into_mut().as_object_mut() {
            fields.insert(
                "result_assembly_ns".to_string(),
                current.saturating_add(elapsed_ns).into(),
            );
        }
    }

    pub fn record_query(&mut self) {
        self.query_count += 1;
    }

    pub fn query_count(&self) -> usize {
        self.query_count
    }

    pub fn total_query_count(&self) -> usize {
        if let Some(explain_data) = &self.parallel_explain {
            explain_data.total_query_count
        } else {
            self.query_count
        }
    }

    /// Merge per-segment JSON. Last-write-wins per segment id (re-queries
    /// replace rather than append).
    pub fn accumulate_segment_info(&mut self, info: BTreeMap<SegmentId, serde_json::Value>) {
        self.segment_info.extend(info);
    }

    pub fn segment_info(&self) -> &BTreeMap<SegmentId, serde_json::Value> {
        &self.segment_info
    }

    /// EXPLAIN-friendly view: segment short UUID → JSON value.
    pub fn segment_info_for_explain(&self) -> BTreeMap<String, serde_json::Value> {
        self.segment_info
            .iter()
            .map(|(id, value)| (id.short_uuid_string(), value.clone()))
            .collect()
    }

    pub fn parallel_explain(&self) -> Option<&ParallelExplainData> {
        self.parallel_explain.as_ref()
    }

    pub(crate) fn set_parallel_explain(&mut self, data: ParallelExplainData) {
        self.parallel_explain = Some(data);
    }

    pub fn reset(&mut self) {
        self.query_count = 0;
        self.segment_info.clear();
        self.parallel_explain = None;
    }
}

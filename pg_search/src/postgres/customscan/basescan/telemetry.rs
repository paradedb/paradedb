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

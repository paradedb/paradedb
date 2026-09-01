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

use serde_json::{Number, Value};
use tantivy::index::SegmentId;

fn is_segment_inventory_gauge(name: &str) -> bool {
    matches!(name, "segment_rows" | "segment_clusters")
}

fn sum_numbers(left: &Number, right: &Number) -> Number {
    if let (Some(left), Some(right)) = (left.as_u64(), right.as_u64()) {
        return left.saturating_add(right).into();
    }
    if let (Some(left), Some(right)) = (left.as_i64(), right.as_i64()) {
        return left.saturating_add(right).into();
    }
    let sum = left.as_f64().unwrap_or_default() + right.as_f64().unwrap_or_default();
    Number::from_f64(sum).unwrap_or_else(|| {
        Number::from_f64(if sum.is_sign_negative() {
            -f64::MAX
        } else {
            f64::MAX
        })
        .expect("finite f64 must serialize")
    })
}

fn max_numbers(left: &Number, right: &Number) -> Number {
    if let (Some(left), Some(right)) = (left.as_u64(), right.as_u64()) {
        return left.max(right).into();
    }
    if let (Some(left), Some(right)) = (left.as_i64(), right.as_i64()) {
        return left.max(right).into();
    }
    Number::from_f64(
        left.as_f64()
            .unwrap_or(f64::NEG_INFINITY)
            .max(right.as_f64().unwrap_or(f64::NEG_INFINITY)),
    )
    .expect("JSON numbers are finite")
}

fn merge_segment_value(existing: &mut Value, incoming: Value) {
    let incoming_fields = match incoming {
        Value::Object(fields) => fields,
        other => {
            *existing = other;
            return;
        }
    };
    let Some(existing_fields) = existing.as_object_mut() else {
        *existing = Value::Object(incoming_fields);
        return;
    };

    for (name, incoming_value) in incoming_fields {
        let Some(existing_value) = existing_fields.get_mut(&name) else {
            existing_fields.insert(name, incoming_value);
            continue;
        };
        match (existing_value.as_number(), incoming_value.as_number()) {
            (Some(existing_number), Some(incoming_number)) => {
                let merged = if is_segment_inventory_gauge(&name) {
                    max_numbers(existing_number, incoming_number)
                } else {
                    sum_numbers(existing_number, incoming_number)
                };
                *existing_value = Value::Number(merged);
            }
            _ => *existing_value = incoming_value,
        }
    }
}

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

    /// Merge whole-query per-segment JSON from another Top-K collection.
    pub fn accumulate_segment_info(&mut self, info: BTreeMap<SegmentId, serde_json::Value>) {
        for (segment_id, incoming) in info {
            if let Some(existing) = self.segment_info.get_mut(&segment_id) {
                merge_segment_value(existing, incoming);
            } else {
                self.segment_info.insert(segment_id, incoming);
            }
        }
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

#[cfg(test)]
mod tests {
    use super::ScanTelemetry;
    use serde_json::json;
    use std::collections::BTreeMap;
    use tantivy::index::SegmentId;

    fn segment_info(
        segment_id: SegmentId,
        value: serde_json::Value,
    ) -> BTreeMap<SegmentId, serde_json::Value> {
        BTreeMap::from([(segment_id, value)])
    }

    #[test]
    fn repeated_topk_calls_add_counters_and_durations_without_duplicating_inventory() {
        let segment_id = SegmentId::from_bytes([1; 16]);
        let mut telemetry = ScanTelemetry::default();
        telemetry.accumulate_segment_info(segment_info(
            segment_id,
            json!({
                "layer0_scored": 10,
                "layer0_scan_ns": 20,
                "work_charged": 1.25,
                "segment_rows": 100,
                "segment_clusters": 2,
                "termination": "first"
            }),
        ));
        telemetry.accumulate_segment_info(segment_info(
            segment_id,
            json!({
                "layer0_scored": 7,
                "layer0_scan_ns": 30,
                "work_charged": 0.75,
                "segment_rows": 80,
                "segment_clusters": 3,
                "termination": "second"
            }),
        ));

        let value = &telemetry.segment_info()[&segment_id];
        assert_eq!(value["layer0_scored"], 17);
        assert_eq!(value["layer0_scan_ns"], 50);
        assert_eq!(value["work_charged"], 2.0);
        assert_eq!(value["segment_rows"], 100);
        assert_eq!(value["segment_clusters"], 3);
        assert_eq!(value["termination"], "second");
    }

    #[test]
    fn segment_metrics_remain_per_segment() {
        let first = SegmentId::from_bytes([1; 16]);
        let second = SegmentId::from_bytes([2; 16]);
        let mut telemetry = ScanTelemetry::default();
        telemetry.accumulate_segment_info(segment_info(first, json!({"layer0_scored": 5})));
        telemetry.accumulate_segment_info(segment_info(second, json!({"layer0_scored": 8})));

        assert_eq!(telemetry.segment_info()[&first]["layer0_scored"], 5);
        assert_eq!(telemetry.segment_info()[&second]["layer0_scored"], 8);
    }

    #[test]
    fn integer_counter_addition_is_bounded() {
        let segment_id = SegmentId::from_bytes([1; 16]);
        let mut telemetry = ScanTelemetry::default();
        telemetry.accumulate_segment_info(segment_info(
            segment_id,
            json!({"candidates_scored": u64::MAX}),
        ));
        telemetry
            .accumulate_segment_info(segment_info(segment_id, json!({"candidates_scored": 1})));

        assert_eq!(
            telemetry.segment_info()[&segment_id]["candidates_scored"],
            u64::MAX
        );
    }
}

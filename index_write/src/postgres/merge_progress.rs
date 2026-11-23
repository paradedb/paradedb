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

use crate::index::merge_policy::MUTABLE_SEGMENT_LAYER;
use crate::index::writer::index::format_bytes;
use crate::postgres::ps_status::set_ps_display_suffix;
use crate::postgres::storage::block::{CTID_MAP_EXT, STATS_EXT};
use log::{Level, LevelFilter, Log, Metadata, Record, max_level, set_max_level};
use pgrx::pg_sys::{self, BackendState};
use std::cell::RefCell;
use std::ffi::CString;

const MERGER_LOG_TARGET: &str = "tantivy::indexer::merger";
const MERGE_PLUGIN_PREFIX: &str = "merge-plugin: ";

thread_local! {
    static MERGE_STATUS: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub(super) struct MergeProgress {
    previous: Option<String>,
    previous_log_level: LevelFilter,
}

impl MergeProgress {
    pub fn start(batch: usize, batches: usize, layer: u64, segments: usize) -> Self {
        let target = if layer == MUTABLE_SEGMENT_LAYER {
            "mutable conversion".to_owned()
        } else {
            format!("target layer {}", format_bytes(layer))
        };
        let segment_label = if segments == 1 { "segment" } else { "segments" };
        let status =
            format!("merging batch {batch}/{batches}: {segments} {segment_label}; {target}");
        let previous = MERGE_STATUS.replace(Some(status));
        let previous_log_level = max_level();
        set_max_level(previous_log_level.max(LevelFilter::Debug));
        let progress = Self {
            previous,
            previous_log_level,
        };
        progress.report("preparing");
        progress
    }

    pub fn report(&self, phase: &str) {
        report_phase(phase);
    }
}

// We intentionally do NOT use `impl_safe_drop!` here because cleanup only touches Rust state
// and must restore the logging scope during unwinding.
impl Drop for MergeProgress {
    fn drop(&mut self) {
        MERGE_STATUS.set(self.previous.take());
        set_max_level(self.previous_log_level);
    }
}

fn report_phase(phase: &str) {
    let status = MERGE_STATUS.with_borrow(|status| {
        status
            .as_ref()
            .map(|status| CString::new(format!("{status}; phase={phase}")).expect("merge status"))
    });
    if let Some(status) = status {
        unsafe {
            set_ps_display_suffix(status.as_ptr());
            pg_sys::pgstat_report_activity(BackendState::STATE_RUNNING, status.as_ptr());
        }
    }
}

pub(super) struct MergeProgressLogger;

impl Log for MergeProgressLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.target() == MERGER_LOG_TARGET && metadata.level() == Level::Debug
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) || !MERGE_STATUS.with_borrow(Option::is_some) {
            return;
        }
        // Tantivy's pinned merge loop emits this record before each component on the calling thread.
        let message = record.args().to_string();
        let Some(extensions) = message.strip_prefix(MERGE_PLUGIN_PREFIX) else {
            return;
        };
        let phase = if extensions.contains("\"term\"") {
            "inverted index"
        } else if extensions.contains("\"fast\"") {
            "fast fields"
        } else if extensions.contains("\"store\"") {
            "document store"
        } else if extensions.contains("\"vec\"") {
            "vectors"
        } else if extensions.contains(STATS_EXT) {
            "statistics"
        } else if extensions.contains(CTID_MAP_EXT) {
            "tuple map"
        } else {
            "custom components"
        };
        report_phase(phase);
    }

    fn flush(&self) {}
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::index::mvcc::MvccSatisfies;
    use crate::index::writer::index::{Mergeable, SearchIndexMerger};
    use crate::postgres::PgSearchRelation;
    use log::debug;
    use pgrx::pg_sys::Oid;
    use pgrx::prelude::*;
    use std::ffi::CStr;
    use std::thread;

    const TARGET_LAYER_BYTES: u64 = 1024 * 1024;

    fn activity() -> String {
        unsafe {
            CStr::from_ptr(pg_sys::pgstat_get_backend_current_activity(
                pg_sys::MyProcPid,
                false,
            ))
            .to_string_lossy()
            .into_owned()
        }
    }

    #[pg_test]
    fn test_merge_progress_activity() {
        let progress = MergeProgress::start(2, 5, TARGET_LAYER_BYTES, 8);
        let prefix = "merging batch 2/5: 8 segments; target layer 1024 kB";
        assert_eq!(activity(), format!("{prefix}; phase=preparing"));
        debug!(target: MERGER_LOG_TARGET, "merge-plugin: {:?}", ["fast"]);
        assert_eq!(activity(), format!("{prefix}; phase=fast fields"));

        thread::spawn(|| {
            debug!(target: MERGER_LOG_TARGET, "merge-plugin: {:?}", ["vec", "centroids"]);
        })
        .join()
        .unwrap();
        assert_eq!(activity(), format!("{prefix}; phase=fast fields"));

        progress.report("complete");
        drop(progress);
        debug!(target: MERGER_LOG_TARGET, "merge-plugin: {:?}", ["store"]);
        assert_eq!(activity(), format!("{prefix}; phase=complete"));

        let _progress = MergeProgress::start(1, 1, MUTABLE_SEGMENT_LAYER, 1);
        assert_eq!(
            activity(),
            "merging batch 1/1: 1 segment; mutable conversion; phase=preparing"
        );
    }

    #[pg_test]
    fn test_tantivy_merge_reports_phase() {
        Spi::run(
            "CREATE TABLE phase_merge (id bigint, body text);
             CREATE INDEX phase_merge_idx ON phase_merge USING paradedb (id, body)
             WITH (mutable_segment_rows=0, layer_sizes='0', background_layer_sizes='0');
             INSERT INTO phase_merge VALUES (1, 'beer');
             INSERT INTO phase_merge VALUES (2, 'wine');",
        )
        .unwrap();
        let oid = Spi::get_one::<Oid>("SELECT 'phase_merge_idx'::regclass::oid")
            .unwrap()
            .unwrap();
        let index = PgSearchRelation::with_lock(oid, pg_sys::AccessShareLock as _);
        let mut merger = SearchIndexMerger::open(&index, MvccSatisfies::Mergeable).unwrap();
        let segments = merger
            .searchable_segment_ids()
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>();
        assert_eq!(segments.len(), 2);
        let _progress = MergeProgress::start(1, 1, TARGET_LAYER_BYTES, segments.len());
        let merged = merger.merge_segments(&segments).unwrap().unwrap();
        assert_eq!(merged.num_docs(), 2);
        assert!(activity().ends_with("; phase=tuple map"));
    }
}

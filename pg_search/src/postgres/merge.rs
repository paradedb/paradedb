// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::index::merge_policy::LayeredMergePolicy;
use crate::index::mvcc::MvccSatisfies;
use crate::index::writer::index::SearchIndexMerger;
use crate::postgres::ps_status::{set_ps_display_suffix, MERGING_IN_BACKGROUND};
use crate::postgres::storage::block::{SegmentMetaEntry, SEGMENT_METAS_START};
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::storage::LinkedItemList;
use crate::postgres::PgSearchRelation;

use pgrx::bgworkers::*;
use pgrx::pg_sys;
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::{function_name, pg_guard, FromDatum, IntoDatum, PgLogLevel, PgSqlErrorCode};
use std::ffi::CStr;

#[derive(Debug, Clone)]
struct IndexLayerSizes {
    foreground_layer_sizes: Vec<u64>,
    background_layer_sizes: Vec<u64>,
}

impl From<&PgSearchRelation> for IndexLayerSizes {
    fn from(index: &PgSearchRelation) -> Self {
        let index_options = index.options();
        let segment_components =
            LinkedItemList::<SegmentMetaEntry>::open(index, SEGMENT_METAS_START);
        let all_entries = unsafe { segment_components.list() };
        let index_byte_size = all_entries
            .iter()
            .map(|entry| entry.byte_size())
            .sum::<u64>();
        let target_segment_count = index_options.target_segment_count();
        let target_byte_size = index_byte_size / target_segment_count as u64;

        // clamp the highest layer size to be less than the following:
        //
        // `index_byte_size` / `target_segment_count`
        //
        // i.e. how big should each segment be if we want to have exactly `target_segment_count` segments?
        //
        // for instance, imagine:
        // - layer sizes: [1mb, 10mb, 100mb]
        // - index size: 200mb
        // - target segment count: 10
        //
        // then our recomputed layer sizes would be:
        // - [1mb, 10mb]
        //
        // why? the 100mb layer gets excluded because the target segment size is 20mb
        pgrx::debug1!("target_byte_size for merge: {target_byte_size}");

        let mut foreground_layer_sizes = index_options.foreground_layer_sizes();
        foreground_layer_sizes.retain(|&layer_size| layer_size < target_byte_size);
        pgrx::debug1!("adjusted foreground_layer_sizes: {foreground_layer_sizes:?}");

        let mut background_layer_sizes = index_options.background_layer_sizes();
        background_layer_sizes.retain(|&layer_size| layer_size < target_byte_size);
        pgrx::debug1!("adjusted background_layer_sizes {background_layer_sizes:?}");

        Self {
            foreground_layer_sizes,
            background_layer_sizes,
        }
    }
}
impl IndexLayerSizes {
    fn foreground(&self) -> Vec<u64> {
        self.foreground_layer_sizes.clone()
    }

    fn background(&self) -> Vec<u64> {
        self.background_layer_sizes.clone()
    }
}

/// Kick off a merge of the index, if needed.
///
/// First merge into the smaller layers in the foreground,
/// then launch a background worker to merge down the larger layers.
pub unsafe fn do_merge(index: &PgSearchRelation) -> anyhow::Result<()> {
    let merger = SearchIndexMerger::open(MvccSatisfies::Mergeable.directory(index))?;
    let layer_sizes = IndexLayerSizes::from(index);
    let foreground_layers = layer_sizes.foreground();
    let background_layers = layer_sizes.background();

    let metadata = MetaPage::open(index);
    let merge_lock = metadata.acquire_merge_lock();

    let needs_background_merge = !background_layers.is_empty() && {
        let mut background_merge_policy = LayeredMergePolicy::new(background_layers.clone());
        background_merge_policy.set_mergeable_segment_entries(&metadata, &merge_lock, &merger);
        let merge_candidates = background_merge_policy.simulate();
        !merge_candidates.is_empty()
    };

    // first merge down the foreground layers
    if !foreground_layers.is_empty() {
        let foreground_merge_policy = LayeredMergePolicy::new(foreground_layers);
        unsafe { foreground_merge_policy.merge_index(index, merge_lock, false) };
    } else {
        // we no longer need to hold the [`MergeLock`] as we're not merging in the foreground
        drop(merge_lock);
    }

    pgrx::debug1!("foreground merge complete, needs_background_merge: {needs_background_merge}");

    // then launch a background process to merge down the background layers
    // only if we determine that there are enough segments to merge in the background
    if needs_background_merge {
        try_launch_background_merger(index);
    }

    Ok(())
}

/// Try to launch a background process to merge down the index.
/// Is not guaranteed to launch the process if there are not enough `max_worker_processes` available.
unsafe fn try_launch_background_merger(index: &PgSearchRelation) {
    let dbname = CStr::from_ptr(pg_sys::get_database_name(pg_sys::MyDatabaseId))
        .to_string_lossy()
        .into_owned();

    let worker_name = format!(
        "background merger for {}.{}",
        index.namespace(),
        index.name()
    );
    if BackgroundWorkerBuilder::new(&worker_name)
        .enable_spi_access()
        .enable_shmem_access(None)
        .set_library("pg_search")
        .set_function("background_merge")
        .set_argument(index.oid().into_datum())
        .set_extra(&dbname)
        .set_notify_pid(unsafe { pg_sys::MyProcPid })
        .load_dynamic()
        .is_err()
    {
        ErrorReport::new(
            PgSqlErrorCode::ERRCODE_INSUFFICIENT_RESOURCES,
            "not enough available `max_worker_processes` to launch a background merger",
            function_name!(),
        )
        .set_hint("`SET max_worker_processes = <number>`")
        .report(PgLogLevel::WARNING);
    }
}

/// Actually do the merge
/// This function is called by the background worker.
#[pg_guard]
#[no_mangle]
extern "C-unwind" fn background_merge(arg: pg_sys::Datum) {
    BackgroundWorker::attach_signal_handlers(SignalWakeFlags::SIGHUP | SignalWakeFlags::SIGTERM);
    BackgroundWorker::connect_worker_to_spi(Some(BackgroundWorker::get_extra()), None);
    BackgroundWorker::transaction(|| {
        unsafe {
            set_ps_display_suffix(MERGING_IN_BACKGROUND.as_ptr());
            pg_sys::pgstat_report_activity(
                pg_sys::BackendState::STATE_RUNNING,
                MERGING_IN_BACKGROUND.as_ptr(),
            );
        };

        let index_oid = unsafe { u32::from_datum(arg, false) }.unwrap();
        let index = PgSearchRelation::open(index_oid.into());
        let layer_sizes = IndexLayerSizes::from(&index);
        let background_layers = layer_sizes.background();

        let merge_policy = LayeredMergePolicy::new(background_layers);
        let metadata = unsafe { MetaPage::open(&index) };
        let merge_lock = unsafe { metadata.acquire_merge_lock() };
        unsafe { merge_policy.merge_index(&index, merge_lock, true) };
    });
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::postgres::options::{
        DEFAULT_BACKGROUND_LAYER_SIZES, DEFAULT_FOREGROUND_LAYER_SIZES,
    };
    use pgrx::prelude::*;

    enum LayerSizes {
        Default,
        Foreground(String),
        Background(String),
    }

    impl LayerSizes {
        fn config_str(&self) -> String {
            match self {
                LayerSizes::Default => "".to_string(),
                LayerSizes::Foreground(sizes) => format!(", foreground_layer_sizes = '{sizes}'"),
                LayerSizes::Background(sizes) => format!(", background_layer_sizes = '{sizes}'"),
            }
        }
    }

    fn create_index_with_layer_sizes(layer_sizes: LayerSizes) -> pg_sys::Oid {
        Spi::run("SET client_min_messages = 'debug1';").unwrap();
        Spi::run("CREATE TABLE IF NOT EXISTS t (id SERIAL, data TEXT);").unwrap();
        Spi::run("INSERT INTO t (data) VALUES ('test');").unwrap();
        Spi::run(
            format!(
                "CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id'{})",
                layer_sizes.config_str()
            )
            .as_str(),
        )
        .unwrap();
        Spi::get_one::<pg_sys::Oid>(
            "SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';",
        )
        .expect("spi should succeed")
        .unwrap()
    }

    #[pg_test]
    fn test_configured_foreground_layer_sizes() {
        let index_oid = create_index_with_layer_sizes(LayerSizes::Foreground(
            "1kb, 10kb, 100kb, 1mb".to_string(),
        ));
        let index = PgSearchRelation::open(index_oid);
        let layer_sizes = index.options().foreground_layer_sizes();
        assert_eq!(layer_sizes, vec![1024, 10240, 102400, 1048576]);
    }

    #[pg_test]
    fn test_configured_background_layer_sizes() {
        let index_oid = create_index_with_layer_sizes(LayerSizes::Background(
            "1kb, 10kb, 100kb, 1mb".to_string(),
        ));
        let index = PgSearchRelation::open(index_oid);
        let layer_sizes = index.options().background_layer_sizes();
        assert_eq!(layer_sizes, vec![1024, 10240, 102400, 1048576]);
    }

    #[pg_test]
    fn test_zeroed_foreground_layer_sizes() {
        let index_oid = create_index_with_layer_sizes(LayerSizes::Foreground("0".to_string()));
        let index = PgSearchRelation::open(index_oid);
        let layer_sizes = index.options().foreground_layer_sizes();
        assert!(layer_sizes.is_empty());
    }

    #[pg_test]
    fn test_zeroed_background_layer_sizes() {
        let index_oid = create_index_with_layer_sizes(LayerSizes::Background("0".to_string()));
        let index = PgSearchRelation::open(index_oid);
        let layer_sizes = index.options().background_layer_sizes();
        assert!(layer_sizes.is_empty());
    }

    #[pg_test]
    fn test_default_foreground_layer_sizes() {
        let index_oid = create_index_with_layer_sizes(LayerSizes::Default);
        let index = PgSearchRelation::open(index_oid);
        let layer_sizes = index.options().foreground_layer_sizes();
        assert_eq!(layer_sizes, DEFAULT_FOREGROUND_LAYER_SIZES.to_vec());
    }

    #[pg_test]
    fn test_default_background_layer_sizes() {
        let index_oid = create_index_with_layer_sizes(LayerSizes::Default);
        let index = PgSearchRelation::open(index_oid);
        let layer_sizes = index.options().background_layer_sizes();
        assert_eq!(layer_sizes, DEFAULT_BACKGROUND_LAYER_SIZES.to_vec());
    }
}

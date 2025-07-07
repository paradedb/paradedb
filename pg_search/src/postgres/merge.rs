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
use crate::postgres::insert::merge_index_with_policy;
use crate::postgres::ps_status::{set_ps_display_suffix, MERGING_IN_BACKGROUND};
use crate::postgres::storage::block::{SegmentMetaEntry, SEGMENT_METAS_START};
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::storage::LinkedItemList;
use crate::postgres::PgSearchRelation;

use pgrx::bgworkers::*;
use pgrx::pg_sys;
use pgrx::{pg_guard, FromDatum, IntoDatum};
use std::ffi::CStr;

#[derive(Debug, Clone)]
struct LayerSizes {
    layer_sizes: Vec<u64>,
    background_layer_size_threshold: u64,
}

impl From<&PgSearchRelation> for LayerSizes {
    fn from(index: &PgSearchRelation) -> Self {
        let index_options = index.options();
        let segment_components =
            LinkedItemList::<SegmentMetaEntry>::open(index, SEGMENT_METAS_START);
        let all_entries = unsafe { segment_components.list() };
        let index_byte_size = all_entries
            .iter()
            .map(|entry| entry.byte_size())
            .sum::<u64>();
        let mut layer_sizes = index_options.layer_sizes();
        pgrx::debug1!("original layer_sizes: {layer_sizes:?}");

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
        let background_layer_size_threshold = index_options.background_layer_size_threshold();
        layer_sizes.retain(|&layer_size| {
            layer_size < target_byte_size || layer_size < background_layer_size_threshold
        });
        pgrx::debug1!("adjusted layer_sizes {layer_sizes:?} because of threshold {background_layer_size_threshold}");

        Self {
            layer_sizes,
            background_layer_size_threshold,
        }
    }
}
impl LayerSizes {
    fn foreground(&self) -> Vec<u64> {
        self.layer_sizes
            .iter()
            .filter(|&&size| size < self.background_layer_size_threshold)
            .cloned()
            .collect::<Vec<u64>>()
    }

    fn background(&self) -> Vec<u64> {
        self.layer_sizes
            .iter()
            .filter(|&&size| size >= self.background_layer_size_threshold)
            .cloned()
            .collect::<Vec<u64>>()
    }
}

/// Kick off a merge of the index, if needed.
///
/// First merge into the smaller layers in the foreground,
/// then launch a background worker to merge down the larger layers.
pub unsafe fn do_merge(
    index_oid: pg_sys::Oid,
    do_foreground_merge: bool,
    do_background_merge: bool,
) -> anyhow::Result<()> {
    let index = PgSearchRelation::open(index_oid);
    let merger = SearchIndexMerger::open(MvccSatisfies::Mergeable.directory(&index))?;
    let layer_sizes = LayerSizes::from(&index);

    // first merge down the foreground layers
    if do_foreground_merge {
        let foreground_layers = layer_sizes.foreground();
        let foreground_merge_policy = LayeredMergePolicy::new(foreground_layers);
        unsafe { merge_index_with_policy(&index, foreground_merge_policy, false) };
    }

    pgrx::debug1!("foreground merge complete, now evaluating background merge");

    // then launch a background process to merge down the background layers
    // only if we determine that there are enough segments to merge in the background
    if do_background_merge {
        let merge_candidates = {
            let metadata = MetaPage::open(&index);
            let merge_lock = metadata.acquire_merge_lock();
            let background_layers = layer_sizes.background();
            let mut background_merge_policy = LayeredMergePolicy::new(background_layers.clone());

            pgrx::debug1!("background layers: {background_layers:?}");

            background_merge_policy.simulate(&metadata, &merge_lock, &merger)
        };

        pgrx::debug1!("background merge_candidates: {merge_candidates:?}");

        if merge_candidates.is_empty() {
            return Ok(());
        }

        try_launch_background_merger(&index);
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
    let _ = BackgroundWorkerBuilder::new(&worker_name)
        .enable_spi_access()
        .enable_shmem_access(None)
        .set_library("pg_search")
        .set_function("background_merge")
        .set_argument(index.oid().into_datum())
        .set_extra(&dbname)
        .set_notify_pid(unsafe { pg_sys::MyProcPid })
        .load_dynamic();
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
        let layer_sizes = LayerSizes::from(&index);
        let background_layers = layer_sizes.background();

        let merge_policy = LayeredMergePolicy::new(background_layers);
        unsafe { merge_index_with_policy(&index, merge_policy, true) };
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    fn test_layer_sizes() {
        let layer_sizes = LayerSizes {
            layer_sizes: vec![100, 1000, 10000, 100000],
            background_layer_size_threshold: 10000,
        };

        assert_eq!(layer_sizes.foreground(), vec![100, 1000]);
        assert_eq!(layer_sizes.background(), vec![10000, 100000]);
    }
}

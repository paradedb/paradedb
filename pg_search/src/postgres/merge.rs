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
use crate::postgres::insert::merge_index_with_policy;
use crate::postgres::options::SearchIndexOptions;
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

impl From<SearchIndexOptions> for LayerSizes {
    fn from(index_options: SearchIndexOptions) -> Self {
        Self {
            layer_sizes: index_options.layer_sizes(),
            background_layer_size_threshold: index_options.background_layer_size_threshold(),
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

/// Kick of a merge of the index.
///
/// First merge into the smaller layers in the foreground,
/// then launch a background worker to merge down the larger layers.
pub unsafe fn do_merge(index_oid: pg_sys::Oid) {
    let index = PgSearchRelation::with_lock(index_oid, pg_sys::AccessShareLock as _);
    let heaprel = index
        .heap_relation()
        .expect("index should belong to a heap relation");

    /*
     * Recompute VACUUM XID boundaries.
     *
     * We don't actually care about the oldest non-removable XID.  Computing
     * the oldest such XID has a useful side-effect that we rely on: it
     * forcibly updates the XID horizon state for this backend.  This step is
     * essential; GlobalVisCheckRemovableFullXid() will not reliably recognize
     * that it is now safe to recycle newly deleted pages without this step.
     */
    unsafe { pg_sys::GetOldestNonRemovableTransactionId(heaprel.as_ptr()) };

    let index_options = unsafe { SearchIndexOptions::from_relation(&index) };
    let layer_sizes = LayerSizes::from(index_options);
    let foreground_layers = layer_sizes.foreground();

    // first merge down the foreground layers
    let foreground_merge_policy = LayeredMergePolicy::new(foreground_layers);
    unsafe { merge_index_with_policy(&index, foreground_merge_policy, false, false, false) };

    // then launch a background process to merge down the background layers
    try_launch_background_merger(index_oid);
}

/// Try to launch a background process to merge down the index.
/// Is not guaranteed to launch the process if there are not enough `max_worker_processes` available.
unsafe fn try_launch_background_merger(index_oid: pg_sys::Oid) {
    let dbname = CStr::from_ptr(pg_sys::get_database_name(pg_sys::MyDatabaseId))
        .to_string_lossy()
        .into_owned();

    let _ = BackgroundWorkerBuilder::new("background merger")
        .enable_spi_access()
        .enable_shmem_access(None)
        .set_library("pg_search")
        .set_function("background_merge")
        .set_argument(index_oid.into_datum())
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
        let index_oid = unsafe { u32::from_datum(arg, false) }.unwrap();
        let index = PgSearchRelation::with_lock(index_oid.into(), pg_sys::AccessShareLock as _);
        let index_options = unsafe { SearchIndexOptions::from_relation(&index) };
        let layer_sizes = LayerSizes::from(index_options);
        let background_layers = layer_sizes.background();

        let merge_policy = LayeredMergePolicy::new(background_layers);
        unsafe { merge_index_with_policy(&index, merge_policy, false, true, true) };
    });
}

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

use crate::gucs;
use crate::index::merge_policy::LayeredMergePolicy;
use crate::postgres::index::IndexKind;
use crate::postgres::insert::merge_index_with_policy;
use crate::postgres::options::SearchIndexOptions;
use crate::postgres::storage::block::{SegmentMetaEntry, SEGMENT_METAS_START};
use crate::postgres::storage::LinkedItemList;
use crate::postgres::PgSearchRelation;

use pgrx::bgworkers::*;
use pgrx::pg_sys;
use pgrx::{pg_guard, FromDatum, IntoDatum};
use std::ffi::CStr;

/// Try to launch a background process to merge down the index.
/// Is not guaranteed to launch the process if there are not enough `max_worker_processes` available.
pub unsafe fn try_launch_background_merger(index_oid: pg_sys::Oid) {
    let dbname = CStr::from_ptr(pg_sys::get_database_name(pg_sys::MyDatabaseId))
        .to_string_lossy()
        .into_owned();

    let _ = BackgroundWorkerBuilder::new("background merger")
        .enable_spi_access()
        .enable_shmem_access(None)
        .set_library("pg_search")
        .set_function("do_merge")
        .set_argument(index_oid.into_datum())
        .set_extra(&dbname)
        .set_notify_pid(unsafe { pg_sys::MyProcPid })
        .load_dynamic();
}

/// Actually do the merge
/// This function is called by the background worker.
#[pg_guard]
#[no_mangle]
pub extern "C-unwind" fn do_merge(arg: pg_sys::Datum) {
    BackgroundWorker::attach_signal_handlers(SignalWakeFlags::SIGHUP | SignalWakeFlags::SIGTERM);
    BackgroundWorker::connect_worker_to_spi(Some(BackgroundWorker::get_extra()), None);
    BackgroundWorker::transaction(|| {
        let index_oid = unsafe { u32::from_datum(arg, false) }.unwrap();
        let index = PgSearchRelation::with_lock(index_oid.into(), pg_sys::AccessShareLock as _);
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

        // compute how big the largest segments should be to achieve the target segment count
        // assuming the index consisted only of these largest segments
        // let target_byte_size = {
        //     let index_kind = IndexKind::for_index(index.clone()).unwrap();
        //     let index_byte_size = index_kind
        //         .partitions()
        //         .map(|index| {
        //             let segment_components =
        //                 LinkedItemList::<SegmentMetaEntry>::open(&index, SEGMENT_METAS_START);
        //             let all_entries = unsafe { segment_components.list() };
        //             all_entries
        //                 .iter()
        //                 .map(|entry| entry.byte_size())
        //                 .sum::<u64>()
        //         })
        //         .sum::<u64>();
        //     let target_segment_count = gucs::target_segment_count();
        //     index_byte_size / target_segment_count as u64
        // };

        let index_options = unsafe { SearchIndexOptions::from_relation(&index) };
        let mut layer_sizes = index_options.layer_sizes();

        // if the uppermost layer is not big enough to create the target number of segments,
        // add the `target_byte_size` as the uppermost layer
        //
        // we arbitrarily say that the `target_byte_size` must be at least 3x the size of the
        // uppermost layer to avoid having two layers be too close together
        // if target_byte_size > layer_sizes.iter().max().unwrap() * 3 {
        //     // add the target byte size as the uppermost layer
        //     layer_sizes.push(target_byte_size);
        // }

        let merge_policy = LayeredMergePolicy::new(layer_sizes);
        unsafe { merge_index_with_policy(&index, merge_policy, true, true, true) };
    });
}

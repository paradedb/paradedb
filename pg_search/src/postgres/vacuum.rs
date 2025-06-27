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
use crate::postgres::insert::merge_index_with_policy;
use crate::postgres::index::IndexKind;
use crate::postgres::options::SearchIndexOptions;
use crate::postgres::storage::block::{SegmentMetaEntry, SEGMENT_METAS_START};
use crate::postgres::storage::LinkedItemList;
use crate::postgres::PgSearchRelation;

use pgrx::bgworkers::*;
use pgrx::pg_sys;
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::{
    direct_function_call, function_name, pg_guard, FromDatum, IntoDatum, PgLogLevel, PgSqlErrorCode,
};
use std::ffi::CStr;

#[pg_guard]
pub unsafe extern "C-unwind" fn amvacuumcleanup(
    info: *mut pg_sys::IndexVacuumInfo,
    stats: *mut pg_sys::IndexBulkDeleteResult,
) -> *mut pg_sys::IndexBulkDeleteResult {
    let index_oid = (*(*info).index).rd_id;
    let dbname = CStr::from_ptr(pg_sys::get_database_name(pg_sys::MyDatabaseId))
        .to_string_lossy()
        .into_owned();

    if let Ok(worker) = BackgroundWorkerBuilder::new("background merger")
        .enable_spi_access()
        .enable_shmem_access(None)
        .set_library("pg_search")
        .set_function("merge_in_background")
        .set_argument(index_oid.into_datum())
        .set_extra(&dbname)
        .set_notify_pid(unsafe { pg_sys::MyProcPid })
        .load_dynamic()
    {
        let pid = worker
            .wait_for_startup()
            .expect("background merger should have started");
        assert!(pid > 0, "background merger should have a valid PID");
    } else {
        ErrorReport::new(
            PgSqlErrorCode::ERRCODE_INSUFFICIENT_RESOURCES,
            "background merger for the BM25 index was not launched, likely because there were not enough `max_worker_processes` available",
            function_name!(),
        )
        .set_hint("`SET max_worker_processes = <number>`")
        .report(PgLogLevel::ERROR);
    }

    stats
}

#[pg_guard]
#[no_mangle]
pub extern "C-unwind" fn merge_in_background(arg: pg_sys::Datum) {
    BackgroundWorker::attach_signal_handlers(SignalWakeFlags::SIGHUP | SignalWakeFlags::SIGTERM);
    BackgroundWorker::connect_worker_to_spi(Some(BackgroundWorker::get_extra()), None);
    BackgroundWorker::transaction(|| {
        let index_oid = unsafe { u32::from_datum(arg, false) }.unwrap();
        let index = PgSearchRelation::with_lock(index_oid.into(), pg_sys::AccessShareLock as _);
        let index_options = unsafe { SearchIndexOptions::from_relation(&index) };
        let layer_sizes = index_options.layer_sizes();
        let merge_policy = LayeredMergePolicy::new(layer_sizes);
        unsafe { merge_index_with_policy(&index, merge_policy, true, true, true) };
    });
}

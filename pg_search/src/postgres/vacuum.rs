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

use pgrx::bgworkers::BackgroundWorkerBuilder;
use pgrx::*;

#[pg_guard]
pub unsafe extern "C-unwind" fn amvacuumcleanup(
    info: *mut pg_sys::IndexVacuumInfo,
    stats: *mut pg_sys::IndexBulkDeleteResult,
) -> *mut pg_sys::IndexBulkDeleteResult {
    let worker = BackgroundWorkerBuilder::new("background merger")
        .enable_shmem_access(None)
        .set_library("pg_search")
        .set_function("merge_in_background")
        .set_argument((*(*info).index).rd_id.into_datum())
        .set_notify_pid(unsafe { pg_sys::MyProcPid })
        .load_dynamic()
        .expect("background merger should have loaded");
    let pid = worker
        .wait_for_startup()
        .expect("background merger should have started");
    assert!(pid > 0, "background mergershould have a valid PID");

    stats
}

#[pg_guard]
#[no_mangle]
pub extern "C-unwind" fn merge_in_background(arg: pg_sys::Datum) {
    use pgrx::bgworkers::*;
    BackgroundWorker::attach_signal_handlers(SignalWakeFlags::SIGHUP | SignalWakeFlags::SIGTERM);
    let index_oid = unsafe { u32::from_datum(arg, false) }.unwrap();
}

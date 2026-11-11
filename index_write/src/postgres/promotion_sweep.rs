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

//! The promotion sweep: a worker that starts once the server is read-write and, if the startup
//! process left the standby marker (`postgres::promotion`), marks every ParadeDB index in every
//! database as replicated and removes the marker.

use crate::postgres::catalog::{OidExt, paradedb_index_oids};
use crate::postgres::mutable_sweep::{connect, connectable_databases};
use crate::postgres::promotion::{marker_path, marker_present, remove_marker};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::metadata::MetaPage;
use pgrx::bgworkers::{
    BackgroundWorker, BackgroundWorkerBuilder, BackgroundWorkerStatus, BgWorkerStartTime,
    DynamicBackgroundWorker,
};
use pgrx::{pg_guard, pg_sys};
use std::time::Duration;

/// The launcher's end of a one-byte dynamic shared memory segment that a database worker sets
/// once every index is marked. Postgres reports a dynamic worker as stopped whether it returned
/// or failed, so the launcher cannot tell from the worker handle alone.
struct SweepResult(*mut pg_sys::dsm_segment);

impl SweepResult {
    /// Called outside any transaction: a segment owned by a resource owner is detached at commit.
    fn create() -> Self {
        let segment = unsafe { pg_sys::dsm_create(1, 0) };
        unsafe {
            pg_sys::dsm_segment_address(segment)
                .cast::<u8>()
                .write_volatile(0)
        };
        Self(segment)
    }

    fn task(&self, database: pg_sys::Oid) -> SweepTask {
        SweepTask {
            database,
            result: unsafe { pg_sys::dsm_segment_handle(self.0) },
        }
    }

    /// Waits for the worker to exit, then reads its report.
    fn outcome(self, worker: DynamicBackgroundWorker) -> Result<bool, BackgroundWorkerStatus> {
        worker.wait_for_shutdown()?;
        Ok(unsafe {
            pg_sys::dsm_segment_address(self.0)
                .cast::<u8>()
                .read_volatile()
                != 0
        })
    }
}

crate::impl_safe_drop!(SweepResult, |self| {
    unsafe { pg_sys::dsm_detach(self.0) };
});

/// What a database worker receives: the database to sweep and where to report.
struct SweepTask {
    database: pg_sys::Oid,
    result: pg_sys::dsm_handle,
}

impl SweepTask {
    fn into_datum(self) -> pg_sys::Datum {
        const { assert!(std::mem::size_of::<pg_sys::Datum>() >= 8) };
        pg_sys::Datum::from(u64::from(self.database.to_u32()) | (u64::from(self.result) << 32))
    }

    fn from_datum(argument: pg_sys::Datum) -> Self {
        let value = argument.value() as u64;
        Self {
            database: pg_sys::Oid::from(value as u32),
            result: (value >> 32) as pg_sys::dsm_handle,
        }
    }

    /// Call only once the marking transaction has committed: the launcher takes the byte as proof.
    fn report_success(self) {
        let segment = unsafe { pg_sys::dsm_attach(self.result) };
        if segment.is_null() {
            return;
        }
        unsafe {
            pg_sys::dsm_segment_address(segment)
                .cast::<u8>()
                .write_volatile(1)
        };
        unsafe { pg_sys::dsm_detach(segment) };
    }
}

pub fn init() {
    BackgroundWorkerBuilder::new("pg_search promotion sweep launcher")
        .enable_spi_access()
        .set_library("pg_search")
        .set_function("promotion_sweep_launcher")
        .set_start_time(BgWorkerStartTime::RecoveryFinished)
        .set_restart_time(Some(Duration::from_secs(5)))
        .load();
}

// Starts once the server is read-write: boot on a primary, promotion on a hot standby. A normal
// return ends the worker for this postmaster lifetime; an ERROR exits 1 and it restarts.
#[pg_guard]
#[unsafe(no_mangle)]
unsafe extern "C-unwind" fn promotion_sweep_launcher(_arg: pg_sys::Datum) {
    if !marker_present() {
        return;
    }
    unsafe { connect(None) };
    let databases = BackgroundWorker::transaction(|| unsafe { connectable_databases() });
    for database in databases {
        let result = SweepResult::create();
        let worker = BackgroundWorkerBuilder::new("pg_search promotion sweep")
            .enable_spi_access()
            .set_library("pg_search")
            .set_function("promotion_sweep_database")
            .set_argument(Some(result.task(database).into_datum()))
            .set_notify_pid(unsafe { pg_sys::MyProcPid })
            .load_dynamic()
            .unwrap_or_else(|_| {
                pgrx::error!(
                    "pg_search could not start a promotion sweep worker; max_worker_processes may be exhausted"
                )
            });
        // After the wait pgrx reports only Stopped or PostmasterDied; anything else is a bug and
        // must not end the sweep.
        let succeeded = match result.outcome(worker) {
            Ok(succeeded) => succeeded,
            Err(BackgroundWorkerStatus::PostmasterDied) => return,
            Err(status) => pgrx::error!(
                "pg_search lost track of a promotion sweep worker ({status:?}); retrying"
            ),
        };
        if !succeeded {
            pgrx::error!(
                "pg_search could not mark the replicated indexes in database {} after promotion; retrying",
                database.to_u32()
            );
        }
    }
    if let Err(e) = remove_marker() {
        pgrx::error!(
            "pg_search could not remove {}: {e}",
            marker_path().display()
        );
    }
    pgrx::log!(
        "pg_search marked the indexes replicated from the former primary as unusable; REINDEX rebuilds them on this server"
    );
}

#[pg_guard]
#[unsafe(no_mangle)]
unsafe extern "C-unwind" fn promotion_sweep_database(argument: pg_sys::Datum) {
    let task = SweepTask::from_datum(argument);
    let database = task.database;
    unsafe { connect(Some(database)) };
    let marked = BackgroundWorker::transaction(|| {
        let mut marked = 0usize;
        for oid in paradedb_index_oids() {
            let Some(index) = PgSearchRelation::try_open(oid, pg_sys::RowExclusiveLock as _) else {
                continue;
            };
            if !unsafe { (*index.rd_rel).relam }.is_paradedb_am() {
                continue;
            }
            MetaPage::mark_standby_tainted(&index);
            marked += 1;
        }
        marked
    });
    task.report_success();
    pgrx::log!(
        "pg_search marked {marked} replicated index(es) in database {} as unusable after promotion",
        database.to_u32()
    );
}

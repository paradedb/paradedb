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

//! Refuses ParadeDB indexes that reached this server through physical replication once the
//! server has been promoted. A standby's pages must stay byte copies of the primary's, so the
//! startup process records standby mode in a file under the data directory, and a worker that
//! starts once the server is read-write marks every ParadeDB index and removes the file.

use crate::postgres::catalog::OidExt;
use crate::postgres::index::{is_partitioned_index, leaf_partition_indexes};
use crate::postgres::mutable_sweep::{connect, connectable_databases, paradedb_index_oids};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::metadata::MetaPage;
use pgrx::bgworkers::{
    BackgroundWorker, BackgroundWorkerBuilder, BackgroundWorkerStatus, BgWorkerStartTime,
    DynamicBackgroundWorker,
};
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::{PgLogLevel, PgSqlErrorCode, function_name, pg_guard, pg_sys};
use std::cell::Cell;
use std::ffi::{CStr, OsStr};
use std::fs;
use std::io::{ErrorKind, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Written by the startup process while this cluster replays as a standby; its presence means a
/// promotion sweep is owed. The content is not read.
const MARKER_FILE: &str = "pg_search_standby";

fn marker_path() -> PathBuf {
    let data_dir = unsafe { CStr::from_ptr(pg_sys::DataDir) };
    Path::new(OsStr::from_bytes(data_dir.to_bytes())).join(MARKER_FILE)
}

/// A marker that cannot be inspected counts as present, so a damaged file never unblocks a
/// replicated index.
fn marker_present() -> bool {
    match fs::metadata(marker_path()) {
        Ok(_) => true,
        Err(e) => e.kind() != ErrorKind::NotFound,
    }
}

fn write_marker() -> std::io::Result<()> {
    let path = marker_path();
    let tmp = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp)?;
    file.write_all(b"standby\n")?;
    file.sync_all()?;
    fs::rename(&tmp, &path)?;
    fs::File::open(path.parent().expect("marker lives in the data directory"))?.sync_all()
}

fn remove_marker() -> std::io::Result<bool> {
    match fs::remove_file(marker_path()) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

/// The rmgr startup callback, in the startup process before redo. Point-in-time and crash
/// recovery of this server's own data leave `StandbyMode` false and are not restores, so they
/// neither write nor remove the marker.
pub(crate) fn recovery_starting() {
    if unsafe { pg_sys::StandbyMode } {
        record_standby();
    } else if restoring_base_backup() {
        forget_standby();
    }
}

/// The rmgr cleanup callback, after the redo loop. On a promoting standby `StandbyMode` is still
/// set here, so the marker is fresh at promotion even if it was removed while the standby ran.
pub(crate) fn recovery_ending() {
    if unsafe { pg_sys::StandbyMode } {
        record_standby();
    }
}

fn record_standby() {
    if let Err(e) = write_marker() {
        pgrx::warning!(
            "pg_search could not record that this server is a standby in {}: {e}",
            marker_path().display()
        );
    }
}

/// A base backup taken from a standby carries the marker, and restoring it is not a promotion:
/// the same bytes taken from the primary carry no marker.
fn forget_standby() {
    match remove_marker() {
        Ok(true) => pgrx::log!(
            "pg_search removed the standby marker carried by the base backup; this restore is not a promotion"
        ),
        Ok(false) => {}
        Err(e) => pgrx::warning!(
            "pg_search could not remove {}: {e}",
            marker_path().display()
        ),
    }
}

/// True while recovery starts from a base backup: `InitWalRecovery` copies the backup start
/// point from `backup_label` into `pg_control` before the rmgr startup callbacks run, and it is
/// cleared once the backup end is reached.
fn restoring_base_backup() -> bool {
    let mut crc_ok = false;
    let control = unsafe { pg_sys::get_controlfile(pg_sys::DataDir, &mut crc_ok) };
    if control.is_null() {
        return false;
    }
    let restoring = crc_ok && unsafe { (*control).backupStartPoint } != 0;
    unsafe { pg_sys::pfree(control.cast()) };
    restoring
}

thread_local! {
    // Timeline on which this backend last found no marker. An in-place promotion starts a new
    // timeline, so a backend that lived through one looks again.
    static CLEAR_ON_TIMELINE: Cell<Option<pg_sys::TimeLineID>> = const { Cell::new(None) };
}

fn sweep_pending() -> bool {
    let timeline = unsafe { pg_sys::GetWALInsertionTimeLine() };
    if CLEAR_ON_TIMELINE.get() == Some(timeline) {
        return false;
    }
    let pending = marker_present();
    if !pending {
        CLEAR_ON_TIMELINE.set(Some(timeline));
    }
    pending
}

/// True when `index` must not serve queries: it arrived through physical replication and this
/// server has since been promoted, or the promotion sweep has not finished yet. Always false
/// while the server is in recovery, where the standby read gate applies instead.
pub fn index_blocked_after_promotion(index: &PgSearchRelation) -> bool {
    if unsafe { pg_sys::RecoveryInProgress() } {
        return false;
    }
    if sweep_pending() {
        return true;
    }
    // A partitioned index has no metapage; the sweep marks its leaves.
    if is_partitioned_index(index.oid()) {
        return leaf_partition_indexes(index).any(|leaf| MetaPage::open(&leaf).standby_tainted());
    }
    MetaPage::open(index).standby_tainted()
}

/// For the "table has no ParadeDB index" fallbacks: report a blocked index as the reason instead.
pub fn report_if_blocked(heaprelid: pg_sys::Oid) {
    if heaprelid == pg_sys::Oid::INVALID || unsafe { pg_sys::RecoveryInProgress() } {
        return;
    }
    let heaprel = PgSearchRelation::with_lock(heaprelid, pg_sys::AccessShareLock as _);
    if let Some(index) = heaprel
        .valid_paradedb_indices()
        .find(index_blocked_after_promotion)
    {
        report_blocked(&index);
    }
}

/// The error a blocked index raises, with the `REINDEX` hint.
pub fn report_blocked(index: &PgSearchRelation) -> ! {
    ErrorReport::new(
        PgSqlErrorCode::ERRCODE_FEATURE_NOT_SUPPORTED,
        format!(
            "index \"{}\" was replicated from another server and this server has since been promoted; failover is supported on ParadeDB Enterprise, not ParadeDB Community",
            index.name()
        ),
        function_name!(),
    )
    .set_detail("ParadeDB Community does not serve indexes that arrived through physical replication. ParadeDB Enterprise is commercially licensed and included with ParadeDB Cloud. To self-host ParadeDB Enterprise, contact sales@paradedb.com.")
    .set_hint(format!(
        "Run `REINDEX INDEX {}` to rebuild the index on this server.",
        pgrx::spi::quote_qualified_identifier(index.namespace(), index.name())
    ))
    .report(PgLogLevel::ERROR);
    unreachable!("ErrorReport::report at ERROR level does not return")
}

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

impl Drop for SweepResult {
    fn drop(&mut self) {
        unsafe { pg_sys::dsm_detach(self.0) };
    }
}

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

pub(crate) fn init() {
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

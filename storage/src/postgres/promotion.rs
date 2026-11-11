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

//! Refuses ParadeDB indexes that reached this server through physical replication. The primary
//! names an index in the rmgr record it emits when the index is built and at the end of every
//! VACUUM; a standby replaying the record sets a taint in the index's metapage. VACUUM is the
//! only normal writer of that page, and its record follows the write, so the taint outlives the
//! full-page images that replication delivers. `REINDEX` starts from a fresh metapage.
//!
//! A standby promoted by a restart without `standby.signal` replays the primary's WAL once more
//! as crash recovery, which repeats the full-page images; the marker file written while the
//! server was a standby tells that recovery to taint as well.

use crate::postgres::index::{is_partitioned_index, leaf_partition_indexes};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::metadata::MetaPage;
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::{PgLogLevel, PgSqlErrorCode, function_name, pg_sys};
use std::ffi::{CStr, OsStr};
use std::fs;
use std::io::{ErrorKind, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

const MARKER_FILE: &str = "pg_search_standby";

pub fn marker_path() -> PathBuf {
    let data_dir = unsafe { CStr::from_ptr(pg_sys::DataDir) };
    Path::new(OsStr::from_bytes(data_dir.to_bytes())).join(MARKER_FILE)
}

/// A marker that cannot be inspected counts as present, so a damaged file never skips the taint.
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

/// Set in the startup process when a crash recovery replays WAL this server received as a
/// standby.
static TAINT_DURING_CRASH_RECOVERY: AtomicBool = AtomicBool::new(false);

/// True when replaying an `INIT_INDEX` record must taint the index: the server is a standby, or
/// it was one and is being promoted by a restart.
pub(crate) fn taints_during_redo() -> bool {
    (unsafe { pg_sys::StandbyMode }) || TAINT_DURING_CRASH_RECOVERY.load(Ordering::Relaxed)
}

/// The rmgr startup callback, in the startup process before redo. Point-in-time and crash
/// recovery of a primary's own data find no marker and taint nothing.
pub(crate) fn recovery_starting() {
    if unsafe { pg_sys::StandbyMode } {
        if let Err(e) = write_marker() {
            pgrx::warning!(
                "pg_search could not record that this server is a standby in {}: {e}",
                marker_path().display()
            );
        }
    } else if marker_present() {
        if restoring_base_backup() {
            forget_standby();
        } else {
            TAINT_DURING_CRASH_RECOVERY.store(true, Ordering::Relaxed);
            pgrx::log!(
                "this server was a ParadeDB Community standby and is starting as a primary; \
                 ParadeDB indexes named in the replayed WAL refuse queries until REINDEX"
            );
        }
    }
}

/// The rmgr cleanup callback, after the redo loop: the server is leaving recovery as a primary.
pub(crate) fn recovery_ending() {
    if taints_during_redo()
        && let Err(e) = remove_marker()
    {
        pgrx::warning!(
            "pg_search could not remove {}: {e}",
            marker_path().display()
        );
    }
}

/// A base backup taken from a standby carries the marker, and restoring it is not a promotion:
/// the same bytes taken from the primary carry no marker. Taints already in the index files stay
/// until `REINDEX`.
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

/// True when `index` must not serve queries: its metapage carries the standby taint. Always
/// false while the server is in recovery, where the standby read gate applies instead.
pub fn index_blocked_after_promotion(index: &PgSearchRelation) -> bool {
    if unsafe { pg_sys::RecoveryInProgress() } {
        return false;
    }
    // A partitioned index has no storage of its own; the records name its leaves.
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

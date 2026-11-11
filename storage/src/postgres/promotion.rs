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
//! server has been promoted. The primary names an index in the rmgr record it emits when the
//! index is built and whenever it is vacuumed. A standby's pages must stay byte copies of the
//! primary's, so the startup process lists each named index in a file under the data directory
//! instead of marking the index; the gate reads that list once the server is read-write.
//! `REINDEX` gives the index a new file number, which the list does not name.

use crate::api::HashSet;
use crate::postgres::index::{is_partitioned_index, leaf_partition_indexes};
use crate::postgres::rel::PgSearchRelation;
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::{PgLogLevel, PgSqlErrorCode, function_name, pg_sys};
use std::ffi::{CStr, OsStr};
use std::fs;
use std::io::{self, ErrorKind, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const REGISTRY_FILE: &str = "pg_search_standby";

/// An index as the `INIT_INDEX` record names it, and as the registry file lists it.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReplicatedIndex {
    pub database: u32,
    pub index: u32,
    pub relfilenumber: u32,
}

impl ReplicatedIndex {
    pub fn of(index: &PgSearchRelation) -> Self {
        Self {
            database: index.database_oid().to_u32(),
            index: index.oid().to_u32(),
            relfilenumber: index.relfilenumber(),
        }
    }

    fn line(&self) -> String {
        format!("{} {} {}\n", self.database, self.index, self.relfilenumber)
    }

    fn parse(line: &str) -> Option<Self> {
        let mut fields = line.split_whitespace().map(str::parse::<u32>);
        let entry = Self {
            database: fields.next()?.ok()?,
            index: fields.next()?.ok()?,
            relfilenumber: fields.next()?.ok()?,
        };
        fields.next().is_none().then_some(entry)
    }
}

pub fn registry_path() -> PathBuf {
    let data_dir = unsafe { CStr::from_ptr(pg_sys::DataDir) };
    Path::new(OsStr::from_bytes(data_dir.to_bytes())).join(REGISTRY_FILE)
}

/// A missing file is an empty registry; an unreadable or malformed one is an error.
fn read_registry() -> io::Result<HashSet<ReplicatedIndex>> {
    let text = match fs::read_to_string(registry_path()) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(HashSet::default()),
        Err(e) => return Err(e),
    };
    text.lines()
        .map(|line| {
            ReplicatedIndex::parse(line).ok_or_else(|| {
                io::Error::new(ErrorKind::InvalidData, format!("unexpected line {line:?}"))
            })
        })
        .collect()
}

fn write_registry(entries: &HashSet<ReplicatedIndex>) -> io::Result<()> {
    let path = registry_path();
    let tmp = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp)?;
    let mut lines: Vec<String> = entries.iter().map(ReplicatedIndex::line).collect();
    lines.sort();
    file.write_all(lines.concat().as_bytes())?;
    file.sync_all()?;
    fs::rename(&tmp, &path)?;
    fs::File::open(path.parent().expect("registry lives in the data directory"))?.sync_all()
}

/// The redo of an `INIT_INDEX` record in the startup process of a standby.
pub(crate) fn record_replicated_index(index: ReplicatedIndex) {
    let result = read_registry().and_then(|mut entries| {
        if !entries.insert(index) {
            return Ok(false);
        }
        write_registry(&entries).map(|()| true)
    });
    match result {
        Ok(true) => pgrx::log!(
            "ParadeDB index {} in database {} was built or vacuumed on the primary; ParadeDB \
             Community does not serve ParadeDB indexes on a standby, and after promotion this one \
             refuses queries until REINDEX",
            index.index,
            index.database
        ),
        Ok(false) => {}
        Err(e) => pgrx::warning!(
            "pg_search could not list ParadeDB index {} of database {} in {}: {e}; after \
             promotion it will not be refused",
            index.index,
            index.database,
            registry_path().display()
        ),
    }
}

/// The rmgr startup callback, in the startup process before redo. A base backup taken from a
/// standby carries the registry, and restoring it is not a promotion: the same bytes taken from
/// the primary carry no registry. Point-in-time and crash recovery of this server's own data are
/// not restores and keep it.
pub(crate) fn recovery_starting() {
    if unsafe { pg_sys::StandbyMode } || !restoring_base_backup() {
        return;
    }
    match fs::remove_file(registry_path()) {
        Ok(()) => pgrx::log!(
            "pg_search removed the list of replicated ParadeDB indexes carried by the base backup; this restore is not a promotion"
        ),
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => pgrx::warning!(
            "pg_search could not remove {}: {e}",
            registry_path().display()
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

/// Loaded once per process, after recovery has ended and the startup process no longer writes
/// the file. A registry that cannot be read blocks every ParadeDB index with an error naming it.
fn registered(index: ReplicatedIndex) -> bool {
    static REGISTRY: OnceLock<HashSet<ReplicatedIndex>> = OnceLock::new();
    REGISTRY
        .get_or_init(|| {
            read_registry().unwrap_or_else(|e| {
                pgrx::error!(
                    "pg_search could not read the list of replicated ParadeDB indexes in {}: {e}",
                    registry_path().display()
                )
            })
        })
        .contains(&index)
}

/// True when `index` must not serve queries: it arrived through physical replication and this
/// server has since been promoted. Always false while the server is in recovery, where the
/// standby read gate applies instead.
pub fn index_blocked_after_promotion(index: &PgSearchRelation) -> bool {
    if unsafe { pg_sys::RecoveryInProgress() } {
        return false;
    }
    // A partitioned index has no storage of its own; the records name its leaves.
    if is_partitioned_index(index.oid()) {
        return leaf_partition_indexes(index).any(|leaf| registered(ReplicatedIndex::of(&leaf)));
    }
    registered(ReplicatedIndex::of(index))
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

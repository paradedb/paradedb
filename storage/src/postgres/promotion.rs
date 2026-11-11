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
//! only routine writer of that page, and its record follows the write, so the taint outlives the
//! full-page images that replication delivers; the one-time metapage upgrade of an index built by
//! an old release clears it until the next VACUUM. `REINDEX` starts from a fresh metapage.

use crate::postgres::index::{is_partitioned_index, leaf_partition_indexes};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::metadata::MetaPage;
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::{PgLogLevel, PgSqlErrorCode, function_name, pg_sys};
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

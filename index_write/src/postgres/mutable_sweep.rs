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
#![allow(unsafe_op_in_unsafe_fn)]

use crate::api::{HashMap, HashSet};
use crate::index::merge_policy::LayeredMergePolicy;
use crate::index::mvcc::MvccSatisfies;
use crate::index::writer::index::SearchIndexMerger;
use crate::postgres::catalog::{OidExt, lookup_extension_version, paradedb_index_oids};
use crate::postgres::delete::VacuumSignal;
use crate::postgres::index::free_leaf_partition_indexes;
use crate::postgres::locks::AdvisoryLock;
use crate::postgres::merge::{
    MergeMode, MergeSlot, MergeSlotVariant, merge_crash_details, merge_index,
    warn_unsupported_vector_merge,
};
use crate::postgres::ps_status::{MERGING, set_ps_display_suffix};
use crate::postgres::storage::metadata::MetaPage;
use pgrx::bgworkers::{BackgroundWorker, BackgroundWorkerBuilder, SignalWakeFlags};
use pgrx::{
    GucContext, GucFlags, GucRegistry, GucSetting, IntoDatum, PgTryBuilder, Spi,
    check_for_interrupts, pg_guard, pg_sys,
};
use std::ffi::CStr;
use std::panic::AssertUnwindSafe;
use std::time::{Duration, Instant};

static INTERVAL_MS: GucSetting<i32> = GucSetting::<i32>::new(1000);

// Check for missing workers and new databases independently of the sweep interval.
const WORKER_CHECK_INTERVAL: Duration = Duration::from_secs(30);

// Yield to database maintenance on the same cadence as PostgreSQL's DROP DATABASE wait.
const DATABASE_CHECK_INTERVAL: Duration = Duration::from_millis(100);

pub fn init() {
    GucRegistry::define_int_guc(
        c"paradedb.mutable_sweep_interval",
        c"Time between mutable-segment sweeps, in milliseconds.",
        c"Zero disables periodic sweeps. Write-triggered merging is unaffected.",
        &INTERVAL_MS,
        0,
        i32::MAX,
        GucContext::Sighup,
        GucFlags::UNIT_MS,
    );
    BackgroundWorkerBuilder::new("pg_search mutable sweep launcher")
        .enable_spi_access()
        .set_library("pg_search")
        .set_function("mutable_sweep_launcher")
        .set_restart_time(Some(Duration::from_secs(1)))
        .load();
}

unsafe fn connect(database: Option<pg_sys::Oid>) {
    unsafe extern "C-unwind" {
        #[link_name = "die"]
        fn worker_sigterm(signal: i32);
    }
    #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
    pg_sys::pqsignal(pg_sys::SIGTERM as i32, Some(worker_sigterm));
    #[cfg(not(any(feature = "pg15", feature = "pg16", feature = "pg17")))]
    pg_sys::pqsignal_be(pg_sys::SIGTERM as i32, Some(worker_sigterm));
    BackgroundWorker::attach_signal_handlers(SignalWakeFlags::SIGHUP);
    BackgroundWorker::connect_worker_to_spi_by_oid(database, None);
    if pg_sys::synchronous_commit > pg_sys::SyncCommitLevel::SYNCHRONOUS_COMMIT_LOCAL_FLUSH as i32 {
        pg_sys::SetConfigOption(
            c"synchronous_commit".as_ptr(),
            c"local".as_ptr(),
            pg_sys::GucContext::PGC_SUSET,
            pg_sys::GucSource::PGC_S_OVERRIDE,
        );
    }
}

unsafe fn reload() {
    check_for_interrupts();
    if BackgroundWorker::sighup_received() {
        pg_sys::ProcessConfigFile(pg_sys::GucContext::PGC_SIGHUP);
    }
}

#[pg_guard]
#[unsafe(no_mangle)]
unsafe extern "C-unwind" fn mutable_sweep_launcher(_arg: pg_sys::Datum) {
    connect(None);
    let mut worker_context = pgrx::PgMemoryContexts::new("pg_search sweep worker handles");
    loop {
        reload();
        if INTERVAL_MS.get() != 0 {
            let (databases, active) = BackgroundWorker::transaction(|| {
                let mut active = HashSet::default();
                for backend in 1..=pg_sys::pgstat_fetch_stat_numbackends() {
                    #[cfg(feature = "pg15")]
                    let status = pg_sys::pgstat_fetch_stat_beentry(backend);
                    #[cfg(not(feature = "pg15"))]
                    let status = {
                        let entry = pg_sys::pgstat_get_local_beentry_by_index(backend);
                        if entry.is_null() {
                            continue;
                        }
                        &raw mut (*entry).backendStatus
                    };
                    if !status.is_null()
                        && (*status).st_backendType == pg_sys::BackendType::B_BG_WORKER
                        && CStr::from_ptr((*status).st_appname) == c"pg_search mutable sweep"
                    {
                        active.insert((*status).st_databaseid);
                    }
                }
                let relation =
                    pg_sys::table_open(pg_sys::DatabaseRelationId, pg_sys::AccessShareLock as _);
                let scan = pg_sys::systable_beginscan(
                    relation,
                    pg_sys::InvalidOid,
                    false,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                );
                let mut databases = HashSet::default();
                loop {
                    let tuple = pg_sys::systable_getnext(scan);
                    if tuple.is_null() {
                        break;
                    }
                    let database = &*pgrx::GETSTRUCT!(tuple)
                        .get()
                        .cast::<pg_sys::FormData_pg_database>();
                    if database.datallowconn
                        && !database.datistemplate
                        && database.datconnlimit >= -1
                    {
                        databases.insert(database.oid);
                    }
                }
                pg_sys::systable_endscan(scan);
                pg_sys::table_close(relation, pg_sys::AccessShareLock as _);
                (databases, active)
            });
            for database in databases {
                if active.contains(&database) {
                    continue;
                }
                worker_context.switch_to(|_| {
                    let _ = BackgroundWorkerBuilder::new("pg_search mutable sweep")
                        .enable_spi_access()
                        .set_library("pg_search")
                        .set_function("mutable_sweep_database")
                        .set_argument(database.into_datum())
                        .load_dynamic();
                });
                worker_context.reset();
            }
        }
        if !BackgroundWorker::wait_latch(Some(WORKER_CHECK_INTERVAL)) {
            return;
        }
    }
}

#[pg_guard]
#[unsafe(no_mangle)]
unsafe extern "C-unwind" fn mutable_sweep_database(database: pg_sys::Datum) {
    connect(Some(pg_sys::Oid::from(database.value() as u32)));
    let Some(_worker_lock) = BackgroundWorker::transaction(|| {
        AdvisoryLock::conditional_lock_session(i64::from_be_bytes(*b"pdbsweep"))
    }) else {
        return;
    };
    // Debug installs use pgrx's temporary SQL version. Release workers must match
    // their loaded library, even when a newer package has replaced the control file.
    let development_version = if cfg!(debug_assertions) {
        BackgroundWorker::transaction(|| {
            Spi::get_one::<String>(
                "SELECT default_version FROM pg_catalog.pg_available_extensions WHERE name = 'pg_search'",
            )
            .expect("should be able to inspect pg_search's default version")
        })
    } else {
        None
    };
    let mut pass_context = pgrx::PgMemoryContexts::new("pg_search mutable sweep pass");
    let mut next_sweep = Instant::now();
    let mut interval = INTERVAL_MS.get();
    let mut previous_counts = HashMap::default();
    loop {
        reload();
        if INTERVAL_MS.get() != interval {
            interval = INTERVAL_MS.get();
            next_sweep = Instant::now();
            previous_counts.clear();
        }
        if interval == 0 {
            return;
        }
        let sweeping = Instant::now() >= next_sweep;
        let available = BackgroundWorker::transaction(|| {
            // Yield to database maintenance while idle; protect merges during an active pass.
            let tag = pg_sys::LOCKTAG {
                locktag_field1: pg_sys::InvalidOid.into(),
                locktag_field2: pg_sys::DatabaseRelationId.into(),
                locktag_field3: pg_sys::MyDatabaseId.into(),
                locktag_field4: 0,
                locktag_type: pg_sys::LockTagType::LOCKTAG_OBJECT as u8,
                locktag_lockmethodid: pg_sys::DEFAULT_LOCKMETHOD as u8,
            };
            if pg_sys::LockAcquire(&tag, pg_sys::RowExclusiveLock as _, false, true)
                == pg_sys::LockAcquireResult::LOCKACQUIRE_NOT_AVAIL
            {
                return false;
            }
            if sweeping {
                pg_sys::LockSharedObjectForSession(
                    pg_sys::DatabaseRelationId,
                    pg_sys::MyDatabaseId,
                    0,
                    pg_sys::RowExclusiveLock as _,
                );
            }
            true
        });
        if !available {
            return;
        }
        if sweeping {
            let observed_counts = pass_context.switch_to(|_| {
                let indexes = BackgroundWorker::transaction(|| {
                    let version = lookup_extension_version(c"pg_search");
                    if !sweep_version_matches(
                        version.as_deref(),
                        env!("CARGO_PKG_VERSION"),
                        development_version.as_deref(),
                    ) {
                        return None;
                    }
                    // Planner hooks can hide shard indexes or race with concurrent drops.
                    let indexes = paradedb_index_oids();
                    let mut current_counts = HashMap::default();
                    let indexes = free_leaf_partition_indexes(&indexes)
                        .filter_map(|index| {
                            if !(*index.rd_rel).relam.is_paradedb_am() || !index.is_valid() {
                                return None;
                            }
                            let mut idle_segments = HashMap::default();
                            MetaPage::open(&index).segment_metas().for_each(|_, entry| {
                                if entry.is_mutable()
                                    && entry.xmax() == pg_sys::InvalidTransactionId
                                {
                                    let key = (index.oid(), entry.segment_id());
                                    let count = entry.num_docs();
                                    current_counts.insert(key, count);
                                    if previous_counts.get(&key) == Some(&count) {
                                        idle_segments.insert(entry.segment_id(), count);
                                    }
                                }
                            });
                            (!idle_segments.is_empty()).then_some((index.oid(), idle_segments))
                        })
                        .collect::<Vec<_>>();
                    Some((indexes, current_counts))
                });
                let (indexes, current_counts) = indexes?;
                pg_sys::pgstat_report_appname(c"pg_search mutable sweep".as_ptr());
                for (index, idle_segments) in indexes {
                    reload();
                    if INTERVAL_MS.get() == 0 {
                        break;
                    }
                    BackgroundWorker::transaction(|| {
                        let Some(index) = free_leaf_partition_indexes(&[index]).next() else {
                            return;
                        };
                        if (*index.rd_rel).relam.is_paradedb_am() && index.is_valid() {
                            if VacuumSignal::new(index.oid()).wants_cancel() {
                                return;
                            }
                            let Some(_slot) =
                                MergeSlot::new(index.oid(), MergeSlotVariant::Small).lock()
                            else {
                                return;
                            };
                            let metadata = MetaPage::open(&index);
                            let Some(merge_pins) = metadata.try_pin_merge() else {
                                return;
                            };
                            let merge_lock = metadata.acquire_merge_lock();
                            PgTryBuilder::new(AssertUnwindSafe(|| {
                                let frozen = metadata.freeze_mutable_segments_matching(|entry| {
                                    idle_segments.get(&entry.segment_id())
                                        == Some(&entry.num_docs())
                                });
                                if frozen.is_empty() {
                                    return;
                                }
                                set_ps_display_suffix(MERGING.as_ptr());
                                pg_sys::pgstat_report_activity(
                                    pg_sys::BackendState::STATE_RUNNING,
                                    MERGING.as_ptr(),
                                );
                                let merger =
                                    SearchIndexMerger::open(&index, MvccSatisfies::Mergeable)
                                        .expect("should be able to open mutable sweeper merger");
                                let mut policy = LayeredMergePolicy::new(Vec::new());
                                policy.set_mergeable_segment_entries(
                                    &metadata,
                                    &merge_lock,
                                    &merger,
                                );
                                policy.retain(
                                    frozen.iter().map(|entry| entry.segment_id()).collect(),
                                );
                                let (candidates, _, skipped) = policy
                                    .simulate_supported(&merger)
                                    .expect("should be able to validate mutable sweep candidates");
                                warn_unsupported_vector_merge(&index, skipped);
                                let candidates = candidates
                                    .into_iter()
                                    .map(|(_, candidate)| candidate)
                                    .collect::<Vec<_>>();
                                let xid = pg_sys::GetCurrentFullTransactionId();
                                merge_index(
                                    &index,
                                    merger,
                                    &candidates,
                                    merge_lock,
                                    merge_pins,
                                    MergeMode::Background,
                                    xid,
                                    xid,
                                );
                            }))
                            .catch_others(|caught| {
                                dst::observe!(|| {
                                    if let Some(details) = merge_crash_details(&caught) {
                                        dst::assert_unreachable!(
                                            "pg_search mutable sweep crashed",
                                            &details
                                        );
                                    }
                                });
                                caught.rethrow()
                            })
                            .execute();
                        }
                    });
                }
                Some(current_counts)
            });
            pg_sys::UnlockSharedObjectForSession(
                pg_sys::DatabaseRelationId,
                pg_sys::MyDatabaseId,
                0,
                pg_sys::RowExclusiveLock as _,
            );
            pass_context.reset();
            let Some(current_counts) = observed_counts else {
                return;
            };
            previous_counts = current_counts;
            pg_sys::pgstat_report_activity(pg_sys::BackendState::STATE_IDLE, std::ptr::null());
            next_sweep = Instant::now() + Duration::from_millis(interval as u64);
        }
        if !BackgroundWorker::wait_latch(Some(
            next_sweep
                .saturating_duration_since(Instant::now())
                .min(DATABASE_CHECK_INTERVAL),
        )) {
            return;
        }
    }
}

fn sweep_version_matches(
    database_version: Option<&str>,
    library_version: &str,
    development_version: Option<&str>,
) -> bool {
    database_version == Some(development_version.unwrap_or(library_version))
}

#[cfg(test)]
mod version_tests {
    use super::sweep_version_matches;

    #[test]
    fn release_worker_rejects_a_different_schema_version() {
        assert!(sweep_version_matches(Some("0.26.6"), "0.26.6", None));
        assert!(!sweep_version_matches(Some("0.26.7"), "0.26.6", None));
        assert!(!sweep_version_matches(Some("0.26.5"), "0.26.6", None));
        assert!(!sweep_version_matches(None, "0.26.6", None));
    }

    #[test]
    fn debug_worker_accepts_only_its_temporary_schema_version() {
        assert!(sweep_version_matches(
            Some("0.26.7"),
            "0.26.6",
            Some("0.26.7")
        ));
        assert!(!sweep_version_matches(
            Some("0.26.6"),
            "0.26.6",
            Some("0.26.7")
        ));
    }
}

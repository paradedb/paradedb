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

use crate::api::{HashMap, HashSet};
use crate::index::merge_policy::LayeredMergePolicy;
use crate::index::mvcc::MvccSatisfies;
use crate::index::writer::index::SearchIndexMerger;
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
const WORKER_CHECK_INTERVAL: Duration = Duration::from_secs(1);

pub(crate) fn init() {
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

unsafe fn reload() -> bool {
    check_for_interrupts!();
    let reloaded = BackgroundWorker::sighup_received();
    if reloaded {
        pg_sys::ProcessConfigFile(pg_sys::GucContext::PGC_SIGHUP);
    }
    reloaded
}

#[pg_guard]
#[unsafe(no_mangle)]
unsafe extern "C-unwind" fn mutable_sweep_launcher(_arg: pg_sys::Datum) {
    connect(None);
    let mut worker_context = pgrx::PgMemoryContexts::new("pg_search sweep worker handles");
    let mut probe_after = HashMap::default();
    loop {
        if reload() {
            probe_after.clear();
        }
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
                    let database =
                        &*pg_sys::GETSTRUCT(tuple).cast::<pg_sys::FormData_pg_database>();
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
            probe_after
                .retain(|database, _| databases.contains(database) && !active.contains(database));
            for database in databases {
                if active.contains(&database)
                    || probe_after
                        .get(&database)
                        .is_some_and(|deadline| *deadline > Instant::now())
                {
                    continue;
                }
                worker_context.switch_to(|_| {
                    if BackgroundWorkerBuilder::new("pg_search mutable sweep")
                        .enable_spi_access()
                        .set_library("pg_search")
                        .set_function("mutable_sweep_database")
                        .set_argument(database.into_datum())
                        .load_dynamic()
                        .is_ok()
                    {
                        // Databases without pg_search need only an occasional installation check.
                        probe_after.insert(database, Instant::now() + Duration::from_secs(30));
                    }
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
    let mut pass_context = pgrx::PgMemoryContexts::new("pg_search mutable sweep pass");
    let mut next_sweep = Instant::now();
    let mut interval = INTERVAL_MS.get();
    loop {
        reload();
        if INTERVAL_MS.get() != interval {
            interval = INTERVAL_MS.get();
            next_sweep = Instant::now();
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
            let installed = pass_context.switch_to(|_| {
                let indexes = BackgroundWorker::transaction(|| {
                    let version = Spi::get_one::<String>(
                        "SELECT max(extversion::text) FROM pg_catalog.pg_extension WHERE extname = 'pg_search'",
                    )
                    .expect("should be able to inspect pg_search version");
                    if version.as_deref() != Some(env!("CARGO_PKG_VERSION")) {
                        return None;
                    }
                    let indexes = Spi::get_one::<Vec<pg_sys::Oid>>(
                        "SELECT array_agg(i.indexrelid ORDER BY i.indexrelid)
                         FROM pg_catalog.pg_index i
                         JOIN pg_catalog.pg_class c ON c.oid = i.indexrelid
                         JOIN pg_catalog.pg_am am ON am.oid = c.relam
                         JOIN pg_catalog.pg_proc p ON p.oid = am.amhandler
                         JOIN pg_catalog.pg_namespace n ON n.oid = p.pronamespace
                         WHERE n.nspname = 'paradedb' AND p.proname = 'bm25_handler'
                           AND i.indisvalid AND i.indisready AND c.relkind = 'i' AND c.relpersistence <> 't'",
                    )
                    .expect("should be able to discover ParadeDB indexes")
                    .unwrap_or_default();
                    Some(indexes)
                });
                let Some(indexes) = indexes else {
                    return false;
                };
                pg_sys::pgstat_report_appname(c"pg_search mutable sweep".as_ptr());
                for index in indexes {
                    reload();
                    if INTERVAL_MS.get() == 0 {
                        break;
                    }
                    BackgroundWorker::transaction(|| {
                        let Some(index) = free_leaf_partition_indexes(&[index]).next() else {
                            return;
                        };
                        use crate::postgres::catalog::OidExt;
                        if (*index.rd_rel).relam.is_paradedb_am() && index.is_valid() {
                            if VacuumSignal::new(index.oid()).wants_cancel() {
                                return;
                            }
                            let Some(_slot) = MergeSlot::new(index.oid(), MergeSlotVariant::Small).lock()
                            else {
                                return;
                            };
                            let metadata = MetaPage::open(&index);
                            let Some(merge_pins) = metadata.try_pin_merge() else {
                                return;
                            };
                            let merge_lock = metadata.acquire_merge_lock();
                            PgTryBuilder::new(AssertUnwindSafe(|| {
                                if metadata.freeze_mutable_segments().is_empty() {
                                    return;
                                }
                                set_ps_display_suffix(MERGING.as_ptr());
                                pg_sys::pgstat_report_activity(
                                    pg_sys::BackendState::STATE_RUNNING,
                                    MERGING.as_ptr(),
                                );
                                let merger = SearchIndexMerger::open(&index, MvccSatisfies::Mergeable)
                                    .expect("should be able to open mutable sweeper merger");
                                let mut policy = LayeredMergePolicy::new(Vec::new());
                                policy.set_mergeable_segment_entries(&metadata, &merge_lock, &merger);
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
                                        dst::assert_unreachable!("pg_search mutable sweep crashed", &details);
                                    }
                                });
                                caught.rethrow()
                            })
                            .execute();
                        }
                    });
                }
                true
            });
            pg_sys::UnlockSharedObjectForSession(
                pg_sys::DatabaseRelationId,
                pg_sys::MyDatabaseId,
                0,
                pg_sys::RowExclusiveLock as _,
            );
            pass_context.reset();
            if !installed {
                return;
            }
            pg_sys::pgstat_report_activity(pg_sys::BackendState::STATE_IDLE, std::ptr::null());
            next_sweep = Instant::now() + Duration::from_millis(interval as u64);
        }
        if !BackgroundWorker::wait_latch(Some(
            next_sweep
                .saturating_duration_since(Instant::now())
                .min(Duration::from_secs(1)),
        )) {
            return;
        }
    }
}

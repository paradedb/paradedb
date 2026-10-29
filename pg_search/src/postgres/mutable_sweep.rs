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

use crate::postgres::index::free_leaf_partition_indexes;
use crate::postgres::merge::sweep_mutable;
use pgrx::bgworkers::{BackgroundWorker, BackgroundWorkerBuilder, SignalWakeFlags};
use pgrx::{
    GucContext, GucFlags, GucRegistry, GucSetting, IntoDatum, Spi, check_for_interrupts, pg_guard,
    pg_sys,
};
use std::time::Duration;

static INTERVAL_MS: GucSetting<i32> = GucSetting::<i32>::new(1000);

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

unsafe fn reload() {
    check_for_interrupts!();
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
        let interval = INTERVAL_MS.get();
        if !BackgroundWorker::wait_latch(Some(Duration::from_millis(if interval == 0 {
            1000
        } else {
            interval as u64
        }))) {
            return;
        }
        reload();
        if INTERVAL_MS.get() == 0 {
            continue;
        }
        let databases = BackgroundWorker::transaction(|| {
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
            let mut databases = Vec::new();
            loop {
                let tuple = pg_sys::systable_getnext(scan);
                if tuple.is_null() {
                    break;
                }
                let database = &*pg_sys::GETSTRUCT(tuple).cast::<pg_sys::FormData_pg_database>();
                if database.datallowconn && !database.datistemplate && database.datconnlimit >= -1 {
                    databases.push(database.oid);
                }
            }
            pg_sys::systable_endscan(scan);
            pg_sys::table_close(relation, pg_sys::AccessShareLock as _);
            databases
        });
        for database in databases {
            reload();
            if INTERVAL_MS.get() == 0 {
                break;
            }
            worker_context.switch_to(|_| {
                let worker = BackgroundWorkerBuilder::new("pg_search mutable sweep")
                    .enable_spi_access()
                    .set_library("pg_search")
                    .set_function("mutable_sweep_database")
                    .set_argument(database.into_datum())
                    .set_notify_pid(pg_sys::MyProcPid)
                    .load_dynamic();
                if let Ok(worker) = worker {
                    let _ = worker.wait_for_shutdown();
                }
            });
            worker_context.reset();
        }
    }
}

#[pg_guard]
#[unsafe(no_mangle)]
unsafe extern "C-unwind" fn mutable_sweep_database(database: pg_sys::Datum) {
    connect(Some(pg_sys::Oid::from(database.value() as u32)));
    let indexes = BackgroundWorker::transaction(|| {
        let version = Spi::get_one::<String>(
            "SELECT max(extversion::text) FROM pg_catalog.pg_extension WHERE extname = 'pg_search'",
        )
        .expect("should be able to inspect pg_search version");
        if version.as_deref() != Some(env!("CARGO_PKG_VERSION")) {
            return Vec::new();
        }
        Spi::get_one::<Vec<pg_sys::Oid>>(
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
        .unwrap_or_default()
    });
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
                sweep_mutable(&index);
            }
        });
    }
}

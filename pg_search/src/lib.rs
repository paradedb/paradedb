// Copyright (c) 2023-2024 Retake, Inc.
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

mod api;
mod bootstrap;
mod env;
mod index;
mod postgres;
mod query;
mod schema;
mod writer;

#[cfg(test)]
pub mod fixtures;

use pgrx::*;
use shared::gucs::PostgresGlobalGucSettings;
use shared::telemetry::setup_telemetry_background_worker;

// A static variable is required to host grand unified configuration settings.
pub static GUCS: PostgresGlobalGucSettings = PostgresGlobalGucSettings::new();

// A hardcoded value when we can't figure out a good selectivity value
const UNKNOWN_SELECTIVITY: f64 = 0.00001;

// An arbitrary value for what it costs for a plan with one of our operators (@@@) to do whatever
// initial work it needs to do (open tantivy index, start the query, etc).  The value is largely
// meaningless but we should be honest that do _something_.
const DEFAULT_STARTUP_COST: f64 = 10.0;

pgrx::pg_module_magic!();

extension_sql!(
    "GRANT ALL ON SCHEMA paradedb TO PUBLIC;",
    name = "paradedb_grant_all",
    finalize
);

static mut TRACE_HOOK: shared::trace::TraceHook = shared::trace::TraceHook;

/// Convenience method for [`pgrx::pg_sys::MyDatabaseId`]
#[allow(non_snake_case)]
#[inline(always)]
pub fn MyDatabaseId() -> u32 {
    unsafe {
        // SAFETY:  this static is set by Postgres when the backend first connects and is
        // never changed afterwards.  As such, it'll always be set whenever this code runs
        pg_sys::MyDatabaseId.as_u32()
    }
}

/// Initializes option parsing and telemetry
#[allow(clippy::missing_safety_doc)]
#[allow(non_snake_case)]
#[pg_guard]
pub unsafe extern "C" fn _PG_init() {
    if !pg_sys::process_shared_preload_libraries_in_progress {
        error!("pg_search must be loaded via shared_preload_libraries. Add 'pg_search' to shared_preload_libraries in postgresql.conf and restart Postgres.");
    }

    postgres::options::init();
    GUCS.init("pg_search");

    setup_telemetry_background_worker(shared::telemetry::ParadeExtension::PgSearch);

    // Register our tracing / logging hook, so that we can ensure that the logger
    // is initialized for all connections.
    #[allow(static_mut_refs)]
    #[allow(deprecated)]
    pgrx::hooks::register_hook(&mut TRACE_HOOK);
}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}

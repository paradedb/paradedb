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
mod datafusion;
mod duckdb;
mod fdw;
mod hooks;
mod schema;

use hooks::LakehouseHook;
use pgrx::*;
use shared::{
    gucs::PostgresGlobalGucSettings,
    telemetry::{setup_telemetry_background_worker, ParadeExtension},
};

// A static variable is required to host grand unified configuration settings.
pub static GUCS: PostgresGlobalGucSettings = PostgresGlobalGucSettings::new();

pg_module_magic!();

static mut EXTENSION_HOOK: LakehouseHook = LakehouseHook;

#[pg_guard]
pub extern "C" fn _PG_init() {
    #[allow(static_mut_refs)]
    unsafe {
        register_hook(&mut EXTENSION_HOOK)
    };

    GUCS.init("pg_lakehouse");

    setup_telemetry_background_worker(ParadeExtension::PgLakehouse);
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}

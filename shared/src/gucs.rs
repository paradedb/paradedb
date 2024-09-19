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

use pgrx::{GucContext, GucFlags, GucRegistry, GucSetting};

pub trait GlobalGucSettings {
    fn telemetry_enabled(&self) -> bool;

    fn enable_custom_scan(&self) -> bool;
}

pub struct PostgresGlobalGucSettings {
    telemetry: GucSetting<bool>,
    enable_custom_scan: GucSetting<bool>,
}

impl PostgresGlobalGucSettings {
    pub const fn new() -> Self {
        Self {
            telemetry: GucSetting::<bool>::new(true),
            enable_custom_scan: GucSetting::<bool>::new(true),
        }
    }

    pub fn init(&self, extension_name: &str) {
        // Note that Postgres is very specific about the naming convention of variables.
        // They must be namespaced... we use 'paradedb.<variable>' below.
        // They cannot have more than one '.' - paradedb.pg_search.telemetry will not work.

        // telemetry
        GucRegistry::define_bool_guc(
            &format!("paradedb.{extension_name}_telemetry"),
            &format!("Enable telemetry on the ParadeDB {extension_name} extension.",),
            &format!("Enable telemetry on the ParadeDB {extension_name} extension.",),
            &self.telemetry,
            GucContext::Userset,
            GucFlags::default(),
        );
        GucRegistry::define_bool_guc(
            "paradedb.enable_custom_scan",
            "Enable ParadeDB's custom scan",
            "Enable ParadeDB's custom scan",
            &self.enable_custom_scan,
            GucContext::Userset,
            GucFlags::default(),
        );
    }
}

impl Default for PostgresGlobalGucSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalGucSettings for PostgresGlobalGucSettings {
    fn telemetry_enabled(&self) -> bool {
        // If PARADEDB_TELEMETRY is not 'true' at compile time, then we will never enable.
        // This is useful for test builds and CI.
        option_env!("PARADEDB_TELEMETRY") == Some("true") && self.telemetry.get()
    }

    fn enable_custom_scan(&self) -> bool {
        self.enable_custom_scan.get()
    }
}

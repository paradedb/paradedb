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

    /// The `per_tuple_cost` is an arbitrary value that needs to be really high.  In fact, we default
    /// to one hundred million.
    ///
    /// The reason for this is we really do not want Postgres to choose a plan where the `@@@` operator
    /// is used in a sequential scan, filter, or recheck condition... unless of course there's no
    /// other way to solve the query.
    ///
    /// This value is a multiplier that Postgres applies to the estimated row count any given `@@@`
    /// query clause will return.  In our case, higher is better.
    ///
    /// Our IAM impl has its own costing functions that don't use this GUC and provide sensible estimates
    /// for the overall IndexScan.  That plus this help to persuade Postgres to use our IAM whenever
    /// it logically can.
    fn per_tuple_cost(&self) -> f64;
}

pub struct PostgresGlobalGucSettings {
    telemetry: GucSetting<bool>,
    per_tuple_cost: GucSetting<f64>,
}

impl PostgresGlobalGucSettings {
    pub const fn new() -> Self {
        Self {
            telemetry: GucSetting::<bool>::new(true),
            per_tuple_cost: GucSetting::<f64>::new(100_000_000.0),
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

        // per_tuple_cost
        GucRegistry::define_float_guc(
            "paradedb.per_tuple_cost",
            "Arbitrary multiplier for the cost of retrieving a tuple from a USING bm25 index outside of an IndexScan",
            "Default is 100,000,000.0.  It is very expensive to use a USING bm25 index in the wrong query plan",
            &self.per_tuple_cost,
            0.0,
            f64::MAX,
            GucContext::Userset,
            GucFlags::default(),
        )
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

    fn per_tuple_cost(&self) -> f64 {
        self.per_tuple_cost.get()
    }
}

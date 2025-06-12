// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use pgrx::{pg_sys, GucContext, GucFlags, GucRegistry, GucSetting};

/// Allows the user to toggle the use of our "ParadeDB Custom Scan".  The default is `true`.
static ENABLE_CUSTOM_SCAN: GucSetting<bool> = GucSetting::<bool>::new(true);

/// Allows the user to enable or disable the FastFieldsExecState executor. Default is `true`.
static ENABLE_FAST_FIELD_EXEC: GucSetting<bool> = GucSetting::<bool>::new(true);

/// Allows the user to enable or disable the MixedFastFieldExecState executor. Default is `true`.
static ENABLE_MIXED_FAST_FIELD_EXEC: GucSetting<bool> = GucSetting::<bool>::new(true);

/// The number of fast-field columns below-which the MixedFastFieldExecState will be used, rather
/// than the NormalExecState. The Mixed execution mode fetches data as column-oriented, whereas
/// the Normal mode fetches data as row-oriented.
///
/// Each fetch from a fast-field column costs one or two disk seeks, whereas a fetch of a row
/// generally costs one. But with a wide enough row, fetching multiple columns might still result
/// in better cache performance than fetching a row.
static MIXED_FAST_FIELD_EXEC_COLUMN_THRESHOLD: GucSetting<i32> = GucSetting::<i32>::new(3);
static MIXED_FAST_FIELD_EXEC_COLUMN_THRESHOLD_NAME: &str =
    "paradedb.mixed_fast_field_exec_column_threshold";

/// The `PER_TUPLE_COST` is an arbitrary value that needs to be really high.  In fact, we default
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
static PER_TUPLE_COST: GucSetting<f64> = GucSetting::<f64>::new(100_000_000.0);

pub fn init() {
    // Note that Postgres is very specific about the naming convention of variables.
    // They must be namespaced... we use 'paradedb.<variable>' below.

    GucRegistry::define_bool_guc(
        "paradedb.enable_custom_scan",
        "Enable ParadeDB's custom scan",
        "Enable ParadeDB's custom scan",
        &ENABLE_CUSTOM_SCAN,
        GucContext::Userset,
        GucFlags::default(),
    );

    GucRegistry::define_bool_guc(
        "paradedb.enable_fast_field_exec",
        "Enable StringFastFieldsExecState and NumericFastFieldsExecState executor",
        "Enable the StringFastFieldsExecState and NumericFastFieldsExecState executors for handling one string fast field or multiple numeric fast fields",
        &ENABLE_FAST_FIELD_EXEC,
        GucContext::Userset,
        GucFlags::default(),
    );

    GucRegistry::define_bool_guc(
        "paradedb.enable_mixed_fast_field_exec",
        "Enable MixedFastFieldExecState executor",
        "Enable the MixedFastFieldExecState executor for handling multiple string fast fields or mixed string/numeric fast fields",
        &ENABLE_MIXED_FAST_FIELD_EXEC,
        GucContext::Userset,
        GucFlags::default(),
    );

    GucRegistry::define_int_guc(
        MIXED_FAST_FIELD_EXEC_COLUMN_THRESHOLD_NAME,
        "Threshold of fetched columns below which MixedFastFieldExecState will be used.",
        "The number of fast-field columns below-which the MixedFastFieldExecState will be used, rather \
         than the NormalExecState. The Mixed execution mode fetches data as column-oriented, whereas \
         the Normal mode fetches data as row-oriented.",
        &MIXED_FAST_FIELD_EXEC_COLUMN_THRESHOLD,
        0,
        i32::MAX,
        GucContext::Userset,
        GucFlags::default(),
    );

    GucRegistry::define_float_guc(
        "paradedb.per_tuple_cost",
        "Arbitrary multiplier for the cost of retrieving a tuple from a USING bm25 index outside of an IndexScan",
        "Default is 100,000,000.0.  It is very expensive to use a USING bm25 index in the wrong query plan",
        &PER_TUPLE_COST,
        0.0,
        f64::MAX,
        GucContext::Userset,
        GucFlags::default(),
    );
}

pub fn enable_custom_scan() -> bool {
    ENABLE_CUSTOM_SCAN.get()
}

pub fn is_fast_field_exec_enabled() -> bool {
    ENABLE_FAST_FIELD_EXEC.get()
}

pub fn is_mixed_fast_field_exec_enabled() -> bool {
    ENABLE_MIXED_FAST_FIELD_EXEC.get()
}

pub fn mixed_fast_field_exec_column_threshold() -> usize {
    MIXED_FAST_FIELD_EXEC_COLUMN_THRESHOLD
        .get()
        .try_into()
        .unwrap_or_else(|e| {
            panic!("{MIXED_FAST_FIELD_EXEC_COLUMN_THRESHOLD_NAME} must be positive. {e}");
        })
}

pub fn per_tuple_cost() -> f64 {
    PER_TUPLE_COST.get()
}

pub fn adjust_mem(mem: usize, nlaunched: usize, min_mem_per_worker: usize) -> usize {
    // NB:  These limits come from [`tantivy::index_writer::MEMORY_BUDGET_NUM_BYTES_MAX`], which is not publicly exposed
    mod limits {
        // Size of the margin for the `memory_arena`. A segment is closed when the remaining memory
        // in the `memory_arena` goes below MARGIN_IN_BYTES.
        pub const MARGIN_IN_BYTES: usize = 1_000_000;

        // We impose the memory per thread to be no greater than 4GB as that's tantivy's limit
        pub const MEMORY_BUDGET_NUM_BYTES_MAX: usize = (4 * 1024 * 1024 * 1024) - MARGIN_IN_BYTES;
    }

    let mwm_as_bytes = mem * 1024;
    let nlaunched = nlaunched.max(1);
    let per_worker_budget = mwm_as_bytes / nlaunched;

    // clamp the per_thread_budget to the min/max values
    let per_worker_budget = per_worker_budget.clamp(
        limits::MARGIN_IN_BYTES * min_mem_per_worker,
        limits::MEMORY_BUDGET_NUM_BYTES_MAX - 1,
    );

    per_worker_budget * nlaunched
}

pub fn adjust_maintenance_work_mem(nlaunched: usize) -> usize {
    adjust_mem(
        unsafe { pg_sys::maintenance_work_mem as usize },
        nlaunched,
        64,
    )
}

pub fn adjust_work_mem(nlaunched: usize) -> usize {
    adjust_mem(unsafe { pg_sys::work_mem as usize }, nlaunched, 15)
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::prelude::*;

    macro_rules! assert_approx_eq {
        ($a:expr, $b:expr, $percent:expr) => {{
            let a = $a;
            let b = $b;
            let diff = if a > b { a - b } else { b - a };
            let max_val = a.max(b);
            let max_diff = ((max_val as f64) * ($percent as f64 / 100.0)).ceil() as usize;

            assert!(
                diff <= max_diff,
                "assertion failed: `a = {}`, `b = {}` differ by more than {}% (allowed: {}, actual: {})",
                a,
                b,
                $percent,
                max_diff,
                diff
            );
        }};
    }

    #[pg_test]
    fn test_adjust_maintenance_work_mem() {
        Spi::run("SET maintenance_work_mem = '16MB';").unwrap();
        assert_eq!(adjust_maintenance_work_mem(0), 64 * 1_000_000);
        assert_eq!(adjust_maintenance_work_mem(1), 64 * 1_000_000);
        assert_eq!(adjust_maintenance_work_mem(2), 128 * 1_000_000);
        assert_eq!(adjust_maintenance_work_mem(10), 640 * 1_000_000);

        Spi::run("SET maintenance_work_mem = '1GB';").unwrap();
        assert_approx_eq!(adjust_maintenance_work_mem(0), 1024 * 1024 * 1024, 1.0);
        assert_approx_eq!(adjust_maintenance_work_mem(1), 1024 * 1024 * 1024, 1.0);
        assert_approx_eq!(adjust_maintenance_work_mem(2), 1024 * 1024 * 1024, 1.0);
        assert_approx_eq!(adjust_maintenance_work_mem(10), 1024 * 1024 * 1024, 1.0);
        assert_approx_eq!(adjust_maintenance_work_mem(32), 64 * 32 * 1_000_000, 1.0);

        Spi::run("SET maintenance_work_mem = '8GB';").unwrap();
        assert_approx_eq!(adjust_maintenance_work_mem(0), 4 * 1024 * 1024 * 1024, 1.0);
        assert_approx_eq!(adjust_maintenance_work_mem(1), 4 * 1024 * 1024 * 1024, 1.0);
        assert_approx_eq!(adjust_maintenance_work_mem(2), 8 * 1024 * 1024 * 1024, 1.0);
        assert_approx_eq!(adjust_maintenance_work_mem(10), 8 * 1024 * 1024 * 1024, 1.0);
        assert_approx_eq!(adjust_maintenance_work_mem(256), 64 * 256 * 1_000_000, 1.0);
    }
}

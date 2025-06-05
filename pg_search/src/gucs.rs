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

use crate::index::Parallelism;
use pgrx::{pg_sys, GucContext, GucFlags, GucRegistry, GucSetting};
use std::num::NonZeroUsize;

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

/// How much memory should tantivy use during CREATE INDEX.  This value is decided to each indexing
/// thread.  So if there's 10 threads and this value is 100MB, then a total of 1GB will be allocated.
static CREATE_INDEX_MEMORY_BUDGET: GucSetting<i32> = GucSetting::<i32>::new(1024);

/// How much memory should tantivy use during a regular INSERT/UPDATE/COPY statement?  This value is decided to each indexing
/// thread.  So if there's 10 threads and this value is 100MB, then a total of 1GB will be allocated.
static STATEMENT_MEMORY_BUDGET: GucSetting<i32> = GucSetting::<i32>::new(1024);

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

    GucRegistry::define_int_guc(
        "paradedb.create_index_memory_budget",
        "The amount of memory to allocate to 1 indexing process",
        "Default is `1GB`, which is allocated to each indexing process defined by `max_parallel_maintenance_workers`",
        &CREATE_INDEX_MEMORY_BUDGET,
        0,
        i32::MAX,
        GucContext::Userset,
        GucFlags::UNIT_MB,
    );

    GucRegistry::define_int_guc(
        "paradedb.statement_memory_budget",
        "The amount of memory to allocate to 1 thread during an INSERT/UPDATE/COPY statement",
        "Default is `1GB`, which is allocated to each thread defined by `paradedb.statement_parallelism`",
        &STATEMENT_MEMORY_BUDGET,
        0,
        i32::MAX,
        GucContext::Userset,
        GucFlags::UNIT_MB,
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

pub fn create_index_parallelism() -> NonZeroUsize {
    let nthreads = unsafe {
        let mut nthreads = pg_sys::max_parallel_maintenance_workers;
        if pg_sys::parallel_leader_participation {
            nthreads += 1
        }
        nthreads
    };
    adjust_nthreads(nthreads)
}

pub fn create_index_memory_budget() -> usize {
    let memory_budget = CREATE_INDEX_MEMORY_BUDGET.get();
    adjust_budget(memory_budget, create_index_parallelism())
}

// NB:  this is always `1` (one).  There's no (longer) a concept of parallelism during INSERTS/UPDATES
pub fn statement_parallelism() -> NonZeroUsize {
    unsafe { NonZeroUsize::new_unchecked(1) }
}

pub fn statement_memory_budget() -> usize {
    adjust_budget(STATEMENT_MEMORY_BUDGET.get(), statement_parallelism())
}

fn adjust_nthreads(nthreads: i32) -> NonZeroUsize {
    let nthreads = if nthreads <= 0 {
        std::thread::available_parallelism()
            .expect("your computer should have at least one core")
            .get()
    } else {
        nthreads as usize
    };

    unsafe {
        // SAFETY:  we ensured above that nthreads is > 0
        NonZeroUsize::new_unchecked(nthreads)
    }
}

fn adjust_budget(per_thread_budget: i32, parallelism: Parallelism) -> usize {
    // NB:  These limits come from [`tantivy::index_writer::MEMORY_BUDGET_NUM_BYTES_MAX`], which is not publicly exposed
    mod limits {
        // Size of the margin for the `memory_arena`. A segment is closed when the remaining memory
        // in the `memory_arena` goes below MARGIN_IN_BYTES.
        pub const MARGIN_IN_BYTES: usize = 1_000_000;

        // We impose the memory per thread to be at least 15 MB, as the baseline consumption is 12MB,
        // and no greater than 4GB as that's tantivy's limit
        pub const MEMORY_BUDGET_NUM_BYTES_MIN: usize = ((MARGIN_IN_BYTES as u32) * 15u32) as usize;
        pub const MEMORY_BUDGET_NUM_BYTES_MAX: usize = (4 * 1024 * 1024 * 1024) - MARGIN_IN_BYTES;
    }

    let per_thread_budget = if per_thread_budget <= 0 {
        // value is unset, so we'll use the maintenance_work_mem, divided between the parallelism value
        let mwm_as_bytes = unsafe {
            // SAFETY:  Postgres sets maintenance_work_mem when it starts up
            pg_sys::maintenance_work_mem as usize * 1024 // convert from kilobytes to bytes
        };

        mwm_as_bytes / parallelism.get()
    } else {
        per_thread_budget as usize * 1024 * 1024 // convert from megabytes to bytes
    };

    // clamp the per_thread_budget to the min/max values
    let per_thread_budget = per_thread_budget.clamp(
        limits::MEMORY_BUDGET_NUM_BYTES_MIN,
        limits::MEMORY_BUDGET_NUM_BYTES_MAX - 1,
    );

    per_thread_budget * parallelism.get()
}

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
#![recursion_limit = "512"]

mod aggregate;
mod api;
mod bootstrap;
mod index;
mod postgres;
mod query;
mod schema;

pub mod gucs;
pub mod parallel_worker;

use self::postgres::customscan;
use pgrx::*;

/// Postgres' value for a `norm_selec` that hasn't been assigned
const UNASSIGNED_SELECTIVITY: f64 = -1.0;

/// A hardcoded value when we can't figure out a good selectivity value
const UNKNOWN_SELECTIVITY: f64 = 0.00001;

/// A hardcoded value for parameterized plan queries
const PARAMETERIZED_SELECTIVITY: f64 = 0.10;

/// The selectivity value indicating the entire relation will be returned
const FULL_RELATION_SELECTIVITY: f64 = 1.0;

/// An arbitrary value for what it costs for a plan with one of our operators (@@@) to do whatever
/// initial work it needs to do (open tantivy index, start the query, etc).  The value is largely
/// meaningless but we should be honest that do _something_.
const DEFAULT_STARTUP_COST: f64 = 10.0;

pgrx::pg_module_magic!();

extension_sql!(
    r#"
        GRANT ALL ON SCHEMA paradedb TO PUBLIC;
        GRANT ALL ON SCHEMA pdb TO PUBLIC;

        GRANT USAGE, CREATE ON SCHEMA pdb TO PUBLIC;

        GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA pdb TO PUBLIC;
        GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA pdb TO PUBLIC;
        GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA pdb TO PUBLIC;

        ALTER DEFAULT PRIVILEGES IN SCHEMA pdb GRANT ALL PRIVILEGES ON TABLES TO PUBLIC;
        ALTER DEFAULT PRIVILEGES IN SCHEMA pdb GRANT ALL PRIVILEGES ON SEQUENCES TO PUBLIC;
        ALTER DEFAULT PRIVILEGES IN SCHEMA pdb GRANT ALL PRIVILEGES ON FUNCTIONS TO PUBLIC;
        ALTER DEFAULT PRIVILEGES IN SCHEMA pdb GRANT USAGE ON TYPES TO PUBLIC;
    "#,
    name = "paradedb_grant_all",
    finalize
);

pub fn available_parallelism() -> usize {
    use once_cell::sync::Lazy;

    static AVAILABLE_PARALLELISM: Lazy<usize> = Lazy::new(|| {
        std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1)
    });
    *AVAILABLE_PARALLELISM
}

/// Initializes option parsing
#[allow(clippy::missing_safety_doc)]
#[allow(non_snake_case)]
#[pg_guard]
pub unsafe extern "C-unwind" fn _PG_init() {
    // Optional: initialize env_logger for dependency debug output (tantivy, lindera, etc.)
    // Enable with: cargo build --features debug-logging
    // Without this feature, log! macros (to stderr) in dependencies become no-ops.
    // We can't implement our own logger that sends messages to Postgres `ereport()` because
    // of threading concerns.
    #[cfg(feature = "debug-logging")]
    {
        std::env::set_var("RUST_LOG", "warn");
        std::env::set_var("RUST_LOG_STYLE", "never");
        // Use try_init() because parallel workers may call _PG_init() multiple times
        let _ = env_logger::try_init();
    }

    if cfg!(not(any(feature = "pg17", feature = "pg18")))
        && !pg_sys::process_shared_preload_libraries_in_progress
    {
        error!("pg_search must be loaded via shared_preload_libraries. Add 'pg_search' to shared_preload_libraries in postgresql.conf and restart Postgres.");
    }

    postgres::options::init();
    gucs::init();

    #[cfg(not(any(feature = "pg17", feature = "pg18")))]
    postgres::fake_aminsertcleanup::register();

    #[allow(static_mut_refs)]
    #[allow(deprecated)]
    customscan::register_rel_pathlist(customscan::basescan::BaseScan);
    customscan::register_upper_path(customscan::aggregatescan::AggregateScan);
    customscan::register_join_pathlist(customscan::joinscan::JoinScan);

    // Register global planner hook for window function support
    customscan::register_window_aggregate_hook();

    // Initialize the filter query builder
    customscan::aggregatescan::filterquery::init_filter_query_builder();
}

#[pg_extern]
fn random_words(num_words: i32) -> String {
    use rand::Rng;

    let mut rng = rand::rng();
    let letters = "abcdefghijklmnopqrstuvwxyz";
    let mut result = String::new();

    for _ in 0..num_words {
        // Choose a random word length between 3 and 7.
        let word_length = rng.random_range(3..=7);
        let mut word = String::new();

        for _ in 0..word_length {
            // Pick a random letter from the letters string.
            let random_index = rng.random_range(0..letters.len());
            // Safe to use .unwrap() because the index is guaranteed to be valid.
            let letter = letters.chars().nth(random_index).unwrap();
            word.push(letter);
        }
        result.push_str(&word);
        result.push(' ');
    }
    result.trim_end().to_string()
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

        let mut options: Vec<&'static str> = Vec::new();

        if cfg!(not(any(feature = "pg17", feature = "pg18"))) {
            options.push("shared_preload_libraries='pg_search'");
        }

        options
    }
}

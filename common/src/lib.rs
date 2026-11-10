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

pub use pgrx::pgrx_sql_entity_graph;

pub mod allocator;
pub mod api;
pub mod gucs;
pub mod index;
pub mod parallel_worker;
pub mod postgres;
pub mod query;
pub mod scan;
pub mod schema;
pub mod vector;

/// Postgres' value for a `norm_selec` that hasn't been assigned
pub const UNASSIGNED_SELECTIVITY: f64 = -1.0;

/// A hardcoded value when we can't figure out a good selectivity value
pub const UNKNOWN_SELECTIVITY: f64 = 0.00001;

/// A hardcoded value for parameterized plan queries
pub const PARAMETERIZED_SELECTIVITY: f64 = 0.10;

/// The selectivity value indicating the entire relation will be returned
pub const FULL_RELATION_SELECTIVITY: f64 = 1.0;

/// Heuristic selectivity for fuzzy queries with distance <= 1
pub const FUZZY_LOW_SELECTIVITY: f64 = 0.01;

/// Heuristic selectivity for fuzzy queries with distance >= 2
pub const FUZZY_HIGH_SELECTIVITY: f64 = 0.05;

/// Heuristic selectivity for regex queries
pub const REGEX_SELECTIVITY: f64 = 0.01;

/// Heuristic selectivity for more-like-this queries
pub const MORE_LIKE_THIS_SELECTIVITY: f64 = 0.01;

/// Scales the heuristic match estimate into a `DocSet::cost()` for fuzzy/regex/MLT, whose
/// real scorer is too expensive to build at plan time (#4172).
pub const EXPENSIVE_QUERY_COST_FACTOR: f64 = 25.0;

/// An arbitrary value for what it costs for a plan with one of our operators (@@@) to do whatever
/// initial work it needs to do (open tantivy index, start the query, etc).
pub const DEFAULT_STARTUP_COST: f64 = 10.0;

/// Planning-time row estimate used when LIMIT is parameterized (resolved only at
/// execution time).
pub const DEFAULT_PARAMETERIZED_LIMIT_ESTIMATE: f64 = 1000.0;

/// Batch size for Tantivy collectors.
pub const COLLECTOR_BATCH_SIZE: usize = 64;

pub fn available_parallelism() -> usize {
    use once_cell::sync::Lazy;

    static AVAILABLE_PARALLELISM: Lazy<usize> = Lazy::new(|| {
        std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1)
    });
    *AVAILABLE_PARALLELISM
}

#[cfg(any(test, feature = "pg_test"))]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        let pg_search_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("pg_search");
        unsafe {
            // Strip per-crate Cargo test-runner environment variables inherited by the nested
            // `cargo pgrx install` invocation in `pgrx-tests`. Without this, build scripts
            // that emit `cargo:rerun-if-env-changed=CARGO_*` (such as `ring`) see values set by
            // `cargo test` for the current test binary (`CARGO_CRATE_NAME`, `CARGO_PKG_*`, etc.)
            // that were unset during the outer `cargo` build and recompile the workspace.
            for (k, _) in std::env::vars() {
                if k.starts_with("CARGO_PKG_")
                    || matches!(
                        k.as_str(),
                        "CARGO_CRATE_NAME"
                            | "CARGO_BIN_NAME"
                            | "CARGO_PRIMARY_PACKAGE"
                            | "CARGO_MANIFEST_PATH"
                            | "CARGO_TARGET_TMPDIR"
                    )
                {
                    std::env::remove_var(&k);
                }
            }
            std::env::set_var("CARGO_MANIFEST_DIR", &pg_search_dir);
            std::env::set_var("PGRX_MANIFEST_PATH", pg_search_dir.join("Cargo.toml"));
        }
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![
            "shared_preload_libraries='pg_search'",
            "paradedb.mutable_sweep_interval=0",
            "max_worker_processes=64",
            "max_parallel_workers=64",
        ]
    }
}

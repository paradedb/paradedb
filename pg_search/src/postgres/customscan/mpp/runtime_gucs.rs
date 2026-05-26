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

//! Backend-thread snapshot of every GUC that MPP compute paths read.
//!
//! pgrx 0.18 holds a single-thread FFI invariant: `pgrx::GucSetting::get` calls
//! `check_active_thread()` and panics when called off the Postgres backend
//! thread. The MPP worker today runs every fragment future on a current-thread
//! tokio runtime pinned to the backend, so direct `crate::gucs::FOO()` reads in
//! compute paths work — but moving to a multi-thread runtime (the G7-MT
//! follow-up) breaks them.
//!
//! [`MppRuntimeGucs::snapshot`] reads every relevant GUC once on the backend
//! thread; [`build_mpp_session_context`] installs the snapshot as a
//! [`ConfigExtension`] on the per-query [`SessionConfig`] so compute futures
//! can fetch it via [`runtime_gucs_from_ctx`] without a backend round-trip.
//!
//! New compute-path GUC reads should be added to this struct and to
//! [`MppRuntimeGucs::snapshot`]; treat the struct as the closed enumeration of
//! "what compute may read without touching pgrx".

use datafusion::common::extensions_options;
use datafusion::config::ConfigExtension;
use datafusion::execution::TaskContext;

extensions_options! {
    pub struct MppRuntimeGucs {
        pub mpp_trace: bool, default = false
        pub mpp_debug: bool, default = false
        pub dynamic_filter_batch_size: usize, default = 0
        pub hash_join_inlist_pushdown_max_size: usize, default = 0
        pub hash_join_inlist_pushdown_max_distinct_values: usize, default = 0
        pub term_set_gallop_enabled: bool, default = true
        pub term_set_bitset_max_density_unique: f64, default = 0.0
        pub term_set_bitset_max_density_multi: f64, default = 0.0
        /// Snapshot of `paradedb.mpp_worker_count` at query start. Read by replicated-source
        /// scans (non-partitioning sources under MPP) to compute their doc-modulo slice so
        /// each worker emits 1/n_workers of the data instead of the full data N times.
        pub mpp_worker_count: usize, default = 0
    }
}

impl ConfigExtension for MppRuntimeGucs {
    const PREFIX: &'static str = "paradedb_mpp_runtime";
}

impl MppRuntimeGucs {
    /// Snapshot the current GUC values into a self-contained struct.
    ///
    /// MUST be called on the backend thread (the one holding `PGPROC`). The
    /// per-query installer in [`build_mpp_session_context`] takes care of this;
    /// compute futures should never call it directly.
    pub fn snapshot() -> Self {
        Self {
            mpp_trace: crate::gucs::mpp_trace(),
            mpp_debug: crate::gucs::mpp_debug(),
            dynamic_filter_batch_size: crate::gucs::dynamic_filter_batch_size().max(0) as usize,
            hash_join_inlist_pushdown_max_size: crate::gucs::hash_join_inlist_pushdown_max_size()
                .max(0) as usize,
            hash_join_inlist_pushdown_max_distinct_values:
                crate::gucs::hash_join_inlist_pushdown_max_distinct_values().max(0) as usize,
            term_set_gallop_enabled: crate::gucs::term_set_gallop_enabled(),
            term_set_bitset_max_density_unique: crate::gucs::term_set_bitset_max_density_unique(),
            term_set_bitset_max_density_multi: crate::gucs::term_set_bitset_max_density_multi(),
            mpp_worker_count: crate::gucs::mpp_worker_count().max(0) as usize,
        }
    }
}

/// Read the per-query MPP GUC snapshot stored on `ctx`'s `SessionConfig`.
///
/// Returns `None` when the scan is running outside an MPP context (e.g. the
/// non-MPP serial DataFusion path), in which case callers should fall back to
/// reading the live GUC value — that path is guaranteed to be on the backend
/// thread.
pub fn runtime_gucs_from_ctx(ctx: &TaskContext) -> Option<&MppRuntimeGucs> {
    ctx.session_config()
        .options()
        .extensions
        .get::<MppRuntimeGucs>()
}

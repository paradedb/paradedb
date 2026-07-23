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

//! MPP (Massively Parallel Processing) plan partitioning for JoinScan and AggregateScan.
//!
//! Hash-partitions every table by the join key and shuffles intermediate rows between
//! workers through PostgreSQL `shm_mq` queues, so each row is scanned exactly once.
//! Guarded by `paradedb.mpp_worker_count >= 3`, plus enough `max_parallel_workers` /
//! `max_parallel_workers_per_gather` to launch the producers.
//!
//! The transport lives in `datafusion_distributed::shm`. Deadlock avoidance is a
//! cooperative inline drain: a producer stalled on a full outbound pulls its own inbound
//! before retrying, so every peer sending at once cannot wedge.

pub mod dispatch;
pub mod exec_worker;
pub mod glue;
pub mod interrupt;
pub mod launch;
pub mod pg_seams;
pub mod task_estimator;
pub mod worker_fragments;

/// Emit a runtime trace when `paradedb.mpp_debug` is on.
///
/// Routed through `pgrx::warning!` so the line lands in the Postgres server log (and CI bench
/// logs). Gated `#[cfg(not(test))]` because `pgrx::warning!` expands to PG's `ereport` machinery,
/// which the lib-test binary doesn't link against; see the `#[cfg(test)]` no-op stub below.
#[cfg(not(test))]
#[macro_export]
macro_rules! mpp_log {
    ($($arg:tt)*) => {
        if $crate::gucs::mpp_debug() {
            pgrx::warning!($($arg)*);
        }
    };
}

/// `cargo test` variant: no-op. `format_args!` is invoked solely to silence
/// "unused variable" / "unused import" warnings at the call sites.
#[cfg(test)]
#[macro_export]
macro_rules! mpp_log {
    ($($arg:tt)*) => {
        { let _ = format_args!($($arg)*); }
    };
}

/// Fatal MPP-internal invariant breach. Same `!` return type as `pgrx::error!` / `panic!`, so it
/// can sit in any `match` arm or expression position.
///
/// In production this calls `pgrx::error!`, which aborts the transaction via PG's ereport
/// machinery. The lib-test binary (built without `pg_test`) does not link those PG symbols, so
/// the `#[cfg(test)]` arm falls back to `panic!`. Either way the call site stays readable —
/// `fail_loud(format!(...))` — without a per-site `#[cfg]` pair.
#[cfg(not(test))]
pub fn fail_loud(msg: String) -> ! {
    pgrx::error!("{}", msg);
}

#[cfg(test)]
pub fn fail_loud(msg: String) -> ! {
    panic!("{}", msg);
}

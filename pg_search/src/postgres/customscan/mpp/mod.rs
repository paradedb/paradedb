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

//! MPP (Massively Parallel Processing) plan partitioning for `JoinScan` and `AggregateScan`.
//!
//! Hash-partitions or range-partitions tables across parallel workers and shuffles intermediate
//! rows between stages, so each row is scanned at most once. Guarded by PostgreSQL's own
//! parallelism budget: active iff `min(max_parallel_workers_per_gather, max_parallel_workers) >= 2`
//! producers (`#5667`), and the plan-first launch spawns only as many workers as the plan's task
//! stages can occupy.
//!
//! # Architecture
//!
//! ## 1. Standard Tonic/gRPC Transport over DSM `AsyncRead`/`AsyncWrite` Streams
//!
//! Rather than maintaining a custom shared-memory RPC protocol inside `datafusion-distributed`,
//! `pg_search` implements Tokio `AsyncRead + AsyncWrite + tonic::transport::server::Connected`
//! over per-query PostgreSQL DSM SPSC byte rings ([`dsm_stream::DsmAsyncStream`]) and plugs into
//! upstream `datafusion-distributed`'s standard gRPC surface:
//!
//! - Client side ([`worker::DsmGrpcChannelResolver`]): implements `ChannelResolver` using
//!   `tonic::transport::Endpoint::connect_with_connector_lazy` + `hyper_util::rt::TokioIo` and
//!   `datafusion_distributed::grpc::create_worker_client`.
//! - Server side ([`worker::run_mpp_worker`]): runs `datafusion_distributed::Worker` inside
//!   `tonic::transport::Server::builder().add_service(worker.into_worker_server()).serve_with_incoming_shutdown(...)`.
//!
//! ## 2. DSM SPSC Byte-Ring Topology & Wakeup Pool
//!
//! - Ring topology ([`dsm_stream`]):
//!   - Each directed connection between `(client_proc, server_proc)` is a pair of lock-free SPSC
//!     byte rings (`client -> server` and `server -> client`, 64 KiB per ring) in the per-query
//!     DSM segment.
//!   - The leader (`ClientProc(0)`) acts only as a client to workers `1..=W` (`W` connections =
//!     `2 * W` SPSC rings).
//!   - Each worker `w` (`1..=W`) acts as a server for `W` connections (`1` from leader `0` and
//!     `W - 1` from peer workers `w2 != w`) and as a client to the `W - 1` peer workers (`W * (W - 1)`
//!     connections). Same-worker stage reads short-circuit in memory via `datafusion-distributed`'s
//!     `LocalWorkerContext` (`InProcessWorkerChannel`).
//!   - Total across `1` leader and `W` workers: `W^2` duplex connections = `2 * W^2` SPSC byte
//!     rings (`~8 MiB` at `W = 8`).
//! - Wakeup pool ([`wakeup`]):
//!   - `_PG_init` pre-allocates `MaxBackends` non-blocking `AF_UNIX` `socketpair`s registered with
//!     `pg_sys::ReserveExternalFD()`, indexed by `MyProcNumber`.
//!   - Each participating process runs a [`wakeup::WakeupDispatcher`] on its local Tokio runtime
//!     watching `pool[MyProcNumber].read_fd` via `tokio::io::unix::AsyncFd`, draining wakeup bytes
//!     and waking per-ring local `AtomicWaker`s.
//!   - When a peer advances `write_pos` or `read_pos` and observes the peer's `sleeping` flag set
//!     (`SeqCst` `swap(false)`), it writes 1 byte to `pool[peer_slot].write_fd`.
//! - PostgreSQL reactor guards ([`interrupt`], [`wakeup`]):
//!   - [`interrupt::HeldInterrupts`] holds `HOLD_INTERRUPTS` across `runtime.block_on(...)` on both
//!     leader and workers so PostgreSQL signal handlers cannot `siglongjmp` across Tokio frames;
//!     [`interrupt::cancel_pending`] is polled inside `DsmAsyncStream` and
//!     `DsmWorkerShutdownWatcher` so queries abort cleanly before `RESUME_INTERRUPTS` +
//!     `check_for_interrupts!()`.
//!   - `postmaster_alive_fds[POSTMASTER_FD_WATCH]` is monitored via a non-closing `AsyncFd` wrapper
//!     so workers abort immediately if `postmaster` dies.
//!
//! ## 3. Separation of PostgreSQL Worker Lifecycle from Per-Query gRPC Execution
//!
//! - Pre-launch ([`launch`]): walks `DistributedExec` to compute
//!   `producer_count = clamp(max_tasks, 2, cap)`, validates scan nodes, allocates the DSM segment
//!   containing `ParallelScanState` and the `2 * W^2` SPSC byte rings, and launches PostgreSQL
//!   parallel workers.
//! - Startup & execution ([`glue`], [`worker`]): workers publish their `MyProcNumber` wakeup slot,
//!   synchronize on the startup barrier, and run `serve_with_incoming_shutdown`. The leader executes
//!   `DistributedExec`; `QueryCoordinator` dispatches stage plans over `coordinator_channel`, leader
//!   and workers stream batches over `execute_task`, and `c2w_stream` EOF triggers
//!   `Worker::coordinator_channel` to send `TaskMetrics` and `TaskCompletedDynamicFilters` back to
//!   the leader's `DistributedExec::metrics_store`.
//! - Teardown ([`glue::finish_leader_execution`]): on `EXPLAIN ANALYZE`, the leader awaits
//!   `DistributedExec::wait_for_metrics()`; then it detaches its client DSM streams, sets the DSM
//!   `shutdown_flag`, and pokes every worker's wakeup FD so `serve_with_incoming_shutdown` exits
//!   before `WaitForParallelWorkersToFinish`.
//!
//! ## 4. Worker Session & Plan Rewrite Hooks ([`worker`])
//!
//! - `MppWorkerSessionBuilder`: installs the worker's peer `DsmGrpcChannelResolver`, registers
//!   `pg_search` UDFs/UDAFs, attaches `PgSearchPhysicalExtensionCodec::new_worker(...)`, and
//!   installs a `DelegatingMemoryPool` that starts unbounded during plan decode.
//! - `MppWorkerPlanRewriteHandler`: validates `visit_scan_nodes`, tightens `DelegatingMemoryPool`
//!   to a `WorkMemMemoryPool` sized for the decoded stage plan's memory-intensive operators, and
//!   wraps the stage root in `CatchUnwindExec` so any PostgreSQL `ereport(ERROR)` (`CaughtError`
//!   panic) inside `poll_next` is converted into a `DataFusionError::Execution` over gRPC.

pub mod dsm_stream;
pub mod glue;
pub mod interrupt;
pub mod launch;
pub mod wakeup;
pub mod worker;

pub(crate) use interrupt::HeldInterrupts;
pub use worker::panic_payload_message;

/// Emit a runtime trace when `paradedb.mpp_debug` is on.
///
/// Routed through `pgrx::warning!` so the line lands in the Postgres server log (and CI bench
/// logs).
#[macro_export]
macro_rules! mpp_log {
    ($($arg:tt)*) => {
        if $crate::gucs::mpp_debug() {
            pgrx::warning!($($arg)*);
        }
    };
}

/// Fatal MPP-internal invariant breach. Same `!` return type as `pgrx::error!` / `panic!`, so it
/// can sit in any `match` arm or expression position.
pub fn fail_loud(msg: String) -> ! {
    pgrx::error!("{}", msg);
}

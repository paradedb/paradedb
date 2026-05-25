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

//! Backend-thread FFI relay for MPP compute futures.
//!
//! pgrx 0.18 pins every Postgres FFI call to one OS thread per backend
//! process. Today the worker runs every fragment future on a current-thread
//! tokio runtime pinned to that backend thread, so `shm_mq_send` (and any
//! other PG API) is safe to call directly from compute code. The G7-MT
//! follow-up wants to switch producers to a multi-thread runtime so each
//! producer can use more than one core; that breaks the direct calls.
//!
//! [`FfiRelay`] is the bridge. Compute futures hand operations to it; a
//! [`FfiRelayService`] task running on the backend thread (under a Tokio
//! [`tokio::task::LocalSet`]) pulls them off the channel and replays them
//! on the backend. The response comes back via a [`tokio::sync::oneshot`]
//! so the compute future stays correctly suspended until the FFI call
//! returns.
//!
//! The scaffolding is in place today but dormant: nothing in the production
//! send path routes through it yet. Phase 3 (the runtime flip) wires
//! [`MppSender`] to use the relay when [`FfiRelay::is_attached`].
//!
//! Lifetime: the service task lives for the duration of `run_mpp_worker`;
//! drop the [`FfiRelay`] handle to signal shutdown. The service will drain
//! any in-flight ops, reply to them, and exit when the channel closes.

use std::sync::Arc;

use datafusion::common::DataFusionError;
use tokio::sync::{mpsc, oneshot};

use crate::postgres::customscan::mpp::transport::BatchChannelSender;

/// FFI operations that compute futures may need to perform via the backend thread.
///
/// Add a new variant per Postgres API that the multi-thread compute path needs to call.
/// Keep payloads owned (no borrows) so they cross thread boundaries cleanly.
#[allow(dead_code)] // wired up in G7-MT Phase 3 (the runtime flip).
pub(crate) enum FfiOp {
    /// Non-blocking shm_mq send. Returns `Ok(true)` on success, `Ok(false)` if the queue
    /// is full (caller spins + retries), `Err` on detach or unknown PG error.
    ShmMqTrySend {
        channel: Arc<dyn BatchChannelSender>,
        bytes: Vec<u8>,
        response: oneshot::Sender<Result<bool, DataFusionError>>,
    },
}

/// Producer-side handle. Cheap to clone; shares one mpsc sender with the service task.
#[allow(dead_code)] // wired up in G7-MT Phase 3 (the runtime flip).
#[derive(Clone)]
pub(crate) struct FfiRelay {
    tx: mpsc::UnboundedSender<FfiOp>,
}

#[allow(dead_code)] // wired up in G7-MT Phase 3 (the runtime flip).
impl FfiRelay {
    /// Construct a relay paired with the service that drives it.
    ///
    /// The service must be spawned on the backend thread before any compute future calls
    /// into the relay; see [`FfiRelayService::run`] for the polling loop.
    pub fn new() -> (Self, FfiRelayService) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { tx }, FfiRelayService { rx })
    }

    /// Forward a non-blocking shm_mq send to the backend thread and await its result.
    ///
    /// Takes ownership of `bytes` because the buffer has to outlive the await point on
    /// the compute side without holding a borrow into the caller's scratch.
    ///
    /// G7-MT phase 3b perf TODO: the spin loop calls into this on every queue-full retry,
    /// which at 25M-row shuffles can be tens of millions of round-trips. Each round-trip
    /// allocates a fresh `tokio::sync::oneshot` here. Before phase 3b lands, switch to
    /// either a per-sender preallocated reply slot (e.g. `Arc<Notify> + AtomicResult`) or
    /// a batched op variant (`ShmMqTrySendMany { slices: Vec<Vec<u8>>, response: oneshot<Vec<...>> }`).
    /// Measure on the bench before committing — naive oneshot allocation could eat the
    /// multi-thread win.
    pub async fn shm_mq_try_send(
        &self,
        channel: Arc<dyn BatchChannelSender>,
        bytes: Vec<u8>,
    ) -> Result<bool, DataFusionError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(FfiOp::ShmMqTrySend {
                channel,
                bytes,
                response: resp_tx,
            })
            .map_err(|_| DataFusionError::Execution("ffi_relay: service task closed".into()))?;
        match resp_rx.await {
            Ok(result) => result,
            Err(_) => Err(DataFusionError::Execution(
                "ffi_relay: service dropped the response without replying".into(),
            )),
        }
    }
}

/// Service-side handle. Owns the receiver; consumed by [`FfiRelayService::run`].
#[allow(dead_code)] // wired up in G7-MT Phase 3 (the runtime flip).
pub(crate) struct FfiRelayService {
    rx: mpsc::UnboundedReceiver<FfiOp>,
}

#[allow(dead_code)] // wired up in G7-MT Phase 3 (the runtime flip).
impl FfiRelayService {
    /// Polling loop. Pulls ops off the channel and replays them on the current OS thread,
    /// which the caller is responsible for being the backend thread (the one holding
    /// `PGPROC`).
    ///
    /// # LocalSet pinning under multi_thread Tokio
    ///
    /// The intended call shape **today** (current_thread runtime, single-thread world):
    ///
    /// ```ignore
    /// let local = tokio::task::LocalSet::new();
    /// local.spawn_local(service.run());
    /// runtime.block_on(local.run_until(compute_future));
    /// ```
    ///
    /// Works because `runtime.block_on` is a blocking call on the caller's thread, so
    /// `LocalSet::run_until` pins to that same (backend) thread.
    ///
    /// **Under a multi_thread runtime this pattern is wrong.** `runtime.block_on(...)`
    /// parks the caller and the future runs on a worker thread; `LocalSet::run_until`
    /// would then pin the service to whichever worker woke first, *not* the backend.
    /// The relay would round-trip via a non-backend Tokio worker — same panic the relay
    /// was supposed to prevent.
    ///
    /// Phase 3d must restructure this. Two viable shapes:
    ///
    /// 1. Keep a dedicated `current_thread` driver runtime for the FFI service + LocalSet
    ///    on the backend, and a separate `multi_thread` runtime for compute. Compute
    ///    futures hold an `FfiRelay` handle and call into it from any worker thread; the
    ///    relay's mpsc crosses thread boundaries.
    /// 2. Use `LocalRuntime` (tokio-unstable) with `local_set.block_on(&runtime, future)`
    ///    so the LocalSet drives the runtime on the backend thread directly. Not stable yet.
    ///
    /// Option 1 is what the design intends. The `runtime: &Runtime` parameter shape of
    /// `run_mpp_worker` hides this distinction today; tighten it before the flip.
    ///
    /// Returns when the channel closes (every [`FfiRelay`] handle has been dropped).
    pub async fn run(mut self) {
        while let Some(op) = self.rx.recv().await {
            match op {
                FfiOp::ShmMqTrySend {
                    channel,
                    bytes,
                    response,
                } => {
                    let result = channel.try_send_bytes(&bytes);
                    // The compute future may have been cancelled before our reply landed.
                    // Drop the result silently in that case — the response channel's
                    // close is the signal.
                    let _ = response.send(result);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::customscan::mpp::transport::{in_proc_channel, RecvOutcome};

    /// End-to-end: a producer task sends ops through `FfiRelay`, the service runs them
    /// through a real `BatchChannelSender` (in-proc channel here, shm_mq in production),
    /// and the bytes the receiver pulls off the channel match what the producer sent.
    /// Exercises the actual `FfiRelay` / `FfiRelayService` types so a future refactor
    /// of either catches breakage instead of slipping past a stub.
    #[tokio::test(flavor = "current_thread")]
    async fn relay_round_trip_through_real_channel() {
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                let (in_proc_tx, in_proc_rx) = in_proc_channel(8);
                let channel: Arc<dyn BatchChannelSender> = Arc::new(in_proc_tx);

                let (relay, service) = FfiRelay::new();
                let service_handle = tokio::task::spawn_local(service.run());

                assert!(
                    relay
                        .shm_mq_try_send(Arc::clone(&channel), vec![1, 2, 3])
                        .await
                        .expect("first send"),
                    "in-proc channel has capacity, send should succeed"
                );
                assert!(relay
                    .shm_mq_try_send(Arc::clone(&channel), vec![4, 5, 6])
                    .await
                    .expect("second send"),);

                // Receiver sees both payloads in order.
                use crate::postgres::customscan::mpp::transport::BatchChannelReceiver;
                let outcomes = [in_proc_rx.try_recv(), in_proc_rx.try_recv()];
                let bytes: Vec<Vec<u8>> = outcomes
                    .into_iter()
                    .map(|o| match o {
                        RecvOutcome::Bytes(b) => b.to_vec(),
                        other => panic!("expected Bytes, got {other:?}"),
                    })
                    .collect();
                assert_eq!(bytes[0], vec![1, 2, 3]);
                assert_eq!(bytes[1], vec![4, 5, 6]);

                // Drop the relay's producer side so the service observes channel close + exits.
                drop(relay);
                service_handle.await.unwrap();
            })
            .await;
    }
}

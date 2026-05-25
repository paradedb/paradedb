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

use crate::postgres::customscan::mpp::mesh::ShmMqSender;
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
        sender: Arc<ShmMqSender>,
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
    pub async fn shm_mq_try_send(
        &self,
        sender: Arc<ShmMqSender>,
        bytes: Vec<u8>,
    ) -> Result<bool, DataFusionError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(FfiOp::ShmMqTrySend {
                sender,
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
    /// The intended call shape under multi-thread Tokio is:
    ///
    /// ```ignore
    /// let local = tokio::task::LocalSet::new();
    /// local.spawn_local(service.run());
    /// runtime.block_on(local.run_until(compute_future));
    /// ```
    ///
    /// `LocalSet` pins the service to whatever thread enters `run_until`, which by
    /// construction is the backend. Compute futures spawned via [`tokio::task::spawn`]
    /// run on the multi-thread pool and call into the [`FfiRelay`] handle.
    ///
    /// Returns when the channel closes (every [`FfiRelay`] handle has been dropped).
    pub async fn run(mut self) {
        while let Some(op) = self.rx.recv().await {
            match op {
                FfiOp::ShmMqTrySend {
                    sender,
                    bytes,
                    response,
                } => {
                    let result = sender.try_send_bytes(&bytes);
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
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A throwaway op variant the test injects directly into the service receiver, so we
    /// don't need to build a real [`ShmMqSender`] to exercise the channel + LocalSet
    /// plumbing. Validates: the producer side sends an op + awaits, the service pulls
    /// it off the channel on the LocalSet's thread, replies via the oneshot, and the
    /// producer wakes up with the result.
    #[tokio::test(flavor = "current_thread")]
    async fn relay_round_trip_resolves_on_oneshot_reply() {
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                let (tx, mut rx) = mpsc::unbounded_channel::<oneshot::Sender<u32>>();

                let counter = Arc::new(AtomicUsize::new(0));
                let counter_for_service = Arc::clone(&counter);
                let service = tokio::task::spawn_local(async move {
                    while let Some(resp) = rx.recv().await {
                        counter_for_service.fetch_add(1, Ordering::SeqCst);
                        let _ = resp.send(42);
                    }
                });

                let (r1_tx, r1_rx) = oneshot::channel();
                tx.send(r1_tx).unwrap();
                assert_eq!(r1_rx.await.unwrap(), 42);

                let (r2_tx, r2_rx) = oneshot::channel();
                tx.send(r2_tx).unwrap();
                assert_eq!(r2_rx.await.unwrap(), 42);

                drop(tx);
                service.await.unwrap();
                assert_eq!(counter.load(Ordering::SeqCst), 2);
            })
            .await;
    }
}

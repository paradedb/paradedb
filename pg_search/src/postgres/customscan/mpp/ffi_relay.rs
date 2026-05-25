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

use crate::postgres::customscan::mpp::transport::{BatchChannelSender, CooperativeDrainSet};

/// FFI operations that compute futures may need to perform via the backend thread.
///
/// Add a new variant per Postgres API that the multi-thread compute path needs to call.
/// Keep payloads owned (no borrows) so they cross thread boundaries cleanly.
pub(crate) enum FfiOp {
    /// Non-blocking shm_mq send. Returns `Ok(true)` on success, `Ok(false)` if the queue
    /// is full (caller spins + retries), `Err` on detach or unknown PG error.
    ///
    /// The response carries the byte buffer back so the producer can reuse its encode
    /// scratch instead of allocating a fresh `Vec<u8>` on every spin attempt. Eliminates
    /// the `scratch.to_vec()` per-iteration copy that dominated the +25% phase 3b
    /// overhead on shuffle-heavy queries.
    ShmMqTrySend {
        channel: Arc<dyn BatchChannelSender>,
        bytes: Vec<u8>,
        response: oneshot::Sender<(Result<bool, DataFusionError>, Vec<u8>)>,
    },
    /// Run one cooperative-drain pass. Pulls inbound peer frames off the local mesh so
    /// the producer's full-outbound stall has a chance to unblock. Calls
    /// `shm_mq_receive(nowait=true)` underneath, which is FFI and must run on the
    /// backend thread.
    TryDrainPass {
        drain: Arc<dyn CooperativeDrainSet>,
        response: oneshot::Sender<Result<(), DataFusionError>>,
    },
    /// Honor pending Postgres cancel / timeout / SIGINT. Calls
    /// `pgrx::check_for_interrupts!()` underneath.
    ///
    /// On no-interrupt the call is a few-ns flag check and the oneshot resolves with `()`.
    ///
    /// On cancel the macro expands to `siglongjmp` that unwinds to the nearest `PG_TRY`
    /// frame **on the current OS thread**. The service task is running under
    /// `local_set.run_until(...)` inside `runtime.block_on(...)` on the backend thread,
    /// so the `longjmp` skips out of `block_on`, out of `run_mpp_worker`, and pops up at
    /// whatever `PG_TRY` `exec_mpp_worker_impl`'s caller installed. The Tokio runtime,
    /// the LocalSet, every in-flight oneshot, and every compute future are abandoned;
    /// PG's resource owner reclaims them on the way out. This is the same teardown shape
    /// as the direct (non-relay) interrupt-check path — query aborts, no per-future
    /// orderly error.
    CheckForInterrupts { response: oneshot::Sender<()> },
}

/// Producer-side handle. Cheap to clone; shares one mpsc sender with the service task.
#[derive(Clone)]
pub(crate) struct FfiRelay {
    tx: mpsc::UnboundedSender<FfiOp>,
}

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
    /// the compute side without holding a borrow into the caller's scratch. The service
    /// returns the buffer back through the tuple so the producer can reuse it without
    /// allocating a fresh `Vec<u8>` on every spin attempt.
    pub async fn shm_mq_try_send(
        &self,
        channel: Arc<dyn BatchChannelSender>,
        bytes: Vec<u8>,
    ) -> (Result<bool, DataFusionError>, Vec<u8>) {
        let (resp_tx, resp_rx) = oneshot::channel();
        let op = FfiOp::ShmMqTrySend {
            channel,
            bytes,
            response: resp_tx,
        };
        if let Err(send_err) = self.tx.send(op) {
            // Recover the bytes from the rejected op so the caller can keep its scratch
            // buffer intact across a service-shutdown error.
            let recovered = match send_err.0 {
                FfiOp::ShmMqTrySend { bytes, .. } => bytes,
                _ => Vec::new(),
            };
            return (
                Err(DataFusionError::Execution(
                    "ffi_relay: service task closed".into(),
                )),
                recovered,
            );
        }
        match resp_rx.await {
            Ok(pair) => pair,
            Err(_) => (
                Err(DataFusionError::Execution(
                    "ffi_relay: service dropped the response without replying".into(),
                )),
                Vec::new(),
            ),
        }
    }

    /// Forward a cooperative-drain pass to the backend thread and await its result.
    /// The drain pulls inbound peer frames so a full-outbound spin in the same producer
    /// can make progress.
    pub async fn try_drain_pass(
        &self,
        drain: Arc<dyn CooperativeDrainSet>,
    ) -> Result<(), DataFusionError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(FfiOp::TryDrainPass {
                drain,
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

    /// Forward a `pgrx::check_for_interrupts!()` call to the backend thread.
    ///
    /// On no-interrupt the oneshot resolves with `()` after a quick flag check.
    /// On cancel the pgrx macro `siglongjmp`s out of the service task → out of `block_on`
    /// → out of `run_mpp_worker`, tearing down the runtime, the LocalSet, the in-flight
    /// oneshots, and every compute future. The caller's `.await` never wakes; the query
    /// abort is surfaced by PG's normal cancel teardown path. See the doc on
    /// [`FfiOp::CheckForInterrupts`] for the full unwinding shape.
    pub async fn check_for_interrupts(&self) -> Result<(), DataFusionError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(FfiOp::CheckForInterrupts { response: resp_tx })
            .map_err(|_| DataFusionError::Execution("ffi_relay: service task closed".into()))?;
        match resp_rx.await {
            Ok(()) => Ok(()),
            Err(_) => Err(DataFusionError::Execution(
                "ffi_relay: service dropped the interrupt check (likely cancelled)".into(),
            )),
        }
    }
}

/// Service-side handle. Owns the receiver; consumed by [`FfiRelayService::run`].
pub(crate) struct FfiRelayService {
    rx: mpsc::UnboundedReceiver<FfiOp>,
}

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
                    // Return the buffer so the producer reuses it on the next attempt
                    // instead of allocating fresh. The compute future may have been
                    // cancelled before our reply landed — drop the result silently in
                    // that case, the response channel's close is the signal.
                    let _ = response.send((result, bytes));
                }
                FfiOp::TryDrainPass { drain, response } => {
                    let result = drain.try_drain_pass();
                    let _ = response.send(result);
                }
                FfiOp::CheckForInterrupts { response } => {
                    // pgrx::check_for_interrupts! is gated out of test builds because the
                    // macro pulls in PG symbols the lib-test binary doesn't link.
                    #[cfg(not(test))]
                    pgrx::check_for_interrupts!();
                    let _ = response.send(());
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

                let (r1, returned1) = relay
                    .shm_mq_try_send(Arc::clone(&channel), vec![1, 2, 3])
                    .await;
                assert!(
                    r1.expect("first send"),
                    "in-proc channel has capacity, send should succeed"
                );
                // The relay returns the byte buffer so the producer can reuse it; the test
                // doesn't reuse here, just asserts the round-trip preserves the contents.
                assert_eq!(returned1, vec![1, 2, 3]);

                let (r2, returned2) = relay
                    .shm_mq_try_send(Arc::clone(&channel), vec![4, 5, 6])
                    .await;
                assert!(r2.expect("second send"));
                assert_eq!(returned2, vec![4, 5, 6]);

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

    /// A `CooperativeDrainSet` stub the test installs to count drain-pass calls.
    struct CountingDrain {
        passes: std::sync::atomic::AtomicUsize,
    }

    impl CooperativeDrainSet for CountingDrain {
        fn try_drain_pass(&self) -> Result<(), DataFusionError> {
            self.passes
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    /// Verify `TryDrainPass` + `CheckForInterrupts` ops both round-trip through the relay.
    /// On the production path the spin loop calls these between `try_send_bytes` retries
    /// when the outbound queue is full; today's tests never hit a full queue so coverage
    /// in transport.rs doesn't exercise either op.
    #[tokio::test(flavor = "current_thread")]
    async fn relay_routes_drain_and_interrupt_ops() {
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                let (relay, service) = FfiRelay::new();
                let service_handle = tokio::task::spawn_local(service.run());

                let drain = Arc::new(CountingDrain {
                    passes: std::sync::atomic::AtomicUsize::new(0),
                });
                let drain_handle: Arc<dyn CooperativeDrainSet> = Arc::clone(&drain) as _;

                for _ in 0..3 {
                    relay
                        .try_drain_pass(Arc::clone(&drain_handle))
                        .await
                        .expect("drain pass");
                    relay.check_for_interrupts().await.expect("interrupt check");
                }
                assert_eq!(drain.passes.load(std::sync::atomic::Ordering::SeqCst), 3);

                drop(relay);
                service_handle.await.unwrap();
            })
            .await;
    }

    /// Higher-fidelity drain test: build a real `DrainHandle::cooperative` over an in-proc
    /// channel, attach it through the relay, and verify the drain pass acquires the inner
    /// mutexes + returns `Ok` without contention. Catches a regression where the inner
    /// `std::sync::Mutex` is swapped for a `tokio::sync::Mutex` and breaks the
    /// cross-thread invariant the relay relies on.
    #[tokio::test(flavor = "current_thread")]
    async fn relay_routes_drain_pass_through_real_drain_handle() {
        use crate::postgres::customscan::mpp::transport::{
            in_proc_channel, DrainHandle, MppReceiver,
        };
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                let (_sender_kept_alive, receiver) = in_proc_channel(4);
                let drain_handle: Arc<dyn CooperativeDrainSet> = Arc::new(
                    DrainHandle::cooperative(vec![MppReceiver::new(Box::new(receiver))]),
                );

                let (relay, service) = FfiRelay::new();
                let service_handle = tokio::task::spawn_local(service.run());

                // Two drain passes over an empty receiver: both should return Ok(()) and
                // not panic on the mutex locking inside `DrainHandle::try_drain_pass`.
                relay
                    .try_drain_pass(Arc::clone(&drain_handle))
                    .await
                    .expect("first drain pass");
                relay
                    .try_drain_pass(Arc::clone(&drain_handle))
                    .await
                    .expect("second drain pass");

                drop(relay);
                service_handle.await.unwrap();
            })
            .await;
    }
}

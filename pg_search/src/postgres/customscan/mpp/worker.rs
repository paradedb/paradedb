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

//! MPP worker fragment runner.
//!
//! "Worker" matches the DF-D fork's terminology. Every distributed task is a `WorkerConnection`
//! on the receive side, and the fragment runner is that worker's push side.
//!
//! - [`run_worker_fragment`]: PG-parallel-worker push loop. Runs the `n_partitions` output
//!   partitions of `plan` concurrently; each batch yielded by partition `p` is encoded and pushed
//!   through `outbound_senders[p]`. Returns when every output stream is exhausted. Only producer
//!   workers call this; the leader is consumer-only (see
//!   [`crate::postgres::customscan::mpp::glue::producer_worker_count`]).

use std::sync::Arc;
use std::time::Instant;

use datafusion::common::{DataFusionError, Result};
use datafusion::execution::TaskContext;
use datafusion::physical_plan::{ExecutionPlan, ExecutionPlanProperties};
use futures::stream::StreamExt;

use crate::postgres::customscan::mpp::runtime_gucs::runtime_gucs_from_ctx;
use crate::postgres::customscan::mpp::transport::{MppSender, SendBatchStats};

/// Read this thread's accumulated CPU time in nanoseconds, via
/// `CLOCK_THREAD_CPUTIME_ID`. Available on Linux and macOS 10.12+.
/// Returns 0 if the clock isn't readable — diagnostics shouldn't propagate.
fn thread_cpu_ns() -> u64 {
    let mut ts: libc::timespec = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut ts) };
    if rc != 0 {
        return 0;
    }
    (ts.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(ts.tv_nsec as u64)
}

/// Run a producer fragment plan to exhaustion and push every output batch
/// through the corresponding `outbound_senders[partition]`.
///
/// The output partition count of `plan` MUST equal `outbound_senders.len()`;
/// this is checked before the first batch is pulled.
pub async fn run_worker_fragment(
    plan: Arc<dyn ExecutionPlan>,
    outbound_senders: Vec<MppSender>,
    ctx: Arc<TaskContext>,
) -> Result<()> {
    let n_partitions = plan.output_partitioning().partition_count();
    if n_partitions != outbound_senders.len() {
        return Err(DataFusionError::Internal(format!(
            "run_worker_fragment: plan has {} output partitions but {} senders provided",
            n_partitions,
            outbound_senders.len()
        )));
    }

    // `paradedb.mpp_trace` is pulled from the per-query GUC snapshot installed by
    // `build_mpp_session_context`, not from the live pgrx GUC, so this stays safe once the
    // tokio runtime goes multi-thread. Fall back to the live read when there's no snapshot
    // (the non-MPP serial path that shares this module).
    let trace_on = runtime_gucs_from_ctx(&ctx)
        .map(|g| g.mpp_trace)
        .unwrap_or_else(crate::gucs::mpp_trace);

    // Execute every output partition concurrently. Each partition gets its
    // own sender; pushes are independent. `Arc<MppSender>` so each
    // partition's future has its own clone (MppSender is `Sync`).
    let senders: Vec<Arc<MppSender>> = outbound_senders.into_iter().map(Arc::new).collect();
    let mut futures = Vec::with_capacity(n_partitions);
    for (partition, sender) in senders.iter().enumerate() {
        let plan = Arc::clone(&plan);
        let ctx = Arc::clone(&ctx);
        let sender = Arc::clone(sender);
        // Each partition must send a per-channel EOF when the stream ends, regardless of how it
        // ended (success or error). The shm_mq queue is shared across fragments, so dropping the
        // sender doesn't signal end-of-channel — only the EOF frame does. A `Drop`-based guard
        // would be cleaner in principle, but `send_eof_traced` is async and runs through the
        // cooperative-drain spin which must execute on the backend thread, so we sequence it
        // explicitly here.
        futures.push(async move {
            let mut stats = SendBatchStats::default();
            let wall_start = trace_on.then(Instant::now);
            let mut first_batch_ms: f64 = 0.0;
            let mut pull_ns: u64 = 0;
            let mut send_ns: u64 = 0;
            let mut rows_in: u64 = 0;
            let mut batches: u64 = 0;
            let stream_result: Result<(), DataFusionError> = async {
                let mut stream = plan.execute(partition, ctx)?;
                let mut t = Instant::now();
                loop {
                    let next = stream.next().await;
                    if trace_on {
                        let dt = t.elapsed();
                        pull_ns = pull_ns.saturating_add(dt.as_nanos() as u64);
                        if batches == 0 && next.is_some() {
                            first_batch_ms = wall_start.unwrap().elapsed().as_secs_f64() * 1000.0;
                        }
                    }
                    let Some(batch) = next else { break };
                    let batch = batch?;
                    if batch.num_rows() == 0 {
                        if trace_on { t = Instant::now(); }
                        continue;
                    }
                    rows_in += batch.num_rows() as u64;
                    batches += 1;
                    let t_send = trace_on.then(Instant::now);
                    sender
                        .as_ref()
                        .send_batch_traced(&batch, &mut stats)
                        .await?;
                    if let Some(t_send) = t_send {
                        send_ns = send_ns.saturating_add(t_send.elapsed().as_nanos() as u64);
                    }
                    if trace_on { t = Instant::now(); }
                }
                Ok(())
            }
            .await;
            let eof_result = sender.as_ref().send_eof_traced(&mut stats).await;
            if let Some(wall_start) = wall_start {
                let header = sender.as_ref().header;
                pgrx::warning!(
                    "mpp_trace stage={} part={} rows_in={} batches={} wall_ms={:.1} first_batch_ms={:.1} pull_ms={:.1} send_ms={:.1} encode_ms={:.1} send_wait_ms={:.1}",
                    header.stage_id,
                    header.partition,
                    rows_in,
                    batches,
                    wall_start.elapsed().as_secs_f64() * 1000.0,
                    first_batch_ms,
                    pull_ns as f64 / 1.0e6,
                    send_ns as f64 / 1.0e6,
                    stats.encode.as_secs_f64() * 1000.0,
                    stats.send_wait.as_secs_f64() * 1000.0,
                );
            }
            // Stream error first, then any EOF-send error so neither failure mode disappears.
            stream_result.and(eof_result)
        });
    }
    // `join_all`, not `try_join_all`: fail-fast would cancel sibling partitions mid-await before
    // they hit `send_eof_traced`, leaving the consumer's channel buffer stuck and the leader's
    // `select_all` blocked.
    //
    // Fragment-level CPU saturation: with a current-thread tokio runtime, all partition futures
    // share this thread. Sampling CPU around the whole join_all answers "did this PG worker's
    // thread max out a core during the fragment, or was it blocked on shm_mq / upstream?" — a
    // ~100% reading means adding cores per producer (G7-MT) would unlock parallelism; a low
    // reading means shuffle or scan blocking is the binding constraint.
    let frag_wall_start = trace_on.then(Instant::now);
    let frag_cpu_start = trace_on.then(thread_cpu_ns);
    let results = futures::future::join_all(futures).await;
    if trace_on {
        let wall_ms = frag_wall_start.unwrap().elapsed().as_secs_f64() * 1000.0;
        let cpu_ms = thread_cpu_ns().saturating_sub(frag_cpu_start.unwrap()) as f64 / 1.0e6;
        let cpu_pct = if wall_ms > 0.0 {
            cpu_ms / wall_ms * 100.0
        } else {
            0.0
        };
        pgrx::warning!(
            "mpp_trace fragment_cpu partitions={} wall_ms={:.2} cpu_ms={:.2} cpu_pct={:.1}",
            n_partitions,
            wall_ms,
            cpu_ms,
            cpu_pct,
        );
    }
    drop(senders);
    for r in results {
        r?;
    }
    Ok(())
}

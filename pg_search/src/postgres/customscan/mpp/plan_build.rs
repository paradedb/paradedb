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

#![allow(dead_code)]
//! MPP physical-plan primitives.
//!
//! # What MPP actually does, and where this module fits
//!
//! The milestone-1 benchmark is `SELECT COUNT(*) FROM f JOIN p ON f.id = p.fileId
//! WHERE f.content ||| 'Section'` — a scalar aggregate over a join. The MPP win
//! for that query comes from **hash-partitioning the join inputs**, not from
//! partitioning the aggregate output. Per worker, the physical plan looks like:
//!
//! ```text
//!     AggregateExec(Partial, COUNT(*))            ← emits one partial row per worker
//!       HashJoinExec(PartitionMode::Partitioned)
//!         wrap_with_mpp_shuffle([Scan f → Filter], hash=[f.id])
//!         wrap_with_mpp_shuffle([Scan p         ], hash=[p.fileId])
//! ```
//!
//! PG's `Gather` concatenates all participants' Partial outputs and PG's
//! `Finalize Aggregate` sums them into the final COUNT. Our CustomScan produces
//! partial rows; Postgres handles the finalization above us.
//!
//! # What this module provides
//!
//! [`wrap_with_mpp_shuffle`] is the single reusable primitive that wraps any
//! `ExecutionPlan` with the MPP mesh topology:
//!
//! ```text
//!     inner ──── ShuffleExec (hash, self-partition out) ─┐
//!                                                         ├→ UnionExec
//!                          DrainGatherExec (peer rows) ───┘
//! ```
//!
//! The output stream emits exactly the rows of `inner` that hash to this
//! participant — some from local computation, some from peers —
//! merged into a single Arrow IPC-compatible `SendableRecordBatchStream`.
//!
//! Callers (AggregateScan, JoinScan MPP) compose `HashJoinExec`,
//! `AggregateExec(Partial)`, or any other DataFusion operator on top of the
//! wrapped child. Because each wrap consumes one directed mesh of shm_mqs
//! (one `ShuffleWiring` + one `DrainHandle`), a binary hash-partitioned join
//! needs **two** independent mesh wirings — one per side. The DSM hook layer
//! allocates and distributes them; this module is ABI-agnostic about where
//! they come from.
//!
//! # What was here before
//!
//! An earlier version of this module shipped `build_mpp_aggregate_plan` that
//! composed `Partial → Shuffle → Union(DrainGather) → FinalPartitioned`. That
//! topology is structurally correct for *hash-partitioned GROUP BY* aggregates
//! where each final group lives on exactly one participant — but it is
//! **incorrect for scalar aggregates** (our benchmark target). The final
//! `FinalPartitioned` on a participant whose inbound side is empty emits a
//! spurious all-NULL row; PG's Gather concatenates that into the output and
//! breaks the scalar contract. The broken helper is replaced rather than
//! patched: pre-join-shuffle of raw rows (this module) is the correct
//! building block for the benchmark and cleanly composes with DataFusion's
//! existing `HashJoinExec(Partitioned)`.

use std::sync::Arc;

use datafusion::arrow::datatypes::SchemaRef;
use datafusion::common::{DataFusionError, Result};
use datafusion::physical_expr::PhysicalExpr;
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::{ExecutionPlan, ExecutionPlanProperties};

use crate::postgres::customscan::mpp::shuffle::{MppShuffleExec, ShuffleWiring};
use crate::postgres::customscan::mpp::stage::{MppNetworkBoundary, MppStage};
use crate::postgres::customscan::mpp::transport::DrainHandle;

/// Inputs to [`wrap_with_mpp_shuffle`].
///
/// - `child`: the plan whose output rows the shuffle will consume.
/// - `wiring`: outbound half of this participant's mesh edge.
/// - `drain_handle`: inbound half (peer-shipped rows).
/// - `wrapped_schema`: schema both `child.schema()` and peer-sent Arrow IPC
///   batches resolve to. Used to coalesce a multi-partition child if needed.
/// - `tag`: diagnostic label used in `mpp_log!` row-count reports.
/// - `stage`: stage descriptor stamped onto the emitted [`MppShuffleExec`].
/// - `hash_keys`: when `Some(keys)`, the shuffle declares
///   `Partitioning::Hash(keys, N)` natively. When `None`, output is
///   `UnknownPartitioning(1)` (the scalar-final gather case).
/// - `drive_partition`: the output partition that performs the producer +
///   drain work on this participant. For hash shuffles this is
///   `participant_index`; for fixed-target gathers it is the target on every
///   participant.
pub struct MppShuffleInputs {
    pub child: Arc<dyn ExecutionPlan>,
    pub wiring: ShuffleWiring,
    pub drain_handle: Arc<DrainHandle>,
    pub wrapped_schema: SchemaRef,
    pub tag: &'static str,
    pub stage: Option<MppStage>,
    pub hash_keys: Option<Vec<Arc<dyn PhysicalExpr>>>,
    pub drive_partition: u32,
}

/// Wrap a child plan with the MPP hash-shuffle topology — a single
/// [`MppShuffleExec`] that
///
///   * declares `Partitioning::Hash(keys, N)` natively when `hash_keys` is
///     provided (or `UnknownPartitioning(1)` for the scalar-final gather),
///   * consumes both halves of the mesh edge (`wiring` + `drain_handle`)
///     internally, merging the producer-side stream (drives upstream,
///     hash-routes, ships to peers) with the consumer-side stream (drains
///     peer-shipped rows from `shm_mq`) on the participant's
///     `drive_partition`,
///   * returns an empty stream for every other partition.
///
/// Replaces the older
/// `MppPartitionAdapterExec(CoalescePartitionsExec(UnionExec(ShuffleExec,
/// DrainGatherExec)))` four-node topology — same end-to-end semantics, one
/// operator instead of four. Shape-aligned with
/// `datafusion-distributed::NetworkShuffleExec`.
///
/// Both mesh halves are consumed on this single call; a two-input join needs
/// two independent calls — one per side — with independently-allocated
/// meshes.
pub fn wrap_with_mpp_shuffle(
    inputs: MppShuffleInputs,
) -> Result<Arc<dyn ExecutionPlan>, DataFusionError> {
    let MppShuffleInputs {
        child,
        wiring,
        drain_handle,
        wrapped_schema,
        tag,
        stage,
        hash_keys,
        drive_partition,
    } = inputs;
    let _ = wrapped_schema; // Schema is inferred from `child.schema()`.

    // Multi-partition children (e.g. a PgSearchTableProvider scan that emits
    // one partition per Tantivy segment) need to be coalesced first: the
    // shuffle's producer-side only consumes its child's partition 0, so
    // without this every segment beyond the first would be dropped silently,
    // producing correct group counts but wildly wrong per-group aggregates.
    let child: Arc<dyn ExecutionPlan> = if child.output_partitioning().partition_count() > 1 {
        Arc::new(CoalescePartitionsExec::new(child))
    } else {
        child
    };

    let shuffle_node =
        MppShuffleExec::new(child, wiring, drain_handle, hash_keys, drive_partition, tag);
    match stage {
        // Stamp via the `MppNetworkBoundary` seam. The trait method consumes
        // the wiring + drain_handle Mutex<Option<>>s and produces a fresh
        // `Arc`; the unstamped node is dropped immediately after.
        Some(s) => MppNetworkBoundary::with_input_stage(&shuffle_node, s),
        None => Ok(Arc::new(shuffle_node)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::customscan::mpp::shuffle::tests::ModuloPartitioner;
    use crate::postgres::customscan::mpp::transport::{
        in_proc_channel, DrainBuffer, DrainConfig, DrainItem, MppReceiver, MppSender,
    };
    use datafusion::arrow::array::{Int32Array, RecordBatch, StringArray};
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::datasource::memory::MemorySourceConfig;
    use datafusion::physical_plan::ExecutionPlanProperties;
    use datafusion::prelude::SessionContext;
    use futures::StreamExt;
    use std::thread;
    use std::time::Duration;

    fn sample_batch(rows: i32) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
        ]));
        let ids = Int32Array::from_iter_values(0..rows);
        let names = StringArray::from_iter_values((0..rows).map(|i| format!("n{i}")));
        RecordBatch::try_new(schema, vec![Arc::new(ids), Arc::new(names)]).unwrap()
    }

    /// Drive a two-participant mesh in-process: our side shuffles a 10-row
    /// input at participant 0 with `ModuloPartitioner(2)`, a simulated peer ships
    /// synthetic batches into our inbound drain. The wrapped stream should
    /// emit:
    ///
    /// - self-partition rows (even IDs: 0,2,4,6,8)
    /// - peer-partition rows (synthetic IDs 100,200 from the peer)
    ///
    /// and the outbound channel should carry the odd IDs that ShuffleExec
    /// shipped away (1,3,5,7,9).
    #[test]
    fn wrap_with_mpp_shuffle_splices_self_and_peer() {
        let batch = sample_batch(10);
        let schema = batch.schema();
        let input = MemorySourceConfig::try_new_from_batches(schema.clone(), vec![batch]).unwrap();

        let (out_tx, out_rx) = in_proc_channel(16);
        let (in_tx, in_rx) = in_proc_channel(16);

        let wiring = ShuffleWiring {
            partitioner: Arc::new(ModuloPartitioner::new(2)),
            outbound_senders: vec![None, Some(MppSender::new(Box::new(out_tx)))],
            participant_index: 0,
            cooperative_drain: None,
        };

        // Simulated peer: ship two synthetic batches with a small delay so
        // the drain thread actually waits on the waker path at least once.
        let peer_schema = schema.clone();
        let peer_thread = thread::spawn(move || {
            let sender = MppSender::new(Box::new(in_tx));
            for (id, name) in [(100i32, "peer100"), (200i32, "peer200")] {
                thread::sleep(Duration::from_millis(10));
                let b = RecordBatch::try_new(
                    peer_schema.clone(),
                    vec![
                        Arc::new(Int32Array::from_iter_values([id])),
                        Arc::new(StringArray::from_iter_values([name.to_string()])),
                    ],
                )
                .unwrap();
                sender.send_batch(&b).unwrap();
            }
        });

        let inbound_recv = MppReceiver::new(Box::new(in_rx));
        let drain_buffer = DrainBuffer::new(1);
        let drain_handle = DrainHandle::spawn(DrainConfig::new(
            vec![inbound_recv],
            Arc::clone(&drain_buffer),
        ));

        let wrapped = wrap_with_mpp_shuffle(MppShuffleInputs {
            child: input,
            wiring,
            drain_handle: Arc::new(drain_handle),
            wrapped_schema: schema.clone(),
            tag: "test",
            stage: None,
            hash_keys: None,
            drive_partition: 0,
        })
        .unwrap();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let ctx = SessionContext::new();

        let mut emitted_ids = Vec::new();
        rt.block_on(async {
            let n = wrapped.output_partitioning().partition_count();
            for p in 0..n {
                let mut s = wrapped.execute(p, ctx.task_ctx()).unwrap();
                while let Some(b) = s.next().await {
                    let b = b.unwrap();
                    let ids = b.column(0).as_any().downcast_ref::<Int32Array>().unwrap();
                    emitted_ids.extend(ids.values().iter().copied());
                }
            }
        });

        // Drain the outbound channel to verify the peer-destined rows
        // actually got shipped.
        let out_receiver = MppReceiver::new(Box::new(out_rx));
        let out_buffer = DrainBuffer::new(1);
        let out_handle = DrainHandle::spawn(DrainConfig::new(
            vec![out_receiver],
            Arc::clone(&out_buffer),
        ));
        let mut outbound_ids = Vec::new();
        while let DrainItem::Batch(b) = out_buffer.pop_front() {
            outbound_ids.extend(
                b.column(0)
                    .as_any()
                    .downcast_ref::<Int32Array>()
                    .unwrap()
                    .values()
                    .iter()
                    .copied(),
            );
        }
        out_handle.shutdown().unwrap();
        peer_thread.join().unwrap();

        // Union output: self-partition (even IDs) + peer-simulated (100, 200).
        emitted_ids.sort();
        assert_eq!(emitted_ids, vec![0, 2, 4, 6, 8, 100, 200]);
        // Outbound: ShuffleExec shipped odd IDs (rows that hashed to participant 1).
        outbound_ids.sort();
        assert_eq!(outbound_ids, vec![1, 3, 5, 7, 9]);
    }
}

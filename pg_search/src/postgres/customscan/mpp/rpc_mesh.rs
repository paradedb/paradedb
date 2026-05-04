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
//! RPC-style MPP mesh primitives.
//!
//! In the coordinator/worker MPP architecture every worker (including the
//! leader, which runs as worker 0 in the same process) drives a producer
//! subplan that hash-partitions rows into `K` consumer queues at the leader.
//! The total queue count is `N × K` where `N` is the worker count and `K` the
//! consumer-side partition count of the cut.
//!
//! [`MppRpcMesh`] is the runtime handle the customscan populates at DSM-init
//! time:
//! - `outbound_senders` — one per consumer partition; producer subplans push
//!   their hashed sub-batches through these.
//! - `inbound_drains` — one per `(worker, partition)` pair on the leader;
//!   `ShmMqWorkerConnection::stream_partition` yields from these.
//!
//! This file only defines the shape; population is the customscan's job and
//! lands together with the walker rewrite that emits the new operators.

use std::sync::Arc;

use crate::postgres::customscan::mpp::transport::{DrainBuffer, MppSender};

/// Runtime handle that wires the producer/consumer halves of an RPC-style MPP
/// cut to the underlying `shm_mq` mesh.
pub struct MppRpcMesh {
    /// Number of producer-side participants (workers, including leader-as-worker-0).
    pub n_workers: usize,
    /// Number of consumer-side partitions hosted on the leader.
    pub n_partitions: usize,
    /// Senders this participant owns, one per consumer partition.
    /// `outbound_senders.len() == n_partitions`. Empty on
    /// non-producer participants.
    pub outbound_senders: Vec<MppSender>,
    /// Inbound drain buffers indexed `worker * n_partitions + partition`.
    /// `inbound_drains.len() == n_workers * n_partitions`. Empty on
    /// non-consumer participants (i.e. plain workers; only the leader
    /// allocates these).
    pub inbound_drains: Vec<Arc<DrainBuffer>>,
}

impl MppRpcMesh {
    /// Look up the inbound drain for the `(worker, partition)` pair. Returns
    /// `None` if either index is out of range, or if this participant does not
    /// run a consumer plan (i.e. `inbound_drains` is empty).
    pub fn inbound_drain(&self, worker: usize, partition: usize) -> Option<&Arc<DrainBuffer>> {
        if worker >= self.n_workers || partition >= self.n_partitions {
            return None;
        }
        let idx = worker * self.n_partitions + partition;
        self.inbound_drains.get(idx)
    }

    /// Look up the outbound sender for `partition`. Returns `None` on
    /// non-producer participants.
    pub fn outbound_sender(&self, partition: usize) -> Option<&MppSender> {
        self.outbound_senders.get(partition)
    }
}

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
//! [`WorkerTransport`] implementation that streams from the leader's `shm_mq`
//! mesh.
//!
//! The fork's [`WorkerTransport`] seam expects `open(input_stage,
//! target_partitions, target_task, ctx, metrics)` to return a connection that
//! can yield one stream per partition. Our shm_mq-backed implementation maps
//! `target_task` to a producer-side worker index and `partition` to a
//! consumer-side partition index, then yields from the corresponding
//! [`DrainBuffer`] in the [`MppRpcMesh`] populated at DSM-init time.
//!
//! This file is dead code at merge time; the walker rewrite that emits
//! `NetworkShuffleExec` (consumer) on the leader and exposes the mesh handle
//! on the [`TaskContext`] lands in a follow-up PR.

use std::ops::Range;
use std::sync::Arc;

use datafusion::common::{DataFusionError, Result};
use datafusion::execution::TaskContext;
use datafusion::physical_expr_common::metrics::ExecutionPlanMetricsSet;
use datafusion_distributed::{Stage, WorkerConnection, WorkerPartitionStream, WorkerTransport};

use crate::postgres::customscan::mpp::rpc_mesh::MppRpcMesh;
use crate::postgres::customscan::mpp::transport::DrainItem;

/// `WorkerTransport` impl backed by the leader's [`MppRpcMesh`]. Holds an
/// `Arc` to the mesh so each `open()` call returns a thin handle without
/// re-acquiring locks.
pub struct ShmMqWorkerTransport {
    mesh: Arc<MppRpcMesh>,
}

impl ShmMqWorkerTransport {
    pub fn new(mesh: Arc<MppRpcMesh>) -> Self {
        Self { mesh }
    }
}

impl WorkerTransport for ShmMqWorkerTransport {
    fn open(
        &self,
        _input_stage: &Stage,
        _target_partitions: Range<usize>,
        target_task: usize,
        _ctx: &Arc<TaskContext>,
        _metrics: &ExecutionPlanMetricsSet,
    ) -> Result<Box<dyn WorkerConnection + Send + Sync>> {
        if target_task >= self.mesh.n_workers {
            return Err(DataFusionError::Internal(format!(
                "ShmMqWorkerTransport: target_task={target_task} >= n_workers={}",
                self.mesh.n_workers
            )));
        }
        Ok(Box::new(ShmMqWorkerConnection {
            mesh: Arc::clone(&self.mesh),
            worker_index: target_task,
        }))
    }
}

/// One open `WorkerConnection` to a single producer-side worker. Yields one
/// stream per consumer partition from the corresponding [`DrainBuffer`].
struct ShmMqWorkerConnection {
    mesh: Arc<MppRpcMesh>,
    worker_index: usize,
}

impl WorkerConnection for ShmMqWorkerConnection {
    fn stream_partition(&self, partition: usize) -> Result<WorkerPartitionStream> {
        let Some(drain) = self.mesh.inbound_drain(self.worker_index, partition) else {
            return Err(DataFusionError::Internal(format!(
                "ShmMqWorkerConnection: no drain for (worker={}, partition={partition})",
                self.worker_index
            )));
        };
        let drain = Arc::clone(drain);
        let stream = async_stream::stream! {
            while let DrainItem::Batch(batch) = drain.recv().await {
                yield Ok(batch);
            }
        };
        Ok(Box::pin(stream))
    }
}

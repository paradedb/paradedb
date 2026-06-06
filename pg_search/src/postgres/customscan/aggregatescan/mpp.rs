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

//! AggregateScan [`MppWorkerHost`] impl: exposes AggregateScan's typed state to the shared
//! dispatcher in [`mpp::host`]. The runtime lives nested under `datafusion_state`, and the
//! seed `SessionContext` is built from the aggregate profile.

use std::sync::Arc;

use datafusion::execution::context::SessionContext;

use crate::postgres::customscan::aggregatescan::datafusion_exec::create_aggregate_session_context;
use crate::postgres::customscan::aggregatescan::scan_state::MppExecState;
use crate::postgres::customscan::aggregatescan::AggregateScan;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use datafusion_distributed::embedded::MppSender;

use crate::postgres::customscan::mpp::exec_worker::MppWorkerInputs;
use crate::postgres::customscan::mpp::host::MppWorkerHost;

impl MppWorkerHost for CustomScanStateWrapper<AggregateScan> {
    fn already_drained(&self) -> bool {
        self.custom_state()
            .datafusion_state
            .as_ref()
            .and_then(|df| df.runtime.as_ref())
            .is_some()
    }

    fn take_worker_inputs(&mut self) -> MppWorkerInputs {
        // Read these out before we borrow df_state mutably. `parallel_state` and
        // `non_partitioning_segments` pin the worker's PgSearchTableProvider to the right
        // segment slice (partitioning source) and the leader's canonical replica
        // (non-partitioning sources). Without them every worker re-scans the full data
        // and the leader-side hash partitions see the same rows from every worker.
        let parallel_state = self.custom_state().parallel_state;
        let non_partitioning_segments = self.custom_state().non_partitioning_segments.clone();
        let partitioning_source_idx = self.custom_state().mpp_partitioning_source_idx.unwrap_or(0);
        let plan_sources_count = self
            .custom_state()
            .source_manifests
            .len()
            .max(non_partitioning_segments.len() + 1);

        let df_state = self
            .custom_state_mut()
            .datafusion_state
            .as_mut()
            .expect("DataFusion state must be initialized");

        let MppExecState::Worker(worker) = df_state.mpp.as_ref().expect("checked") else {
            unreachable!("exec_mpp_worker called outside Worker state");
        };
        let plan_bytes = worker.plan_bytes.clone();
        let worker_mesh = Arc::clone(&worker.mesh);
        let outbound_senders: Vec<Option<MppSender>> = match df_state.mpp.as_mut() {
            Some(MppExecState::Worker(w)) => std::mem::take(&mut w.outbound_senders),
            _ => unreachable!(),
        };

        MppWorkerInputs {
            parallel_state,
            non_partitioning_segments,
            partitioning_source_idx,
            plan_sources_count,
            plan_bytes,
            worker_mesh,
            outbound_senders,
        }
    }

    fn build_seed_ctx(&self) -> SessionContext {
        create_aggregate_session_context()
    }

    fn install_runtime(&mut self, runtime: tokio::runtime::Runtime) -> &tokio::runtime::Runtime {
        let df_state = self
            .custom_state_mut()
            .datafusion_state
            .as_mut()
            .expect("DataFusion state must be initialized");
        df_state.runtime = Some(runtime);
        df_state.runtime.as_ref().unwrap()
    }
}

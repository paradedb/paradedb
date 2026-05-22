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

//! JoinScan MPP worker exec path.
//!
//! Thin [`MppHostState`] impl that exposes JoinScan's typed state to the shared
//! dispatcher in [`mpp::host`]: the runtime lives directly on `custom_state`, and
//! the seed `SessionContext` is built from the join profile.

use std::sync::Arc;

use datafusion::execution::context::SessionContext;
use pgrx::pg_sys;

use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::joinscan::scan_state::{
    create_datafusion_session_context, MppExecState, SessionContextProfile,
};
use crate::postgres::customscan::joinscan::JoinScan;
use crate::postgres::customscan::mpp::exec_worker::MppWorkerInputs;
use crate::postgres::customscan::mpp::host::{exec_mpp_worker_impl, MppHostState};
use crate::postgres::customscan::mpp::transport::MppSender;

pub(super) fn exec_mpp_worker(
    state: &mut CustomScanStateWrapper<JoinScan>,
) -> *mut pg_sys::TupleTableSlot {
    exec_mpp_worker_impl(state)
}

impl MppHostState for CustomScanStateWrapper<JoinScan> {
    fn already_drained(&self) -> bool {
        self.custom_state().runtime.is_some()
    }

    fn take_worker_inputs(&mut self) -> MppWorkerInputs {
        let parallel_state = self.custom_state().parallel_state;
        let non_partitioning_segments = self.custom_state().non_partitioning_segments.clone();
        let partitioning_source_idx = self.custom_state().mpp_partitioning_source_idx.unwrap_or(0);
        let plan_sources_count = self
            .custom_state()
            .source_manifests
            .len()
            .max(non_partitioning_segments.len() + 1);

        let MppExecState::Worker(worker) = self.custom_state().mpp.as_ref().expect("checked")
        else {
            unreachable!("exec_mpp_worker called outside Worker state");
        };
        let plan_bytes = worker.plan_bytes.clone();
        let worker_mesh = Arc::clone(&worker.mesh);
        let outbound_senders: Vec<Option<MppSender>> = match self.custom_state_mut().mpp.as_mut() {
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
        create_datafusion_session_context(SessionContextProfile::Join)
    }

    fn install_runtime(&mut self, runtime: tokio::runtime::Runtime) -> &tokio::runtime::Runtime {
        self.custom_state_mut().runtime = Some(runtime);
        self.custom_state().runtime.as_ref().unwrap()
    }
}

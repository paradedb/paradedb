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
//! Thin wrapper over [`mpp::exec_worker::run_mpp_worker`]: pulls JoinScan-specific inputs out
//! of the typed state, builds the join-profile seed `SessionContext`, hands the whole thing
//! to the shape-agnostic dispatcher.

use std::sync::Arc;

use pgrx::pg_sys;

use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::joinscan::scan_state as join_scan_state;
use crate::postgres::customscan::joinscan::scan_state::{MppExecState, SessionContextProfile};
use crate::postgres::customscan::joinscan::JoinScan;
use crate::postgres::customscan::mpp::exec_worker::{run_mpp_worker, MppWorkerInputs};
use crate::postgres::customscan::mpp::transport::MppSender;
// `create_datafusion_session_context` lives in joinscan::scan_state and isn't pub from
// crate root, so import via the joinscan module path.
use crate::postgres::customscan::joinscan::scan_state::create_datafusion_session_context;

impl JoinScan {
    /// MPP worker exec: extract inputs from JoinScan's state, build the join-profile seed
    /// context, hand off to the shape-agnostic dispatcher. Workers emit zero rows back to PG;
    /// returning `null_mut()` signals end-of-stream.
    pub(super) fn exec_mpp_worker(
        state: &mut CustomScanStateWrapper<Self>,
    ) -> *mut pg_sys::TupleTableSlot {
        let parallel_state = state.custom_state().parallel_state;
        let non_partitioning_segments = state.custom_state().non_partitioning_segments.clone();
        let partitioning_source_idx = state
            .custom_state()
            .mpp_partitioning_source_idx
            .unwrap_or(0);
        let plan_sources_count = state
            .custom_state()
            .source_manifests
            .len()
            .max(non_partitioning_segments.len() + 1);

        // Already drained on a prior call; just signal EOF.
        if state.custom_state().runtime.is_some() {
            return std::ptr::null_mut();
        }

        let join_scan_state::MppExecState::Worker(worker) =
            state.custom_state().mpp.as_ref().expect("checked")
        else {
            unreachable!("exec_mpp_worker called outside Worker state");
        };
        let plan_bytes = worker.plan_bytes.clone();
        let worker_mesh = Arc::clone(&worker.mesh);
        let outbound_senders: Vec<Option<MppSender>> = match state.custom_state_mut().mpp.as_mut() {
            Some(MppExecState::Worker(w)) => std::mem::take(&mut w.outbound_senders),
            _ => unreachable!(),
        };

        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => pgrx::error!("mpp join worker: tokio runtime build failed: {e}"),
        };
        state.custom_state_mut().runtime = Some(runtime);
        let runtime = state.custom_state().runtime.as_ref().unwrap();

        let seed_ctx = create_datafusion_session_context(SessionContextProfile::Join);

        run_mpp_worker(
            MppWorkerInputs {
                parallel_state,
                non_partitioning_segments,
                partitioning_source_idx,
                plan_sources_count,
                plan_bytes,
                worker_mesh,
                outbound_senders,
            },
            seed_ctx,
            runtime,
        );
        std::ptr::null_mut()
    }
}

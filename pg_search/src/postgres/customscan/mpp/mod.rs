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

//! MPP (Massively Parallel Processing) plan partitioning for JoinScan and AggregateScan.
//!
//! Hash-partitions every table by the join key and shuffles intermediate rows between
//! workers through PostgreSQL `shm_mq` queues, so each row is scanned exactly once.
//! Guarded by `paradedb.enable_mpp` (default off).
//!
//! Transport deadlock-avoidance relies on one dedicated drain thread per participant
//! that reads all inbound queues into a spillable local buffer — this decouples
//! consumer-side backpressure from producer-side backpressure.

pub mod dsm;
pub mod exec;
pub mod glue;
pub mod mesh;
pub mod runtime;
pub mod transport;

use serde::{Deserialize, Serialize};

/// Describes this participant's position in an MPP query. Held by
/// [`glue::MppLeaderState`] / [`glue::MppWorkerState`] so the AggregateScan
/// worker path can size the in-process planner via `total_participants`.
/// The DF-D fork's `WorkerResolver` derives task identity from its own indexing,
/// so this is a diagnostic / sizing hand-off — not a `SessionConfig`
/// extension.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MppParticipantConfig {
    /// 0-based index of this participant. The leader is always index 0.
    pub participant_index: u32,
    /// Total number of participants (leader + workers).
    pub total_participants: u32,
}

/// Emit a runtime trace when `paradedb.mpp_debug` is on.
///
/// Routed through `pgrx::warning!` so the line appears in the Postgres server log
/// (and in CI benchmark logs). No-op when the GUC is off.
#[macro_export]
macro_rules! mpp_log {
    ($($arg:tt)*) => {
        if $crate::gucs::mpp_debug() {
            pgrx::warning!($($arg)*);
        }
    };
}

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
//! Issue #4152. Hash-partitions every table by the join key and shuffles intermediate
//! rows between workers through PostgreSQL `shm_mq` queues, so each row is scanned
//! exactly once. Guarded by `paradedb.enable_mpp` (default off).
//!
//! Transport deadlock-avoidance relies on one dedicated drain thread per participant
//! that reads all inbound queues into a spillable local buffer — this decouples
//! consumer-side backpressure from producer-side backpressure.

pub mod chain;
pub mod coordinator;
pub mod customscan_glue;
pub mod exec_bridge;
pub mod mesh;
pub mod plan_build;
pub mod session;
pub mod shape;
pub mod shuffle;
pub mod stage;
pub mod transport;
pub mod walker;
pub mod worker;

use serde::{Deserialize, Serialize};

/// Describes this participant's seat in an MPP query.
///
/// Injected into the DataFusion `SessionConfig` via `config_options` so downstream
/// operators (optimizer rules, hash partitioners) can discover it without an
/// out-of-band side channel.
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MppParticipantConfig {
    /// 0-based index of this participant. The leader is always index 0.
    pub participant_index: u32,
    /// Total number of participants (leader + workers).
    pub total_participants: u32,
}

/// Session-scoped MPP sharding info. Stashed as a DataFusion session
/// `config_extension` by `exec_datafusion_aggregate` on every participant
/// before `build_join_aggregate_plan` runs, so `PgSearchTableProvider::scan`
/// can shard segments deterministically across participants.
///
/// This is the path that lets the lazy-scan code (one `ScanState` wrapping
/// a `MultiSegmentSearchResults` of *all* segments) still parallelize — the
/// coarse shard at the `PgSearchScanPlan::states` vec level doesn't help
/// that path since there's only one state to filter. With `MppShardConfig`
/// set, the provider calls `reader.search_segments(sharded_ids)` and each
/// participant reads only its share of segments.
#[derive(Clone, Debug)]
pub struct MppShardConfig {
    pub participant_index: u32,
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

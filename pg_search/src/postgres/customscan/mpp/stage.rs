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
//! Stage / TaskKey descriptors for the MPP wire format.
//!
//! Each cut between two stages of an MPP plan is identified by an [`MppStage`]
//! (carrying `(query_id, stage_id, task_count)`); each individual stream
//! between two participants of that stage is keyed by an [`MppTaskKey`]
//! (carrying `(query_id, stage_id, task_number)`). Stamped on every framed
//! batch by the producer side and validated on the consumer side.
//!
//! The fork's `WorkerTransport` seam (introduced on
//! `paradedb/worker-transport`) consumes the equivalent on its own side via
//! the `Stage` struct re-exported from `datafusion-distributed`; we keep the
//! ParadeDB-side wire shape distinct because the in-process `shm_mq` mesh
//! does not address tasks by URL.

/// Identifies a sub-plan rooted at a network boundary.
///
/// One `MppStage` per boundary: the walker assigns a monotonic `stage_id`
/// bottom-up, so the leaf stage is 0 and each boundary above it increments.
/// `task_count` mirrors the number of parallel tasks in the child sub-plan;
/// for an in-process PG MPP query this equals
/// `MppParticipantConfig::total_participants`.
///
/// `query_id` is set by the leader at plan time. A `u64` suffices for
/// single-backend uniqueness (the query never crosses processes outside of
/// the spawned parallel workers, which all inherit it through DSM).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MppStage {
    pub query_id: u64,
    pub stage_id: u32,
    pub task_count: u32,
}

impl MppStage {
    pub fn new(query_id: u64, stage_id: u32, task_count: u32) -> Self {
        Self {
            query_id,
            stage_id,
            task_count,
        }
    }
}

/// Wire identifier for a single stream between two participants. Carried on
/// every framed batch so one `shm_mq` between two participants can in
/// principle multiplex multiple streams (one per `(stage, task,
/// partition)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MppTaskKey {
    pub query_id: u64,
    pub stage_id: u32,
    pub task_number: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpp_stage_is_constructible_and_equal() {
        let a = MppStage::new(7, 2, 4);
        let b = MppStage::new(7, 2, 4);
        assert_eq!(a, b);
        assert_eq!(a.query_id, 7);
        assert_eq!(a.stage_id, 2);
        assert_eq!(a.task_count, 4);
    }

    #[test]
    fn mpp_task_key_is_constructible_and_equal() {
        let a = MppTaskKey {
            query_id: 42,
            stage_id: 1,
            task_number: 3,
        };
        let b = MppTaskKey {
            query_id: 42,
            stage_id: 1,
            task_number: 3,
        };
        assert_eq!(a, b);
    }
}

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

//! Wire-format task descriptor for the MPP transport.
//!
//! [`MppTaskKey`] mirrors `datafusion-distributed`'s `TaskKey` protobuf
//! (`src/worker/worker.proto:51-59`). It rides every framed batch as part
//! of [`transport::FrameId`] so one `shm_mq` between two participants can
//! in principle carry multiple multiplexed streams (one per
//! `(stage, task, partition)` tuple). Today's mesh allocates one `shm_mq`
//! per `(stage, src, dst)` so the multiplex isn't strictly needed, but
//! stamping the descriptor now lets the future channel-flatten dispatcher
//! land without a wire-format break.
//!
//! Stage identity itself (the `(query_id, stage_id, task_count)` tuple
//! the walker stamps on each shuffle/gather boundary) is carried through
//! `datafusion_distributed::Stage` directly — see
//! `walker::emit_shuffle_cut` for the construction site.

/// Wire identifier for a single stream between two participants.
///
/// `query_id` is the low 64 bits of the corresponding
/// `datafusion_distributed::Stage::query_id` (see
/// `walker::emit_shuffle_cut`); `stage_id` matches `Stage::num`.
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

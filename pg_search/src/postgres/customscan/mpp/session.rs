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

//! Plan broadcast codec for MPP.
//!
//! The leader builds a DataFusion `LogicalPlan`, serializes it via the existing
//! `PgSearchExtensionCodec` in `pg_search/src/scan/codec.rs`, and wraps it
//! alongside the session profile + participant count in [`MppPlanBroadcast`].
//! That wrapper is `bincode`-serialized into a DSM region, workers read and
//! deserialize it, then reconstruct their own [`MppParticipantConfig`]
//! (identical `total_participants`, distinct `participant_index`) before
//! calling `create_datafusion_session_context_mpp`.
//!
//! The plan bytes are the same for every participant — participant identity
//! comes from the worker's seat in the mesh, not from the plan encoding.

use pgrx::pg_sys;
use serde::{Deserialize, Serialize};

use crate::postgres::customscan::mpp::MppParticipantConfig;

/// Wire-format tag for the DataFusion session profile. Distinct from the
/// `SessionContextProfile` enum in `joinscan/scan_state.rs` so we can evolve
/// the wire representation without forcing `Serialize`/`Deserialize` on the
/// executor-local type (which may later carry non-serializable fields like
/// `Arc<dyn ...>`). Conversion happens at (de)serialization time via the
/// `From` impls below, which compile-break if either enum grows an unmapped
/// variant.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MppSessionProfile {
    Join = 0,
    Aggregate = 1,
}

impl From<MppSessionProfile>
    for crate::postgres::customscan::joinscan::scan_state::SessionContextProfile
{
    fn from(wire: MppSessionProfile) -> Self {
        use crate::postgres::customscan::joinscan::scan_state::SessionContextProfile;
        match wire {
            MppSessionProfile::Join => SessionContextProfile::Join,
            MppSessionProfile::Aggregate => SessionContextProfile::Aggregate,
        }
    }
}

impl From<crate::postgres::customscan::joinscan::scan_state::SessionContextProfile>
    for MppSessionProfile
{
    fn from(
        executor: crate::postgres::customscan::joinscan::scan_state::SessionContextProfile,
    ) -> Self {
        use crate::postgres::customscan::joinscan::scan_state::SessionContextProfile;
        match executor {
            SessionContextProfile::Join => MppSessionProfile::Join,
            SessionContextProfile::Aggregate => MppSessionProfile::Aggregate,
        }
    }
}

/// Current wire-format version of [`MppPlanBroadcast`]. Bumped when fields are
/// added or reordered so an old worker reading a new leader's bytes aborts
/// cleanly instead of silently decoding shifted fields. bincode 2 does not
/// tag fields, so we do our own versioning.
///
/// - v1 → v2: added `query_id: u64` so workers key their `MppStage` /
///   `MppTaskKey` against the same value the leader stamped on boundary
///   nodes. Old worker reading v2 bytes (or vice versa) now rejects cleanly
///   with a "version mismatch" error instead of silently decoding shifted
///   fields.
pub const MPP_PLAN_BROADCAST_VERSION: u8 = 2;

/// Cheap per-query identifier for `MppStage.query_id`, derived on the leader
/// from `MyProcPid` (top 32 bits, XOR'd with timestamp bits) and
/// `GetCurrentStatementStartTimestamp` (bottom 32 bits). Both come from PG
/// state the leader owns at planning time.
///
/// # Uniqueness
///
/// `u64` is wide enough that collisions across coexisting queries are
/// negligible in practice: two different backends would need the same PID
/// and the same statement-start microsecond. Inside a single backend, the
/// bottom 32 bits (timestamp modulo ~71 minutes) distinguishes sequential
/// queries; the top 32 bits carry `pid ^ (ts >> 32)` so two simultaneous
/// backends disagree even when they happen to start inside the same
/// microsecond.
///
/// # Why not a UUID
///
/// DF-D uses a `uuid::Uuid` to stay unique across services. Our query never
/// crosses processes beyond the parallel workers spawned by the same backend,
/// which inherit the leader's broadcast — so u64 is sufficient and avoids a
/// new crate dependency. Chosen per review comment: "cheapest choice that
/// also helps with debugging". Printing the value in logs makes it easy to
/// correlate mesh traffic to a specific query.
///
/// # Safety
///
/// Both `pg_sys::MyProcPid` and `GetCurrentStatementStartTimestamp` are
/// always valid to read from the backend thread (the caller holds a
/// `pg_sys::PlannedStmt` lifetime, so PG is mid-query). Calling from a
/// non-backend thread would trip pgrx's `check_active_thread` guard; we
/// document that here so the FFI boundary is explicit.
pub fn derive_query_id() -> u64 {
    // SAFETY: both FFI calls are legal on the backend thread, which is
    // where the leader always lives during plan-stash (the only caller).
    unsafe {
        let pid = pg_sys::MyProcPid as u32 as u64;
        let ts = pg_sys::GetCurrentStatementStartTimestamp() as u64;
        ts ^ (pid << 32)
    }
}

/// Bundle the leader writes into DSM at query start; every worker reads the
/// same bytes and reconstructs its own `MppParticipantConfig` from the mesh
/// seat.
///
/// Wire format: bincode 2 `config::standard()` (varint-encoded, intentional
/// divergence from the persistent storage layer which uses `config::legacy()`
/// — this bundle is transient DSM traffic, never written to disk). The first
/// byte is [`MPP_PLAN_BROADCAST_VERSION`]; readers reject mismatches so an
/// older worker will never silently decode a newer leader's layout.
///
/// TODO(perf): `logical_plan` is `Vec<u8>`, which forces bincode to allocate
/// a fresh copy at decode time. For large plans (~500 KB) this is four copies
/// total: serialize-side (leader), bincode buffer (leader), DSM (both), and
/// deserialize-side (worker). A hand-rolled frame with a `&[u8]` view over
/// DSM would drop two of them; revisit if plan-broadcast cost shows up in
/// profiling.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MppPlanBroadcast {
    /// Wire-format version. Always equal to `MPP_PLAN_BROADCAST_VERSION` for
    /// bytes produced by `serialize`; `deserialize` rejects mismatches.
    version: u8,
    /// DataFusion logical plan bytes, produced by
    /// `crate::scan::codec::serialize_logical_plan`. Workers must use
    /// `deserialize_logical_plan_with_runtime` (not the test-only
    /// `deserialize_logical_plan`) so the runtime bindings attach correctly
    /// before physical planning.
    pub logical_plan: Vec<u8>,
    pub total_participants: u32,
    pub session_profile: MppSessionProfile,
    /// Per-query identifier stamped on every [`MppStage`] / [`MppTaskKey`]
    /// boundary descriptor. Workers reuse the same value so mesh framing
    /// agrees across seats. Leader fills via [`derive_query_id`].
    ///
    /// [`MppStage`]: super::stage::MppStage
    /// [`MppTaskKey`]: super::stage::MppTaskKey
    pub query_id: u64,
}

impl MppPlanBroadcast {
    pub fn new(
        logical_plan: Vec<u8>,
        total_participants: u32,
        session_profile: MppSessionProfile,
        query_id: u64,
    ) -> Self {
        Self {
            version: MPP_PLAN_BROADCAST_VERSION,
            logical_plan,
            total_participants,
            session_profile,
            query_id,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, bincode::error::EncodeError> {
        bincode::serde::encode_to_vec(self, bincode::config::standard())
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, bincode::error::DecodeError> {
        let (decoded, consumed): (Self, usize) =
            bincode::serde::decode_from_slice(bytes, bincode::config::standard())?;
        if consumed != bytes.len() {
            // Defense in depth: MPP plan bytes live in DSM at a known-length
            // offset; if the input slice is longer than the bincode frame,
            // either the caller passed the wrong slice bounds or the frame
            // itself is corrupt. Either case should fail loudly rather than
            // silently drop trailing bytes.
            return Err(bincode::error::DecodeError::OtherString(format!(
                "MppPlanBroadcast: consumed {consumed} of {} input bytes",
                bytes.len()
            )));
        }
        if decoded.version != MPP_PLAN_BROADCAST_VERSION {
            return Err(bincode::error::DecodeError::OtherString(format!(
                "MppPlanBroadcast version mismatch: got {}, expected {}",
                decoded.version, MPP_PLAN_BROADCAST_VERSION
            )));
        }
        Ok(decoded)
    }

    /// Produce the per-participant config for the worker that occupies the
    /// given seat. The leader is always seat 0.
    ///
    /// This uses `assert!` (not `debug_assert!`) because a silently bad seat
    /// index in release builds would manifest as "hash partitioner drops
    /// every row mapped to the nonexistent seat" — exactly the class of bug
    /// that produced COUNT(*) = 0 in the prior attempt (see project memory
    /// `project_mpp_correctness_bug.md`). Fail loudly at worker boot instead.
    pub fn participant_config(&self, participant_index: u32) -> MppParticipantConfig {
        assert!(
            participant_index < self.total_participants,
            "participant_index {participant_index} >= total_participants {}",
            self.total_participants
        );
        MppParticipantConfig {
            participant_index,
            total_participants: self.total_participants,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_broadcast_round_trips() {
        let orig = MppPlanBroadcast::new(
            vec![0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe],
            4,
            MppSessionProfile::Aggregate,
            0x1234_5678_9abc_def0,
        );
        let bytes = orig.serialize().expect("serialize");
        let decoded = MppPlanBroadcast::deserialize(&bytes).expect("deserialize");
        assert_eq!(decoded.logical_plan, orig.logical_plan);
        assert_eq!(decoded.total_participants, orig.total_participants);
        assert_eq!(decoded.session_profile, orig.session_profile);
        assert_eq!(decoded.query_id, orig.query_id);
    }

    #[test]
    fn participant_config_differs_per_seat() {
        let bc = MppPlanBroadcast::new(vec![], 3, MppSessionProfile::Join, 0);
        let leader = bc.participant_config(0);
        let w1 = bc.participant_config(1);
        let w2 = bc.participant_config(2);
        assert_eq!(leader.participant_index, 0);
        assert_eq!(w1.participant_index, 1);
        assert_eq!(w2.participant_index, 2);
        for pc in [leader, w1, w2] {
            assert_eq!(pc.total_participants, 3);
        }
    }

    #[test]
    fn deserialize_rejects_truncated_bytes() {
        let orig = MppPlanBroadcast::new(vec![1, 2, 3, 4], 2, MppSessionProfile::Join, 0);
        let bytes = orig.serialize().unwrap();
        // Truncate by removing the last byte — should not round-trip.
        let truncated = &bytes[..bytes.len() - 1];
        assert!(MppPlanBroadcast::deserialize(truncated).is_err());
    }

    #[test]
    fn deserialize_rejects_version_mismatch() {
        let mut orig = MppPlanBroadcast::new(vec![1, 2, 3], 2, MppSessionProfile::Join, 0);
        orig.version = MPP_PLAN_BROADCAST_VERSION.wrapping_add(1);
        let bytes = orig.serialize().unwrap();
        let err = MppPlanBroadcast::deserialize(&bytes).expect_err("version mismatch must reject");
        let msg = format!("{err}");
        assert!(msg.contains("version mismatch"), "unexpected error: {msg}");
    }

    #[test]
    fn enum_round_trips_via_from_impls() {
        use crate::postgres::customscan::joinscan::scan_state::SessionContextProfile;
        for wire in [MppSessionProfile::Join, MppSessionProfile::Aggregate] {
            let executor: SessionContextProfile = wire.into();
            let wire2: MppSessionProfile = executor.into();
            assert_eq!(wire, wire2);
        }
    }

    #[test]
    #[should_panic(expected = "participant_index")]
    fn participant_config_panics_on_out_of_bounds_seat() {
        let bc = MppPlanBroadcast::new(vec![], 2, MppSessionProfile::Join, 0);
        // Seat 2 is out of bounds for total_participants=2; panic at boot
        // time beats a silently dropped partition in production.
        let _ = bc.participant_config(2);
    }

    #[test]
    fn plan_broadcast_round_trips_large_payload() {
        // Simulate a realistic logical-plan byte vector (e.g., ~10 KB).
        let big = (0..10_000u32)
            .map(|i| (i % 251) as u8) // non-trivial pattern
            .collect::<Vec<u8>>();
        let orig = MppPlanBroadcast::new(big.clone(), 2, MppSessionProfile::Aggregate, 0xCAFE);
        let bytes = orig.serialize().unwrap();
        let decoded = MppPlanBroadcast::deserialize(&bytes).unwrap();
        assert_eq!(decoded.logical_plan, big);
    }
}

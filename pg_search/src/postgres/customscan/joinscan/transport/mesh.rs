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

use super::shmem::{
    MultiplexedDsmReader, MultiplexedDsmWriter, ParticipantId, SignalBridge, TransportLayout,
};
use super::ControlMessage;
use parking_lot::Mutex;
use std::sync::Arc;

/// A registry for process-local DSM communication channels.
///
/// This struct encapsulates the raw shared memory regions and the multiplexers
/// that allow this process to communicate with all other participants in the MPP session.
pub struct TransportMesh {
    /// Writers for sending data TO other participants.
    /// Index `j` corresponds to the channel sending to participant `j`.
    pub mux_writers: Vec<Arc<Mutex<MultiplexedDsmWriter>>>,

    /// Readers for receiving data FROM other participants.
    /// Index `j` corresponds to the channel receiving from participant `j`.
    pub mux_readers: Vec<Arc<Mutex<MultiplexedDsmReader>>>,

    /// The local signal bridge for this participant.
    pub bridge: Arc<SignalBridge>,
}

impl TransportMesh {
    /// Initializes the mesh from raw shared memory pointers.
    ///
    /// This function sets up the `MultiplexedDsmWriter`s and `MultiplexedDsmReader`s
    /// by slicing the monolithic shared memory region according to the standard layout:
    /// `[Participant 0 -> 0][Participant 0 -> 1]...[Participant P-1 -> P-1]`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// 1. `base_ptr` points to a valid, initialized shared memory region.
    /// 2. The region size is at least `region_size * total_participants * total_participants`.
    /// 3. The memory is accessible for the lifetime of the returned `TransportMesh`.
    pub unsafe fn init(
        base_ptr: *mut u8,
        layout: TransportLayout,
        participant_id: ParticipantId,
        total_participants: usize,
        bridge: Arc<SignalBridge>,
    ) -> Self {
        let mut mux_writers = Vec::with_capacity(total_participants);
        let mut mux_readers = Vec::with_capacity(total_participants);
        let region_size = layout.total_size();

        for j in 0..total_participants {
            // Writer: Us -> J
            // Layout index: participant_index * P + j
            let writer_idx = (participant_id.0 as usize) * total_participants + j;
            let offset = writer_idx * region_size;
            let writer_base = base_ptr.add(offset);

            mux_writers.push(Arc::new(Mutex::new(MultiplexedDsmWriter::new(
                writer_base,
                layout.data_capacity,
                layout.control_capacity,
                bridge.clone(),
                ParticipantId(j as u16),
            ))));

            // Reader: J -> Us
            // Layout index: j * P + participant_index
            let reader_idx = j * total_participants + (participant_id.0 as usize);
            let offset = reader_idx * region_size;
            let reader_base = base_ptr.add(offset);

            mux_readers.push(Arc::new(Mutex::new(MultiplexedDsmReader::new(
                reader_base,
                layout.data_capacity,
                layout.control_capacity,
                bridge.clone(),
                ParticipantId(j as u16),
            ))));
        }

        Self {
            mux_writers,
            mux_readers,
            bridge,
        }
    }

    /// Detaches all underlying readers, preventing further access to shared memory.
    /// This is a safety mechanism to prevent use-after-free when DSM is unmapped
    /// but Arrow buffers (holding leases) are still alive.
    pub fn detach(&self) {
        for reader in &self.mux_readers {
            reader.lock().detach();
        }
    }

    /// Returns the virtual memory address ranges of all shared memory ring buffers
    /// currently mapped by this mesh (via readers).
    ///
    /// This is used by `SharedMemoryDetector` to identify if an Arrow buffer resides
    /// in shared memory.
    pub fn memory_regions(&self) -> Vec<(usize, usize)> {
        self.mux_readers
            .iter()
            .map(|r| r.lock().memory_region())
            .collect()
    }

    /// Waits for the `BroadcastPlan` control message from the Leader.
    ///
    /// This method polls all writers (connections to other participants) for control messages.
    /// It enforces a strict protocol where the FIRST message received MUST be `BroadcastPlan`.
    pub async fn wait_for_broadcast_plan(&self) -> Vec<u8> {
        futures::future::poll_fn(|cx| {
            for mux in &self.mux_writers {
                let mut guard = mux.lock();
                match guard.poll_read_control_frame(cx) {
                    std::task::Poll::Ready((msg_type, payload)) => {
                        if let Some(ControlMessage::BroadcastPlan(bytes)) =
                            ControlMessage::try_from_frame(msg_type, &payload)
                        {
                            // Return the plan. Any subsequent messages remain in the ring buffer
                            // and will be consumed by the Control Service.
                            return std::task::Poll::Ready(bytes);
                        } else {
                            panic!(
                                "Received unexpected control message before BroadcastPlan: type {}",
                                msg_type
                            );
                        }
                    }
                    std::task::Poll::Pending => {} // Continue checking other muxes
                }
            }
            std::task::Poll::Pending
        })
        .await
    }
}

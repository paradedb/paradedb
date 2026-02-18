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

//! # Transport Layer
//!
//! This module implements the low-level data transport and signaling mechanisms for ParadeDB's
//! parallel execution engine (JoinScan). It decouples the physical movement of data (via Shared
//! Memory Ring Buffers) from the execution logic (DataFusion Plans).
//!
//! ## Architecture
//!
//! The transport layer is stratified into three components:
//!
//! 1.  **Physical Layer (`shmem.rs`)**:
//!     *   Manages raw Shared Memory (DSM) regions.
//!     *   Implements `TransportHeader`, `RingBufferHeader`, and `SignalBridge` (Unix Domain Sockets) for cross-process
//!         notification.
//!     *   Provides `MultiplexedDsmWriter` and `MultiplexedDsmReader` for multiplexing logical
//!         streams over a single physical connection.
//!
//! 2.  **Protocol Layer (`mod.rs`)**:
//!     *   Defines the `ControlMessage` enum (Start/Cancel stream).
//!     *   Provides the `TransportMesh` abstraction which encapsulates the topology of the
//!         parallel session (Writers/Readers for all participants).
//!
//! 3.  **Adaptation Layer (`arrow.rs`)**:
//!     *   Adapts the byte-oriented `shmem` streams to Arrow IPC.
//!     *   Provides `DsmWriter` (using `IpcDataGenerator` for 1-copy writes) and `dsm_reader`
//!         (supporting Zero-Copy reads).
//!     *   Implements optimal memory usage strategies:
//!         1.  **1-Copy Write**: Serializes batches directly to a local buffer before flushing
//!             to shared memory, avoiding intermediate Arrow IPC buffers.
//!         2.  **Zero-Copy Read**: When possible, maps shared memory regions directly to Arrow
//!             `Buffer`s, avoiding copies on the consumer side.
//!
//! ## Key Concepts
//!
//! ### 1. The Transport Mesh
//! The `TransportMesh` is the central registry of all physical connections. It is initialized once
//! per process (Leader or Worker) using a raw pointer to the base of the DSM region. It
//! automatically calculates offsets to establish:
//! *   `mux_writers`: A vector of writers, where index `j` connects to Participant `j`.
//! *   `mux_readers`: A vector of readers, where index `j` receives from Participant `j`.
//!
//! ### 2. Control Protocol
//! Execution is "Lazy" and "Pull-based".
//! *   **Request**: A consumer (Reader) sends a `ControlMessage::StartStream(id)` frame to the
//!     producer.
//! *   **Response**: The producer's `Control Service` (in `exchange.rs`) receives the frame,
//!     parses it, and triggers the corresponding execution task.
//! *   **Data**: Data flows back from producer to consumer via the ring buffer.
//!
//! ### 3. Multiplexing
//! Multiple logical streams (e.g., different stages of a query plan) share the same physical ring
//! buffer connection between two nodes. Each frame is prefixed with a
//! `[stream_id: u32][length: u32]` header.
//!
//! ## Usage
//!
//! Consumers (like `exchange.rs` and `parallel.rs`) should interact primarily with `TransportMesh`
//! and `ControlMessage`. They should not need to manipulate raw pointers or ring buffer headers
//! directly.

mod arrow;
mod mesh;
mod shmem;

// Re-export commonly used types
pub use arrow::{dsm_reader, DsmWriter};
pub use mesh::TransportMesh;
pub use shmem::{
    LogicalStreamId, MultiplexedDsmReader, MultiplexedDsmWriter, ParticipantId, PhysicalStreamId,
    SignalBridge, TransportLayout,
};

/// Control messages sent from Reader to Writer.
#[derive(Debug, Clone, PartialEq)]
pub enum ControlMessage {
    /// Request the writer to start producing data for this stream.
    StartStream(PhysicalStreamId),
    /// Request the writer to stop producing data.
    CancelStream(PhysicalStreamId),
    /// Broadcast the physical plan to all workers (Leader -> Worker).
    /// The payload is the serialized physical plan bytes.
    BroadcastPlan(Vec<u8>),
}

impl ControlMessage {
    pub fn try_from_frame(msg_type: u8, payload: &[u8]) -> Option<Self> {
        match msg_type {
            0 => {
                if payload.len() != 4 {
                    return None;
                }
                let stream_id = PhysicalStreamId(u32::from_le_bytes(payload.try_into().unwrap()));
                Some(ControlMessage::StartStream(stream_id))
            }
            1 => {
                if payload.len() != 4 {
                    return None;
                }
                let stream_id = PhysicalStreamId(u32::from_le_bytes(payload.try_into().unwrap()));
                Some(ControlMessage::CancelStream(stream_id))
            }
            128 => Some(ControlMessage::BroadcastPlan(payload.to_vec())),
            _ => None,
        }
    }
}

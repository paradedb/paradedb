//! # Transport Layer
//!
//! This module implements the low-level data transport and signaling mechanisms for ParadeDB's
//! distributed execution engine (JoinScan). It decouples the physical movement of data (via Shared
//! Memory Ring Buffers) from the execution logic (DataFusion Plans).
//!
//! ## Architecture
//!
//! The transport layer is stratified into three components:
//!
//! 1.  **Physical Layer (`shmem.rs`)**:
//!     *   Manages raw Shared Memory (DSM) regions.
//!     *   Implements `RingBufferHeader` and `SignalBridge` (Unix Domain Sockets) for cross-process
//!         notification.
//!     *   Provides `MultiplexedDsmWriter` and `MultiplexedDsmReader` for multiplexing logical
//!         streams over a single physical connection.
//!     *   **Visibility**: Module-private (`pub(super)`).
//!
//! 2.  **Protocol Layer (`mod.rs`)**:
//!     *   Defines the `ControlMessage` enum (Start/Cancel stream).
//!     *   Provides the `TransportMesh` abstraction which encapsulates the topology of the
//!         distributed session (Writers/Readers for all participants).
//!
//! 3.  **Adaptation Layer (`arrow.rs`)**:
//!     *   Adapts the byte-oriented `shmem` streams to Arrow IPC.
//!     *   Provides `DsmSharedMemoryWriter` (for `StreamWriter`) and `dsm_shared_memory_reader`
//!         (for `StreamDecoder`).
//!     *   Handles "double-buffering" to bridge the gap between synchronous `std::io::Write`
//!         (Arrow) and non-blocking shared memory.
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
pub use arrow::{dsm_shared_memory_reader, DsmSharedMemoryWriter};
pub use mesh::TransportMesh;
pub use shmem::{
    LogicalStreamId, MultiplexedDsmReader, MultiplexedDsmWriter, ParticipantId, PhysicalStreamId,
    RingBufferHeader, SignalBridge,
};

/// Control messages sent from Reader to Writer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControlMessage {
    /// Request the writer to start producing data for this stream.
    StartStream(PhysicalStreamId),
    /// Request the writer to stop producing data.
    CancelStream(PhysicalStreamId),
}

impl ControlMessage {
    pub fn try_from_frame(msg_type: u8, payload: &[u8]) -> Option<Self> {
        if payload.len() != 4 {
            return None;
        }
        let stream_id = PhysicalStreamId(u32::from_le_bytes(payload.try_into().unwrap()));
        match msg_type {
            0 => Some(ControlMessage::StartStream(stream_id)),
            1 => Some(ControlMessage::CancelStream(stream_id)),
            _ => None,
        }
    }
}

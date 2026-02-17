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

//! # Dynamic Shared Memory (DSM) Byte Streams
//!
//! This module provides a generic abstraction for multiplexed, asynchronous byte streams
//! over a shared memory ring buffer. It handles:
//!
//! 1. **Multiplexing**: Multiple logical streams can share a single physical ring buffer
//!    via a simple framing protocol (`[stream_id: u32][len: u32][payload]`).
//! 2. **Signaling**: Uses Unix Domain Sockets (`SignalBridge`) for async waking of Tokio tasks.
//! 3. **Stream Adapters**: Provides a `std::io::Write` adapter (`DsmStreamWriterAdapter`)
//!    and a direct demultiplexing reader (`MultiplexedDsmReader`) for easy integration
//!    with higher-level protocols (like Arrow IPC).

use crate::api::{HashMap, HashSet};
use std::collections::VecDeque;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::Waker;

use interprocess::local_socket::tokio::Listener;
use interprocess::local_socket::traits::tokio::Listener as _;
use interprocess::local_socket::{GenericFilePath, ListenerOptions, ToFsName};
use parking_lot::Mutex;
use tokio::io::AsyncReadExt;

/// A strongly-typed wrapper around `u16` representing a participant's unique index in the MPP session.
///
/// This type ensures safety and clarity when passing participant indices through the
/// transport layer, preventing confusion with other `usize` or `u32` values (like stream IDs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ParticipantId(pub u16);

impl std::fmt::Display for ParticipantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A robust signaling bridge using `interprocess` Local Sockets (Stream-oriented).
///
/// This component provides the async-friendly signaling required by the Tokio runtime.
/// Each participant in the MPP session binds its own dedicated local socket listener.
///
/// # Signaling Mechanism
///
/// When a producer writes data to a DSM buffer, it "signals" the consumer by
/// establishing a connection (if not already cached, performing a 4-byte handshake)
/// and writing a single byte.
///
/// We use synchronous `UnixStream` operations in **non-blocking mode** (after handshake):
/// 1.  **Low Latency**: Local socket operations are extremely fast.
/// 2.  **Safety**: We use non-blocking writes (`set_nonblocking(true)`). If the socket
///     buffer is full (consumer not processing signals), the write returns `WouldBlock`
///     and we silently ignore it. This avoids deadlocks in single-threaded runtimes where
///     blocking the producer might prevent the consumer's acceptor task from running.
///     Dropping signals is safe because a full buffer implies the consumer already has
///     pending signals queued in the kernel to wake it up.
/// 3.  **Simplicity**: Allows usage within non-async contexts without needing a runtime handle.
///
/// # Caching
///
/// The bridge maintains a cache of open connections (`outgoing`) to other participants.
/// This avoids the overhead of `connect()` syscalls and path construction on every signal,
/// which is critical for high-throughput streaming.
pub struct SignalBridge {
    participant_id: ParticipantId,
    session_id: uuid::Uuid,
    /// Cache of outgoing synchronous connections to other participants.
    /// We use `parking_lot::Mutex` for low-overhead synchronous locking.
    outgoing: Mutex<HashMap<ParticipantId, UnixStream>>,
    /// Wakers sharded by the sender who triggered the signal.
    /// `None` key stores "Universal" wakers (e.g. Control Service) that are woken by any signal.
    /// `Some(id)` key stores wakers interested only in signals from `id`.
    wakers: Arc<Mutex<HashMap<Option<ParticipantId>, Vec<Waker>>>>,
}

impl SignalBridge {
    fn socket_name(session_id: uuid::Uuid, id: ParticipantId) -> std::io::Result<String> {
        // Use a filesystem path in /tmp. This works on Unix.
        // interprocess supports namespaced names on Linux (@...) but macOS requires paths.
        // We use explicit filesystem paths for consistency.
        Ok(format!("/tmp/pdb_mpp_{}_{}.sock", session_id, id))
    }

    pub async fn new(
        participant_id: ParticipantId,
        session_id: uuid::Uuid,
    ) -> std::io::Result<Self> {
        let name_str = Self::socket_name(session_id, participant_id)?;
        // Clean up previous file if it exists
        if std::fs::metadata(&name_str).is_ok() {
            let _ = std::fs::remove_file(&name_str);
        }

        let name = name_str.to_fs_name::<GenericFilePath>()?;
        let listener = ListenerOptions::new().name(name).create_tokio()?;

        let wakers = Arc::new(Mutex::new(HashMap::default()));
        let bridge = Self {
            participant_id,
            session_id,
            outgoing: Mutex::new(HashMap::default()),
            wakers,
        };

        bridge.spawn_acceptor(listener);
        Ok(bridge)
    }

    #[cfg(any(test, feature = "pg_test"))]
    pub fn new_dummy() -> Arc<Self> {
        Arc::new(Self {
            participant_id: ParticipantId(0),
            session_id: uuid::Uuid::new_v4(),
            outgoing: Mutex::new(HashMap::default()),
            wakers: Arc::new(Mutex::new(HashMap::default())),
        })
    }

    fn spawn_acceptor(&self, listener: Listener) {
        let wakers = self.wakers.clone();

        tokio::task::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok(mut stream) => {
                        let wakers = wakers.clone();
                        tokio::task::spawn(async move {
                            // Handshake: Read Sender ParticipantId (u32)
                            let mut id_buf = [0u8; 4];
                            if stream.read_exact(&mut id_buf).await.is_err() {
                                return;
                            }
                            let sender_id = ParticipantId(u32::from_le_bytes(id_buf) as u16);

                            // Read in larger chunks to drain coalesced signals efficiently
                            let mut buf = [0u8; 1024];
                            loop {
                                match stream.read(&mut buf).await {
                                    Ok(0) => break, // EOF
                                    Ok(_) => {
                                        // Drop the lock before waking tasks to prevent deadlocks
                                        let wakers_to_wake: Vec<_> = {
                                            let mut guard = wakers.lock();
                                            let mut to_wake = Vec::new();
                                            // Wake specific listeners
                                            if let Some(list) = guard.get_mut(&Some(sender_id)) {
                                                to_wake.append(list);
                                            }
                                            // Wake universal listeners
                                            // Note: We currently wake universal listeners on EVERY signal.
                                            // Ideally we might want to be selective, but for Control Service
                                            // we don't know who sent the control message until we check the ring buffer.
                                            // So waking Control Service on every signal is correct behavior.
                                            if let Some(list) = guard.get_mut(&None) {
                                                // We must CLONE universal wakers because they might need to be
                                                // woken by other participants too. Or do we drain them?
                                                // If we drain them, they need to re-register.
                                                // Standard poll_fn pattern re-registers waker every poll.
                                                // So draining is correct.
                                                to_wake.append(list);
                                            }
                                            to_wake
                                        };
                                        for waker in wakers_to_wake {
                                            waker.wake();
                                        }
                                    }
                                    Err(_) => break,
                                }
                            }
                        });
                    }
                    Err(e) => {
                        pgrx::warning!("SignalBridge acceptor error: {}", e);
                        // Sleep instead of yield to prevent 100% CPU spin loops
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    }
                }
            }
        });
    }

    /// Signals a participant by writing a 4-byte handshake (if new connection) or a byte to a stream connected to its socket.
    ///
    /// # Blocking Behavior
    ///
    /// This method uses **blocking I/O** for the initial handshake transmission (writing 4 bytes)
    /// when establishing a new connection. This is considered safe because writing 4 bytes to a
    /// freshly created local Unix socket is effectively non-blocking (kernel buffers are empty).
    ///
    /// Subsequent signals use **non-blocking I/O** and handle `WouldBlock` by silently dropping the signal
    /// (event coalescing), ensuring that the main loop is never stalled by a slow consumer.
    pub fn signal(&self, target_id: ParticipantId) -> std::io::Result<()> {
        if target_id == self.participant_id {
            // Extract wakers before waking to prevent deadlocks
            let wakers_to_wake: Vec<_> = {
                let mut guard = self.wakers.lock();
                let mut to_wake = Vec::new();
                if let Some(list) = guard.get_mut(&Some(self.participant_id)) {
                    to_wake.append(list);
                }
                if let Some(list) = guard.get_mut(&None) {
                    to_wake.append(list);
                }
                to_wake
            };
            for waker in wakers_to_wake {
                waker.wake();
            }
            return Ok(());
        }

        let needs_connect = {
            let guard = self.outgoing.lock();
            !guard.contains_key(&target_id)
        };

        if needs_connect {
            let name_str = Self::socket_name(self.session_id, target_id)?;

            let mut stream = loop {
                match UnixStream::connect(&name_str) {
                    Ok(s) => break s,
                    Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                    // Safely ignore connection errors if backlog is full or not bound yet
                    Err(e)
                        if e.kind() == ErrorKind::WouldBlock
                            || e.kind() == ErrorKind::ConnectionRefused
                            || e.kind() == ErrorKind::NotFound =>
                    {
                        return Ok(());
                    }
                    Err(e) => return Err(e),
                }
            };

            // Handshake: Write our ParticipantId (u32)
            // Note: This is blocking, but on a local socket with empty buffer it should be instant.
            let _ = stream.write_all(&(self.participant_id.0 as u32).to_le_bytes());

            stream.set_nonblocking(true)?;
            self.outgoing.lock().insert(target_id, stream);
        }

        let mut guard = self.outgoing.lock();
        let stream = match guard.get_mut(&target_id) {
            Some(s) => s,
            None => return Ok(()),
        };

        loop {
            match stream.write(&[1]) {
                Ok(_) => return Ok(()),
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(()),
                Err(e) if e.kind() == ErrorKind::BrokenPipe => {
                    guard.remove(&target_id);
                    // Drop lock before reconnecting to prevent stalling other signals
                    drop(guard);

                    let name_str = Self::socket_name(self.session_id, target_id)?;
                    let mut stream = loop {
                        match UnixStream::connect(&name_str) {
                            Ok(s) => break s,
                            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                            Err(e)
                                if e.kind() == ErrorKind::WouldBlock
                                    || e.kind() == ErrorKind::ConnectionRefused
                                    || e.kind() == ErrorKind::NotFound =>
                            {
                                return Ok(());
                            }
                            Err(e) => return Err(e),
                        }
                    };

                    // Re-handshake
                    let _ = stream.write_all(&(self.participant_id.0 as u32).to_le_bytes());

                    stream.set_nonblocking(true)?;

                    let res = loop {
                        match stream.write(&[1]) {
                            Ok(_) => break Ok(()),
                            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                            Err(e) if e.kind() == ErrorKind::WouldBlock => break Ok(()),
                            Err(e) => break Err(e),
                        }
                    };

                    // Cache the newly established stream so it isn't dropped!
                    self.outgoing.lock().insert(target_id, stream);
                    return res;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Registers a waker to be notified when a signal arrives from a specific participant.
    /// If `source_id` is `None`, the waker is notified on ANY signal (Broadcast).
    pub fn register_waker(&self, waker: Waker, source_id: Option<ParticipantId>) {
        let mut guard = self.wakers.lock();
        let list = guard.entry(source_id).or_default();
        // Deduplicate wakers to prevent memory leaks from spurious polls
        for w in list.iter_mut() {
            if w.will_wake(&waker) {
                *w = waker; // Replace with updated waker just in case
                return;
            }
        }
        list.push(waker);
    }
}

pub const DSM_MAGIC: u64 = 0x5044_425F_4453_4D31; // "PDB_DSM1"

/// The top-level header for a Shared Memory Transport region.
/// Located at the start of the DSM (Dynamic Shared Memory) region for each worker.
///
/// This structure wraps the main data ring buffer and provides an offset to a secondary
/// control channel (also a ring buffer) used for signaling (e.g. stream cancellation).
///
/// # Safety
///
/// This struct contains `AtomicU64` fields (via `RingBufferHeader`), which means it has interior mutability and is NOT `Pod`.
/// Therefore, we cannot use `bytemuck::from_bytes` to safely cast a byte slice to a reference of this struct.
/// Users must manually ensure that the backing memory is properly aligned (align 8) and sized
/// before casting raw pointers.
#[repr(C)]
struct TransportHeader {
    ring: RingBufferHeader,
    /// Offset from the start of the DSM region to the control block.
    /// The control block is used for reverse-channel signaling (e.g. cancellations).
    /// If 0, the control block is not present.
    control_offset: usize,
}

impl TransportHeader {
    /// Initializes a `TransportHeader` at the given pointer.
    unsafe fn init(header: *mut TransportHeader, control_offset: usize) {
        let ring = &mut (*header).ring;
        RingBufferHeader::init(ring);
        (*header).control_offset = control_offset;
    }
}

/// Helper to calculate layout and initialize a Transport region.
///
/// # Memory Layout
///
/// A Transport Region is a contiguous block of Shared Memory containing a Main Data Channel
/// and a Secondary Control Channel.
///
/// ```text
/// +-----------------------------------------------------------+
/// |  TransportHeader (Private Struct)                         |
/// |-----------------------------------------------------------|
/// |  ring: RingBufferHeader                                   | <--- Main Data Channel Header
/// |        - magic (u64)                                      |
/// |        - write_pos (AtomicU64)                            |
/// |        - read_pos (AtomicU64)                             |
/// |        - finished (AtomicU64)                             |
/// |-----------------------------------------------------------|
/// |  control_offset: usize                                    | <--- Offset to Control Region
/// +-----------------------------------------------------------+
/// |                                                           |
/// |  MAIN DATA RING BUFFER                                    |
/// |  (Size: data_capacity)                                    |
/// |                                                           |
/// +-----------------------------------------------------------+ <--- Aligned to 8 bytes
/// |  Control Ring Buffer Header (RingBufferHeader)            | <--- Control Channel Header
/// +-----------------------------------------------------------+
/// |                                                           |
/// |  CONTROL DATA BUFFER                                      |
/// |  (Size: control_capacity)                                 |
/// |                                                           |
/// +-----------------------------------------------------------+
/// ```
#[derive(Debug, Clone, Copy)]
pub struct TransportLayout {
    pub data_capacity: usize,
    pub control_capacity: usize,
}

impl TransportLayout {
    pub fn new(data_capacity: usize, control_capacity: usize) -> Self {
        Self {
            data_capacity,
            control_capacity,
        }
    }

    fn align_up(offset: usize, align: usize) -> usize {
        (offset + align - 1) & !(align - 1)
    }

    pub fn total_size(&self) -> usize {
        let control_start = self.control_offset();
        let control_end =
            control_start + std::mem::size_of::<RingBufferHeader>() + self.control_capacity;
        // Padding
        control_end + 64
    }

    pub fn control_offset(&self) -> usize {
        let unaligned = std::mem::size_of::<TransportHeader>() + self.data_capacity;
        Self::align_up(unaligned, std::mem::align_of::<RingBufferHeader>())
    }

    /// Initializes the headers at the given base pointer.
    ///
    /// # Safety
    /// `base_ptr` must point to a valid memory region of at least `total_size()` bytes.
    pub unsafe fn init(&self, base_ptr: *mut u8) {
        // 1. Initialize Main Header
        let header = base_ptr as *mut TransportHeader;
        TransportHeader::init(header, self.control_offset());

        // 2. Initialize Control Header
        let control_ptr = base_ptr.add(self.control_offset());
        let control_header = control_ptr as *mut RingBufferHeader;
        RingBufferHeader::init(control_header);
    }
}

/// The header for a single circular buffer (SPSC queue).
///
/// This structure maintains the state (`write_pos`, `read_pos`) for a ring buffer.
/// It is used for:
/// 1. The main data channel (embedded in `TransportHeader`).
/// 2. The secondary control channel (located at `control_offset`).
///
/// # Safety
///
/// This struct contains `AtomicU64` fields, which means it has interior mutability and is NOT `Pod`.
/// Therefore, we cannot use `bytemuck::from_bytes` to safely cast a byte slice to a reference of this struct.
/// Users must manually ensure that the backing memory is properly aligned (align 8) and sized
/// before casting raw pointers.
#[repr(C)]
struct RingBufferHeader {
    /// Magic number to detect memory corruption.
    magic: u64,

    /// Monotonically increasing counter of total bytes written to the data region.
    /// The producer increments this after completing a write.
    write_pos: AtomicU64,

    /// Monotonically increasing counter of total bytes read from the data region.
    /// The consumer increments this after consuming a batch.
    read_pos: AtomicU64,

    /// A flag indicating that the producer has completed its execution and
    /// no more data will be written.
    finished: AtomicU64,
}

impl RingBufferHeader {
    /// Initializes a `RingBufferHeader` at the given pointer.
    ///
    /// This writes the magic number and zeroes the counters.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `header` points to a valid, aligned, and writeable memory region
    /// of size `size_of::<RingBufferHeader>()`.
    unsafe fn init(header: *mut RingBufferHeader) {
        std::ptr::write(
            header,
            RingBufferHeader {
                magic: DSM_MAGIC,
                write_pos: AtomicU64::new(0),
                read_pos: AtomicU64::new(0),
                finished: AtomicU64::new(0),
            },
        );
    }
}

/// Internal adapter for writing to the DSM ring buffer via `std::io::Write`.
struct DsmWriteAdapter {
    header: *mut RingBufferHeader,
    data: *mut u8,
    data_len: usize,
}

impl DsmWriteAdapter {
    fn new(header: *mut RingBufferHeader, data: *mut u8, data_len: usize) -> Self {
        unsafe {
            // Check magic
            if (*header).magic != DSM_MAGIC {
                // We can't return error from new(), but we can log.
                // In production this implies severe corruption or uninitialized memory.
                pgrx::warning!(
                    "DsmWriteAdapter::new: Invalid magic number in header: {:x}",
                    (*header).magic
                );
            }
        }
        Self {
            header,
            data,
            data_len,
        }
    }

    /// Calculates how many bytes can currently be written to the buffer.
    fn available_space(&self) -> usize {
        let write_pos = unsafe { (*self.header).write_pos.load(Ordering::Acquire) };
        let read_pos = unsafe { (*self.header).read_pos.load(Ordering::Acquire) };
        // The distance between write and read positions determines occupancy.
        let used = write_pos.wrapping_sub(read_pos) as usize;
        if used > self.data_len {
            // TODO: This state should be impossible with correct logic.
            // It implies either memory corruption (overwritten header) or a race condition
            // where read_pos/write_pos are desynchronized.
            // We return 0 (full) to safely block the writer instead of crashing.
            pgrx::warning!(
                "DsmWriteAdapter::available_space: Invalid state! write_pos={}, read_pos={}, data_len={}, used={}",
                write_pos,
                read_pos,
                self.data_len,
                used
            );
            return 0;
        }
        self.data_len - used
    }
}

impl std::io::Write for DsmWriteAdapter {
    /// Serializes raw bytes into the ring buffer.
    ///
    /// This handles wrapping around the end of the circular buffer by performing
    /// two `copy_nonoverlapping` calls if the message is split across the boundary.
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        unsafe {
            if (*self.header).magic != DSM_MAGIC {
                return Err(std::io::Error::other(format!(
                    "DsmWriteAdapter::write: RingBufferHeader corruption (magic mismatch: {:x})",
                    (*self.header).magic
                )));
            }
        }

        let len = buf.len();
        if len > self.data_len {
            return Err(std::io::Error::other(format!(
                "Write size {} exceeds ring buffer capacity {}",
                len, self.data_len
            )));
        }

        // Non-blocking: check if we have enough space.
        if self.available_space() < len {
            return Err(std::io::Error::from(ErrorKind::WouldBlock));
        }

        unsafe {
            let write_pos = (*self.header).write_pos.load(Ordering::Acquire);
            let offset = (write_pos % self.data_len as u64) as usize;

            if offset + len <= self.data_len {
                // The write fits contiguously at the end of the buffer.
                std::ptr::copy_nonoverlapping(buf.as_ptr(), self.data.add(offset), len);
            } else {
                // The write wraps around to the start of the buffer.
                let first_part = self.data_len - offset;
                let second_part = len - first_part;
                std::ptr::copy_nonoverlapping(buf.as_ptr(), self.data.add(offset), first_part);
                std::ptr::copy_nonoverlapping(buf.as_ptr().add(first_part), self.data, second_part);
            }

            // Update write position.
            (*self.header)
                .write_pos
                .fetch_add(len as u64, Ordering::Release);
        }

        Ok(len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A multiplexer for writing multiple logical streams into a single DSM ring buffer.
///
/// Framing: `[stream_id: u32][len: u32][payload: len bytes]`
pub struct MultiplexedDsmWriter {
    adapter: DsmWriteAdapter,
    /// Set of stream IDs that have been cancelled by the reader.
    cancelled_streams: HashSet<PhysicalStreamId>,
    /// Reader for the control channel (reverse direction).
    control_reader: Option<DsmReadAdapter>,
    /// Bridge for signaling the remote reader.
    bridge: Arc<SignalBridge>,
    /// Index of the remote participant (reader).
    remote_id: ParticipantId,
}

unsafe impl Send for MultiplexedDsmWriter {}
unsafe impl Sync for MultiplexedDsmWriter {}

impl std::fmt::Debug for MultiplexedDsmWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiplexedDsmWriter")
            .field("remote_id", &self.remote_id)
            .field("cancelled_streams", &self.cancelled_streams)
            .finish_non_exhaustive()
    }
}

/// A unique identifier for a physical stream (Logical Stream + Sender).
///
/// This ID is used to multiplex multiple logical data streams over a single physical
/// shared memory ring buffer connection between two participants.
///
/// The ID is a 32-bit integer packed as follows:
/// - **High 16 bits**: The `LogicalStreamId` (from the query plan).
/// - **Low 16 bits**: The `ParticipantId` of the sender.
///
/// This packing strategy assumes that there are fewer than 65536 logical streams
/// and fewer than 65536 participants in a single query execution, which is safe
/// for PostgreSQL's limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhysicalStreamId(pub u32);

impl PhysicalStreamId {
    /// Creates a new `PhysicalStreamId` from a logical stream ID and a sender ID.
    pub fn new(logical: LogicalStreamId, participant_id: ParticipantId) -> Self {
        Self(((logical.0 as u32) << 16) | ((participant_id.0 as u32) & 0xFFFF))
    }

    pub fn to_le_bytes(self) -> [u8; 4] {
        self.0.to_le_bytes()
    }
}

/// A unique identifier for a logical stream in the execution plan.
///
/// This ID corresponds to a specific shuffle/exchange operation in the DataFusion plan.
/// All participants (senders) participating in this exchange will use the same `LogicalStreamId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct LogicalStreamId(pub u16);

impl std::fmt::Display for LogicalStreamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl MultiplexedDsmWriter {
    /// Creates a new multiplexed writer.
    ///
    /// If `control_offset` is non-zero in the header, it also initializes a reader
    /// for the reverse control channel to receive stream cancellation signals.
    pub fn new(
        base_ptr: *mut u8,
        data_capacity: usize,
        bridge: Arc<SignalBridge>,
        remote_id: ParticipantId,
    ) -> Self {
        let header = base_ptr as *mut TransportHeader;
        let data = unsafe { base_ptr.add(std::mem::size_of::<TransportHeader>()) };

        unsafe {
            if (*header).ring.magic != DSM_MAGIC {
                pgrx::warning!(
                    "MultiplexedDsmWriter::new: Invalid magic number in header: {:x}",
                    (*header).ring.magic
                );
            }
        }

        let control_reader = unsafe {
            let offset = (*header).control_offset;
            if offset > 0 {
                // Sanity check control offset to avoid wrapping
                if offset < std::mem::size_of::<TransportHeader>() {
                    pgrx::warning!(
                        "MultiplexedDsmWriter::new: Invalid control_offset {} (too small)",
                        offset
                    );
                }

                // Calculate pointer to control block
                let control_ptr = base_ptr.add(offset);
                let control_header = control_ptr as *mut RingBufferHeader;

                // Data starts after the header.
                let control_data = control_ptr.add(std::mem::size_of::<RingBufferHeader>());
                // We'll use a fixed size for the control buffer for simplicity of this patch,
                // matching what we'll allocate in the test/transfer logic (e.g. 64KB).
                // TODO: Store control capacity in header or layout? For now constant is fine as long as consistent.
                let control_len = 65536;

                Some(DsmReadAdapter::new(
                    control_header,
                    control_data,
                    control_len,
                ))
            } else {
                None
            }
        };

        let adapter = unsafe { DsmWriteAdapter::new(&mut (*header).ring, data, data_capacity) };

        Self {
            adapter,
            cancelled_streams: HashSet::default(),
            control_reader,
            bridge,
            remote_id,
        }
    }

    /// Reads pending control frames from the reverse channel.
    /// Returns a vector of (message_type, payload).
    pub fn read_control_frames(&mut self) -> Vec<(u8, Vec<u8>)> {
        let mut frames = Vec::new();
        if let Some(reader) = &mut self.control_reader {
            // Read all available messages. Each message is 1 byte type + 4 bytes payload.
            let mut header_buf = [0u8; 1];
            while reader.has_data() {
                match reader.read(&mut header_buf) {
                    Ok(1) => {
                        let msg_type = header_buf[0];
                        let mut payload_buf = [0u8; 4];
                        match reader.read(&mut payload_buf) {
                            Ok(4) => {
                                frames.push((msg_type, payload_buf.to_vec()));
                            }
                            _ => break, // Partial read
                        }
                    }
                    Ok(_) => break,  // EOF or partial
                    Err(_) => break, // WouldBlock
                }
            }
        }
        frames
    }

    /// Mark a stream as cancelled, preventing further writes to it.
    pub fn mark_stream_cancelled(&mut self, stream_id: PhysicalStreamId) {
        self.cancelled_streams.insert(stream_id);
    }

    /// Writes a framed message to the ring buffer.
    pub(super) fn write_message(
        &mut self,
        stream_id: PhysicalStreamId,
        payload: &[u8],
    ) -> std::io::Result<()> {
        if self.cancelled_streams.contains(&stream_id) {
            return Err(std::io::Error::from(ErrorKind::BrokenPipe));
        }

        let len = payload.len() as u32;
        let header_len = 8;
        // Calculate padding to ensure the total message size (header + payload + padding) is 8-byte aligned.
        let padding = (8 - (len % 8)) % 8;
        let total_len = (header_len + len + padding) as usize;

        if total_len > self.adapter.data_len {
            return Err(std::io::Error::other(format!(
                "Framed message size {} exceeds ring buffer capacity {}",
                total_len, self.adapter.data_len
            )));
        }

        // Check if the entire framed message fits before writing anything.
        if self.adapter.available_space() < total_len {
            return Err(std::io::Error::from(ErrorKind::WouldBlock));
        }

        self.adapter.write_all(&stream_id.to_le_bytes())?;
        self.adapter.write_all(&len.to_le_bytes())?;
        self.adapter.write_all(payload)?;

        if padding > 0 {
            let pad_bytes = [0u8; 8];
            self.adapter.write_all(&pad_bytes[0..padding as usize])?;
        }

        // Signal the remote reader
        if let Err(e) = self.bridge.signal(self.remote_id) {
            pgrx::warning!("Signal error to remote {}: {}", self.remote_id, e);
        }
        Ok(())
    }

    /// Closes a specific stream by sending an empty message (len=0).
    pub(super) fn close_stream(&mut self, stream_id: PhysicalStreamId) -> std::io::Result<()> {
        if self.cancelled_streams.contains(&stream_id) {
            return Ok(());
        }

        // Write header with len=0
        let len = 0u32;
        let total_len = 8; // Just header, already aligned

        if self.adapter.available_space() < total_len {
            return Err(std::io::Error::from(ErrorKind::WouldBlock));
        }

        self.adapter.write_all(&stream_id.to_le_bytes())?;
        self.adapter.write_all(&len.to_le_bytes())?;
        // No payload, no padding

        let _ = self.bridge.signal(self.remote_id);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn finish(&mut self) -> std::io::Result<()> {
        unsafe {
            if (*self.adapter.header).magic != DSM_MAGIC {
                return Err(std::io::Error::other(
                    "RingBufferHeader corruption (magic mismatch)",
                ));
            }
            (*self.adapter.header).finished.store(1, Ordering::Release);
        }
        let _ = self.bridge.signal(self.remote_id);
        Ok(())
    }
}

/// An adapter for a specific logical stream in a multiplexed DSM writer.
pub struct DsmStreamWriterAdapter {
    multiplexer: Arc<Mutex<MultiplexedDsmWriter>>,
    pub stream_id: PhysicalStreamId,
    buffer: Vec<u8>,
}

impl DsmStreamWriterAdapter {
    pub fn new(multiplexer: Arc<Mutex<MultiplexedDsmWriter>>, stream_id: PhysicalStreamId) -> Self {
        Self {
            multiplexer,
            stream_id,
            buffer: Vec::new(),
        }
    }

    pub fn close_stream(&self) -> std::io::Result<()> {
        self.multiplexer.lock().close_stream(self.stream_id)
    }
}

impl std::io::Write for DsmStreamWriterAdapter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if !self.buffer.is_empty() {
            let mut mux = self.multiplexer.lock();
            mux.write_message(self.stream_id, &self.buffer)?;
            self.buffer.clear();
        }
        Ok(())
    }
}

/// A bridge between the Shared Memory Ring Buffer and the `std::io::Read` trait.
struct DsmReadAdapter {
    header: *mut RingBufferHeader,
    data: *mut u8,
    data_len: usize,
}

impl DsmReadAdapter {
    fn new(header: *mut RingBufferHeader, data: *mut u8, data_len: usize) -> Self {
        unsafe {
            if (*header).magic != DSM_MAGIC {
                pgrx::warning!(
                    "DsmReadAdapter::new: Invalid magic number in header: {:x}",
                    (*header).magic
                );
            }
        }
        Self {
            header,
            data,
            data_len,
        }
    }

    /// Checks if new data is available to be read.
    fn has_data(&self) -> bool {
        let write_pos = unsafe { (*self.header).write_pos.load(Ordering::Acquire) };
        let read_pos = unsafe { (*self.header).read_pos.load(Ordering::Acquire) };
        write_pos > read_pos
    }

    /// Checks if the writer has finished.
    fn is_finished(&self) -> bool {
        unsafe { (*self.header).finished.load(Ordering::Acquire) == 1 }
    }
}

impl std::io::Read for DsmReadAdapter {
    /// Reads raw bytes from the DSM ring buffer into the provided buffer.
    ///
    /// This implementation handles wrap-around reads and updates the `read_pos`.
    /// It returns `ErrorKind::WouldBlock` if no data is available.
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        unsafe {
            if (*self.header).magic != DSM_MAGIC {
                return Err(std::io::Error::other(format!(
                    "DsmReadAdapter::read: RingBufferHeader corruption (magic mismatch: {:x})",
                    (*self.header).magic
                )));
            }
        }

        if !self.has_data() {
            if self.is_finished() {
                return Ok(0);
            }
            return Err(std::io::Error::from(ErrorKind::WouldBlock));
        }

        let write_pos = unsafe { (*self.header).write_pos.load(Ordering::Acquire) };
        let read_pos = unsafe { (*self.header).read_pos.load(Ordering::Acquire) };

        let available = write_pos.wrapping_sub(read_pos) as usize;
        if available > self.data_len {
            return Err(std::io::Error::other(format!(
                "DsmReadAdapter::read: RingBufferHeader corruption (available {} > data_len {})",
                available, self.data_len
            )));
        }

        let to_read = std::cmp::min(buf.len(), available);
        let offset = (read_pos % self.data_len as u64) as usize;

        unsafe {
            if offset + to_read <= self.data_len {
                // Contiguous read.
                std::ptr::copy_nonoverlapping(self.data.add(offset), buf.as_mut_ptr(), to_read);
            } else {
                // Wrap-around read.
                let first_part = self.data_len - offset;
                let second_part = to_read - first_part;
                std::ptr::copy_nonoverlapping(self.data.add(offset), buf.as_mut_ptr(), first_part);
                std::ptr::copy_nonoverlapping(
                    self.data,
                    buf.as_mut_ptr().add(first_part),
                    second_part,
                );
            }
            // Update read position to free up space in the buffer.
            (*self.header)
                .read_pos
                .fetch_add(to_read as u64, Ordering::Release);
        }

        Ok(to_read)
    }
}

#[derive(Default)]
struct StreamState {
    queue: VecDeque<Vec<u8>>,
}

/// A demultiplexer for reading multiple logical streams from a single DSM ring buffer.
pub struct MultiplexedDsmReader {
    adapter: DsmReadAdapter,
    streams: HashMap<PhysicalStreamId, StreamState>,
    /// State for the current message being read from the physical DSM.
    partial_header: Vec<u8>,
    partial_payload: Option<(PhysicalStreamId, Vec<u8>, usize)>,
    /// Writer for the control channel (Reader -> Writer).
    control_writer: Option<DsmWriteAdapter>,
    /// Bridge for signaling/waiting.
    bridge: Arc<SignalBridge>,
    /// Index of the remote participant (writer).
    remote_id: ParticipantId,
}

unsafe impl Send for MultiplexedDsmReader {}
unsafe impl Sync for MultiplexedDsmReader {}

impl std::fmt::Debug for MultiplexedDsmReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiplexedDsmReader")
            .field("remote_id", &self.remote_id)
            .field("active_streams", &self.streams.len())
            .finish_non_exhaustive()
    }
}

impl MultiplexedDsmReader {
    /// Creates a new multiplexed reader.
    ///
    /// If `control_offset` is non-zero in the header, it also initializes a writer
    /// for the reverse control channel to send stream cancellation signals.
    pub fn new(
        base_ptr: *mut u8,
        data_capacity: usize,
        bridge: Arc<SignalBridge>,
        remote_id: ParticipantId,
    ) -> Self {
        let header = base_ptr as *mut TransportHeader;
        let data = unsafe { base_ptr.add(std::mem::size_of::<TransportHeader>()) };

        unsafe {
            if (*header).ring.magic != DSM_MAGIC {
                pgrx::warning!(
                    "MultiplexedDsmReader::new: Invalid magic number in header: {:x}",
                    (*header).ring.magic
                );
            }
        }

        let control_writer = unsafe {
            let offset = (*header).control_offset;
            if offset > 0 {
                // Sanity check control offset
                if offset < std::mem::size_of::<TransportHeader>() {
                    pgrx::warning!(
                        "MultiplexedDsmReader::new: Invalid control_offset {} (too small)",
                        offset
                    );
                }

                // Calculate pointer to control block
                let control_ptr = base_ptr.add(offset);
                let control_header = control_ptr as *mut RingBufferHeader;
                let control_data = control_ptr.add(std::mem::size_of::<RingBufferHeader>());
                let control_len = 65536; // Same constant as in writer

                Some(DsmWriteAdapter::new(
                    control_header,
                    control_data,
                    control_len,
                ))
            } else {
                None
            }
        };

        let adapter = unsafe { DsmReadAdapter::new(&mut (*header).ring, data, data_capacity) };

        Self {
            adapter,
            streams: HashMap::default(),
            partial_header: Vec::with_capacity(8),
            partial_payload: None,
            control_writer,
            bridge,
            remote_id,
        }
    }

    /// Reads from the physical DSM buffer and dispatches messages to stream-specific
    /// buffers until a message for the requested `stream_id` is found.
    ///
    /// This handles the demultiplexing of the physical pipe.
    /// Returns `Ok(Some(Vec<u8>))` for a message, `Ok(None)` for End-of-Stream (EOS),
    /// or `ErrorKind::WouldBlock` if no data is available in the physical buffer.
    pub(super) fn read_for_stream(
        &mut self,
        stream_id: PhysicalStreamId,
    ) -> std::io::Result<Option<Vec<u8>>> {
        let state = self.streams.entry(stream_id).or_default();
        if let Some(payload) = state.queue.pop_front() {
            if payload.is_empty() {
                // EOS marker from queue
                return Ok(None);
            }
            return Ok(Some(payload));
        }

        // Fallback to reading loop
        loop {
            // Read header if needed
            if self.partial_payload.is_none() {
                while self.partial_header.len() < 8 {
                    let mut byte = [0u8; 1];
                    match self.adapter.read(&mut byte) {
                        Ok(0) => return Ok(None), // EOF
                        Ok(_) => self.partial_header.push(byte[0]),
                        Err(e) => return Err(e),
                    }
                }

                let msg_stream_id = PhysicalStreamId(u32::from_le_bytes(
                    self.partial_header[0..4].try_into().unwrap(),
                ));
                let msg_len = u32::from_le_bytes(self.partial_header[4..8].try_into().unwrap());

                // Calculate total length including padding for 8-byte alignment
                let padding = (8 - (msg_len % 8)) % 8;
                let total_len = (msg_len + padding) as usize;

                self.partial_payload = Some((
                    msg_stream_id,
                    Vec::with_capacity(total_len),
                    msg_len as usize,
                ));
                self.partial_header.clear();
            }

            // Read payload
            if let Some((_, ref mut payload, msg_len)) = self.partial_payload {
                let padding = (8 - (msg_len % 8)) % 8;
                let total_len = msg_len + padding;

                while payload.len() < total_len {
                    let mut chunk = vec![0u8; total_len - payload.len()];
                    match self.adapter.read(&mut chunk) {
                        Ok(0) => return Ok(None), // Unexpected EOF
                        Ok(n) => payload.extend_from_slice(&chunk[..n]),
                        Err(e) => return Err(e),
                    }
                }

                // Dispatch completed message
                let (id, mut completed_payload, logical_len) = self.partial_payload.take().unwrap();

                // Truncate padding to restore original logical message
                completed_payload.truncate(logical_len);

                // Signal the writer that space is potentially available
                let _ = self.bridge.signal(self.remote_id);

                if id == stream_id {
                    if completed_payload.is_empty() {
                        return Ok(None); // EOS
                    }
                    return Ok(Some(completed_payload));
                } else {
                    self.streams
                        .entry(id)
                        .or_default()
                        .queue
                        .push_back(completed_payload);
                }
            }
        }
    }

    fn send_control_message(
        &mut self,
        msg_type: u8,
        stream_id: PhysicalStreamId,
    ) -> std::io::Result<()> {
        if let Some(writer) = &mut self.control_writer {
            if writer.available_space() < 5 {
                return Err(std::io::Error::from(ErrorKind::WouldBlock));
            }
            writer.write_all(&[msg_type])?;
            writer.write_all(&stream_id.to_le_bytes())?;
            // Signal the writer (remote_id) to check control messages
            let _ = self.bridge.signal(self.remote_id);
            Ok(())
        } else {
            Ok(())
        }
    }

    /// Signals the writer to start producing data for a stream.
    pub(super) fn start_stream(&mut self, stream_id: PhysicalStreamId) -> std::io::Result<()> {
        self.send_control_message(0, stream_id)
    }

    /// Cancels a stream by writing its ID to the control channel.
    pub(super) fn cancel_stream(&mut self, stream_id: PhysicalStreamId) -> std::io::Result<()> {
        self.send_control_message(1, stream_id)
    }

    /// Async version of `read_for_stream` that registers the current task's waker
    /// with the bridge if data is not yet available.
    pub(super) fn poll_read_for_stream(
        &mut self,
        stream_id: PhysicalStreamId,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<Option<Vec<u8>>>> {
        // Register waker FIRST to avoid race condition where data arrives
        // between read attempt and registration.
        self.bridge
            .register_waker(cx.waker().clone(), Some(self.remote_id));

        match self.read_for_stream(stream_id) {
            Ok(Some(msg)) => std::task::Poll::Ready(Ok(Some(msg))),
            Ok(None) => std::task::Poll::Ready(Ok(None)),
            Err(e) if e.kind() == ErrorKind::WouldBlock => std::task::Poll::Pending,
            Err(e) => std::task::Poll::Ready(Err(e)),
        }
    }
}

#[cfg(any(test, feature = "pg_test"))]
pub mod test_utils {
    use super::*;

    // Helper to create a dummy header and data buffer
    pub struct TestBuffer {
        _storage: Vec<u64>,
        pub base_ptr: *mut u8,
        pub capacity: usize,
    }

    impl TestBuffer {
        pub fn new(capacity: usize) -> Self {
            let control_capacity = 65536;
            let layout = TransportLayout::new(capacity, control_capacity);
            let total_size = layout.total_size();

            // Align size to 8 bytes (u64)
            let u64_count = total_size.div_ceil(8);
            let mut storage = vec![0u64; u64_count];

            let base_ptr = storage.as_mut_ptr() as *mut u8;

            unsafe {
                layout.init(base_ptr);
            }

            Self {
                _storage: storage,
                base_ptr,
                capacity,
            }
        }
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::test_utils::TestBuffer;
    use super::*;
    use std::io::Write;
    use std::sync::Arc;

    fn create_dummy_bridge() -> Arc<SignalBridge> {
        SignalBridge::new_dummy()
    }

    #[pgrx::pg_test]
    fn test_basic_read_write() {
        let buf = TestBuffer::new(1024);
        let bridge = create_dummy_bridge();
        let writer_mux = Arc::new(Mutex::new(MultiplexedDsmWriter::new(
            buf.base_ptr,
            buf.capacity,
            bridge.clone(),
            ParticipantId(1),
        )));
        let reader_mux = Arc::new(Mutex::new(MultiplexedDsmReader::new(
            buf.base_ptr,
            buf.capacity,
            bridge,
            ParticipantId(0),
        )));

        let mut writer = DsmStreamWriterAdapter::new(writer_mux, PhysicalStreamId(1));

        let payload = b"Hello World";
        writer.write_all(payload).unwrap();
        writer.flush().unwrap();

        let msg = reader_mux
            .lock()
            .read_for_stream(PhysicalStreamId(1))
            .unwrap()
            .unwrap();
        assert_eq!(msg, payload);
    }

    #[pgrx::pg_test]
    fn test_multiplexing() {
        let buf = TestBuffer::new(1024);
        let bridge = create_dummy_bridge();
        let writer_mux = Arc::new(Mutex::new(MultiplexedDsmWriter::new(
            buf.base_ptr,
            buf.capacity,
            bridge.clone(),
            ParticipantId(1),
        )));
        let reader_mux = Arc::new(Mutex::new(MultiplexedDsmReader::new(
            buf.base_ptr,
            buf.capacity,
            bridge,
            ParticipantId(0),
        )));

        let mut w1 = DsmStreamWriterAdapter::new(writer_mux.clone(), PhysicalStreamId(1));
        let mut w2 = DsmStreamWriterAdapter::new(writer_mux.clone(), PhysicalStreamId(2));

        w1.write_all(b"Stream1-A").unwrap();
        w1.flush().unwrap();
        w2.write_all(b"Stream2-A").unwrap();
        w2.flush().unwrap();
        w1.write_all(b"Stream1-B").unwrap();
        w1.flush().unwrap();

        // Read Stream 1
        let msg = reader_mux
            .lock()
            .read_for_stream(PhysicalStreamId(1))
            .unwrap()
            .unwrap();
        assert_eq!(msg, b"Stream1-A");

        // Read Stream 2
        let msg = reader_mux
            .lock()
            .read_for_stream(PhysicalStreamId(2))
            .unwrap()
            .unwrap();
        assert_eq!(msg, b"Stream2-A");

        // Read Stream 1 again
        let msg = reader_mux
            .lock()
            .read_for_stream(PhysicalStreamId(1))
            .unwrap()
            .unwrap();
        assert_eq!(msg, b"Stream1-B");
    }

    #[pgrx::pg_test]
    fn test_buffer_wrap_around() {
        // Create a small buffer to force wrap-around
        let buf = TestBuffer::new(32); // 32 bytes data capacity
        let bridge = create_dummy_bridge();
        let writer_mux = Arc::new(Mutex::new(MultiplexedDsmWriter::new(
            buf.base_ptr,
            buf.capacity,
            bridge.clone(),
            ParticipantId(1),
        )));
        let reader_mux = Arc::new(Mutex::new(MultiplexedDsmReader::new(
            buf.base_ptr,
            buf.capacity,
            bridge,
            ParticipantId(0),
        )));

        let mut writer = DsmStreamWriterAdapter::new(writer_mux, PhysicalStreamId(1));

        // Frame overhead is 8 bytes (4 stream_id + 4 len).
        // Max payload in one message is 32 - 8 = 24 bytes (but limited by available space check logic)

        // Write some initial data to advance the pointer
        let msg1 = vec![1u8; 10];
        writer.write_all(&msg1).unwrap();
        writer.flush().unwrap();

        // Read it back to clear space but advance read_pos
        let msg = reader_mux
            .lock()
            .read_for_stream(PhysicalStreamId(1))
            .unwrap()
            .unwrap();
        assert_eq!(msg.len(), 10);

        // Now write enough to wrap around
        // Buffer size 32.
        // Written: 8 + 10 = 18 bytes.
        // Available: 32 (since we read it).
        // Write Pos: 18.
        // We write 20 bytes payload. Total 28 bytes.
        // 18 + 28 = 46. 46 % 32 = 14.
        // Should wrap.

        let msg2 = vec![2u8; 20];
        writer.write_all(&msg2).unwrap();
        writer.flush().unwrap();

        let msg = reader_mux
            .lock()
            .read_for_stream(PhysicalStreamId(1))
            .unwrap()
            .unwrap();
        assert_eq!(msg, msg2);
    }

    #[pgrx::pg_test]
    fn test_buffer_full() {
        let buf = TestBuffer::new(50);
        let bridge = create_dummy_bridge();
        let writer_mux = Arc::new(Mutex::new(MultiplexedDsmWriter::new(
            buf.base_ptr,
            buf.capacity,
            bridge.clone(),
            ParticipantId(1),
        )));
        let reader_mux = Arc::new(Mutex::new(MultiplexedDsmReader::new(
            buf.base_ptr,
            buf.capacity,
            bridge,
            ParticipantId(0),
        )));

        let mut writer = DsmStreamWriterAdapter::new(writer_mux, PhysicalStreamId(1));

        // Frame overhead 8 bytes.
        // Write 40 bytes payload. Total 48.
        let msg1 = vec![1u8; 40];
        writer.write_all(&msg1).unwrap();
        writer.flush().unwrap(); // Success. 2 bytes left.

        // Try to write another message. even small one.
        // Overhead 8 bytes -> requires 8 bytes at least.
        let msg2 = vec![2u8; 1];
        writer.write_all(&msg2).unwrap();
        let res = writer.flush();

        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), ErrorKind::WouldBlock);

        // Read to free up space
        let msg = reader_mux
            .lock()
            .read_for_stream(PhysicalStreamId(1))
            .unwrap()
            .unwrap();
        assert!(!msg.is_empty());

        // Now flush should succeed (retry the write logic effectively)
        // Note: DsmStreamWriterAdapter buffers internally.
        // When flush failed, the buffer was not cleared.
        writer.flush().unwrap();

        let msg = reader_mux
            .lock()
            .read_for_stream(PhysicalStreamId(1))
            .unwrap()
            .unwrap();
        assert_eq!(msg, msg2);
    }

    #[pgrx::pg_test]
    fn test_message_too_large() {
        let buf = TestBuffer::new(100);
        let bridge = create_dummy_bridge();
        let writer_mux = Arc::new(Mutex::new(MultiplexedDsmWriter::new(
            buf.base_ptr,
            buf.capacity,
            bridge,
            ParticipantId(1),
        )));

        let mut writer = DsmStreamWriterAdapter::new(writer_mux, PhysicalStreamId(1));

        let msg = vec![0u8; 200];
        writer.write_all(&msg).unwrap();
        let res = writer.flush();

        assert!(res.is_err());
        // Custom error message check
        assert!(res
            .unwrap_err()
            .to_string()
            .contains("exceeds ring buffer capacity"));
    }

    #[pgrx::pg_test]
    fn test_finish_flag() {
        let buf = TestBuffer::new(1024);
        let bridge = create_dummy_bridge();
        let writer_mux = Arc::new(Mutex::new(MultiplexedDsmWriter::new(
            buf.base_ptr,
            buf.capacity,
            bridge.clone(),
            ParticipantId(1),
        )));
        let reader_mux = Arc::new(Mutex::new(MultiplexedDsmReader::new(
            buf.base_ptr,
            buf.capacity,
            bridge,
            ParticipantId(0),
        )));

        let mut writer = DsmStreamWriterAdapter::new(writer_mux.clone(), PhysicalStreamId(1));

        writer.write_all(b"data").unwrap();
        writer.flush().unwrap();

        writer_mux.lock().finish().unwrap();

        let msg = reader_mux
            .lock()
            .read_for_stream(PhysicalStreamId(1))
            .unwrap()
            .unwrap();
        assert_eq!(msg, b"data");

        // Next read should see EOF (None) because finished is set
        let msg = reader_mux
            .lock()
            .read_for_stream(PhysicalStreamId(1))
            .unwrap();
        assert!(msg.is_none());
    }

    #[pgrx::pg_test]
    fn test_signal_bridge() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async {
            let uuid = uuid::Uuid::new_v4();
            let bridge1 = SignalBridge::new(ParticipantId(1), uuid).await.unwrap();
            let _bridge2 = SignalBridge::new(ParticipantId(2), uuid).await.unwrap();

            // Bridge 1 signals Bridge 2
            // We can't verify reception easily without messing with the bridge internals or blocking,
            // but we can verify it doesn't error.
            bridge1.signal(ParticipantId(2)).unwrap();
            bridge1.signal(ParticipantId(1)).unwrap(); // Should be no-op or ok
        });
    }

    use crate::launch_parallel_process;
    use crate::parallel_worker::mqueue::MessageQueueSender;
    use crate::parallel_worker::{
        ParallelProcess, ParallelState, ParallelStateManager, ParallelStateType, ParallelWorker,
        ParallelWorkerNumber, WorkerStyle,
    };
    use crate::postgres::locks::Spinlock;
    use std::task::Poll;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct DsmStreamTestState {
        pub mutex: Spinlock,
        pub nlaunched: usize,
    }

    impl ParallelStateType for DsmStreamTestState {}

    impl DsmStreamTestState {
        pub fn set_launched_workers(&mut self, nlaunched: usize) {
            let _lock = self.mutex.acquire();
            self.nlaunched = nlaunched;
        }

        pub fn launched_workers(&mut self) -> usize {
            let _lock = self.mutex.acquire();
            self.nlaunched
        }
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct DsmStreamTestConfig {
        pub total_participants: usize,
        pub session_id: uuid::Bytes,
        pub buffer_size: usize,
        pub num_messages: usize,
        pub msg_size: usize,
    }

    impl ParallelStateType for DsmStreamTestConfig {}

    pub struct DsmStreamTestProcess {
        pub state: DsmStreamTestState,
        pub config: DsmStreamTestConfig,
        pub ring_buffer_region: Vec<u8>,
    }

    impl DsmStreamTestProcess {
        pub fn new(
            total_participants: usize,
            buffer_size: usize,
            num_messages: usize,
            msg_size: usize,
        ) -> Self {
            let session_id = uuid::Uuid::new_v4();
            let control_capacity = 65536;
            let layout = TransportLayout::new(buffer_size, control_capacity);
            let total_size = layout.total_size();

            let mut region = vec![0u8; total_size];
            unsafe {
                layout.init(region.as_mut_ptr());
            }

            Self {
                state: DsmStreamTestState {
                    mutex: Spinlock::default(),
                    nlaunched: 0,
                },
                config: DsmStreamTestConfig {
                    total_participants,
                    session_id: *session_id.as_bytes(),
                    buffer_size,
                    num_messages,
                    msg_size,
                },
                ring_buffer_region: region,
            }
        }
    }

    impl ParallelProcess for DsmStreamTestProcess {
        fn state_values(&self) -> Vec<&dyn ParallelState> {
            vec![&self.state, &self.config, &self.ring_buffer_region]
        }
    }

    pub struct DsmStreamTestWorker<'a> {
        pub state: &'a mut DsmStreamTestState,
        pub config: DsmStreamTestConfig,
        base_ptr: *mut u8,
    }

    impl ParallelWorker for DsmStreamTestWorker<'_> {
        fn new_parallel_worker(
            state_manager: ParallelStateManager,
            _worker_number: ParallelWorkerNumber,
        ) -> Self {
            let state = state_manager
                .object::<DsmStreamTestState>(0)
                .unwrap()
                .unwrap();
            let config = state_manager
                .object::<DsmStreamTestConfig>(1)
                .unwrap()
                .unwrap();

            // Buffer is at index 2
            let ring_buffer_slice = state_manager.slice::<u8>(2).unwrap().unwrap();
            let base_ptr = ring_buffer_slice.as_ptr() as *mut u8;

            Self {
                state,
                config: *config,
                base_ptr,
            }
        }

        fn run(
            self,
            _mq_sender: &MessageQueueSender,
            worker_number: ParallelWorkerNumber,
        ) -> anyhow::Result<()> {
            let participant_index = worker_number.to_participant_index(true); // Leader is 0
            let participant_id = ParticipantId(participant_index as u16);

            // Signal readiness
            let current = self.state.launched_workers();
            self.state.set_launched_workers(current + 1);

            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let session_id = uuid::Uuid::from_bytes(self.config.session_id);
            let bridge = runtime
                .block_on(SignalBridge::new(participant_id, session_id))
                .unwrap();
            let bridge = Arc::new(bridge);

            let reader_mux = Arc::new(Mutex::new(MultiplexedDsmReader::new(
                self.base_ptr,
                self.config.buffer_size,
                bridge.clone(),
                ParticipantId(0), // Remote is leader (0)
            )));

            let mut received_bytes = 0;
            let total_bytes = self.config.num_messages * self.config.msg_size;

            runtime.block_on(async {
                loop {
                    let res = futures::future::poll_fn(|cx| {
                        match reader_mux
                            .lock()
                            .poll_read_for_stream(PhysicalStreamId(1), cx)
                        {
                            Poll::Ready(Ok(Some(vec))) => Poll::Ready(Ok(Some(vec))),
                            Poll::Ready(Ok(None)) => Poll::Ready(Ok(None)),
                            Poll::Pending => Poll::Pending,
                            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                        }
                    })
                    .await;

                    match res {
                        Ok(None) => {
                            // If we finished reading everything, break.
                            if received_bytes >= total_bytes {
                                break;
                            }
                            break;
                        }
                        Ok(Some(vec)) => {
                            received_bytes += vec.len();
                            if received_bytes >= total_bytes {
                                break;
                            }
                        }
                        Err(e) => panic!("Read error: {}", e),
                    }
                }
            });

            assert_eq!(received_bytes, total_bytes);
            Ok(())
        }
    }

    #[pgrx::pg_test]
    fn test_concurrent_throughput_multi_process() {
        let total_participants = 2; // Leader + 1 Worker
        let buffer_size = 4096;
        let num_messages = 1000;
        let msg_size = 128;

        let process =
            DsmStreamTestProcess::new(total_participants, buffer_size, num_messages, msg_size);
        let session_id_bytes = process.config.session_id;

        let mut launched = launch_parallel_process!(
            DsmStreamTestProcess<DsmStreamTestWorker>,
            process,
            WorkerStyle::Query,
            1, // 1 worker
            16384
        )
        .expect("Failed to launch parallel process");

        let state = launched
            .state_manager_mut()
            .object::<DsmStreamTestState>(0)
            .unwrap()
            .unwrap();
        state.set_launched_workers(1); // Leader counts as 1

        // Wait for worker to launch
        while state.launched_workers() < total_participants {
            pgrx::check_for_interrupts!();
            std::thread::yield_now();
        }

        // Leader (Producer) Logic
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let session_id = uuid::Uuid::from_bytes(session_id_bytes);
        let bridge = runtime
            .block_on(SignalBridge::new(ParticipantId(0), session_id))
            .unwrap();
        let bridge = Arc::new(bridge);

        let ring_buffer_slice = launched.state_manager().slice::<u8>(2).unwrap().unwrap();
        let base_ptr = ring_buffer_slice.as_ptr() as *mut u8;

        let writer_mux = Arc::new(Mutex::new(MultiplexedDsmWriter::new(
            base_ptr,
            buffer_size,
            bridge.clone(),
            ParticipantId(1), // Remote is worker (1)
        )));
        let mut writer = DsmStreamWriterAdapter::new(writer_mux.clone(), PhysicalStreamId(1));

        let msg = vec![1u8; msg_size];

        runtime.block_on(async {
            for _ in 0..num_messages {
                writer.write_all(&msg).unwrap();
                futures::future::poll_fn(|cx| {
                    bridge.register_waker(cx.waker().clone(), None);
                    match writer.flush() {
                        Ok(_) => Poll::Ready(Ok(())),
                        Err(e) if e.kind() == ErrorKind::WouldBlock => Poll::Pending,
                        Err(e) => Poll::Ready(Err(e)),
                    }
                })
                .await
                .unwrap_or_else(|e| panic!("Producer flush failed: {}", e));
            }

            // Finish
            writer_mux.lock().finish().unwrap();
        });

        // Wait for worker to finish
        for _ in launched {}
    }
}

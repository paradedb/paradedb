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

//! Lock-free SPSC byte rings in Postgres DSM implementing Tokio `AsyncRead` and `AsyncWrite`.
//!
//! Topology for `1` leader (`proc 0`) and `W` workers (`1..=W`):
//! - Leader (`ClientProc(0)`) opens `W` client connections to `ServerProc(1..=W)`.
//! - Each worker `w` (`1..=W`) opens `W - 1` client connections to peer workers `s != w`, and
//!   accepts `W` server connections (`1` from `ClientProc(0)` and `W - 1` from `ClientProc(c != w)`).
//! - Each directed connection is a pair of SPSC byte rings (`client -> server` and
//!   `server -> client`), giving `W + W * (W - 1) = W^2` duplex connections = `2 * W^2` SPSC rings.
//! - Same-worker stage reads (`w == s`) short-circuit in memory via `datafusion-distributed`'s
//!   `LocalWorkerContext` (`InProcessWorkerChannel`) and never touch DSM.

use std::io;
use std::pin::Pin;
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering, fence};
use std::task::{Context, Poll};

use futures::task::AtomicWaker;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tonic::transport::server::Connected;

use crate::postgres::customscan::mpp::interrupt::cancel_pending;
use crate::postgres::customscan::mpp::wakeup::{WakeupDispatcher, WakeupSlot, wake_slot};

/// Magic number stored in [`DsmMeshHeader`] (`"PDB_DSM2"`).
const DSM_MESH_MAGIC: u64 = 0x5044_425F_4453_4D32;

/// Default byte capacity per SPSC ring (`64 KiB`) used in unit tests. Must be a power of two.
#[cfg(test)]
pub const DEFAULT_SPSC_RING_CAPACITY: usize = 64 * 1024;

/// Minimum byte capacity per SPSC ring (`16 KiB`, matching HTTP/2 minimum `MAX_FRAME_SIZE`).
pub const MIN_SPSC_RING_CAPACITY: usize = 16 * 1024;

/// Client process index in the MPP mesh (`0` for leader, `1..=W` for workers).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClientProc(pub u32);

/// Server process index in the MPP mesh (`1..=W` for workers; leader is never a server).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ServerProc(pub u32);

/// 64-byte header at the start of the DSM region (`align(8)` to match `pg_sys::MAXIMUM_ALIGNOF`).
#[repr(C, align(8))]
pub struct DsmMeshHeader {
    pub magic: u64,
    pub n_procs: u32,
    pub ring_capacity: u32,
    pub shutdown_flag: AtomicBool,
    _pad: [u8; 47],
}

const _: () = assert!(std::mem::size_of::<DsmMeshHeader>() == 64);

/// Producer-owned 64-byte cache line of an SPSC ring.
#[repr(C, align(8))]
pub struct RingProducerLine {
    pub write_pos: AtomicU64,
    pub sleeping: AtomicBool,
    pub closed: AtomicBool,
    _pad: [u8; 54],
}

const _: () = assert!(std::mem::size_of::<RingProducerLine>() == 64);

/// Consumer-owned 64-byte cache line of an SPSC ring.
#[repr(C, align(8))]
pub struct RingConsumerLine {
    pub read_pos: AtomicU64,
    pub sleeping: AtomicBool,
    pub closed: AtomicBool,
    _pad: [u8; 54],
}

const _: () = assert!(std::mem::size_of::<RingConsumerLine>() == 64);

/// Cache-line-separated lock-free SPSC byte-ring header in DSM.
#[repr(C, align(8))]
pub struct DsmRingHeader {
    pub producer: RingProducerLine,
    pub consumer: RingConsumerLine,
    pub capacity: u64,
    _pad: [u8; 56],
}

const _: () = assert!(std::mem::size_of::<DsmRingHeader>() == 192);

#[inline]
const fn align_up_64(val: usize) -> usize {
    (val + 63) & !63
}

/// Compute the connection index in `0..W^2` for `(client, server)`.
pub fn connection_index(
    n_procs: u32,
    client: ClientProc,
    server: ServerProc,
) -> Result<usize, String> {
    if n_procs < 2 {
        return Err(format!("mpp: invalid n_procs {n_procs}"));
    }
    let w = (n_procs - 1) as usize;
    let c = client.0 as usize;
    let s = server.0 as usize;
    if s == 0 || s > w {
        return Err(format!("mpp: server proc {s} out of range 1..={w}"));
    }
    if c > w || c == s {
        return Err(format!("mpp: invalid client proc {c} for server proc {s}"));
    }
    if c == 0 {
        Ok(s - 1)
    } else {
        let peer_ord = if s < c { s - 1 } else { s - 2 };
        Ok(w + (c - 1) * (w - 1) + peer_ord)
    }
}

/// Total number of SPSC rings (`2 * W^2`) for `n_procs = 1 + W`.
#[inline]
pub fn total_spsc_rings(n_procs: u32) -> Result<usize, String> {
    if n_procs < 2 {
        return Err(format!("mpp: n_procs must be >= 2, got {n_procs}"));
    }
    let w = (n_procs - 1) as usize;
    w.checked_mul(w)
        .and_then(|w2| w2.checked_mul(2))
        .ok_or_else(|| format!("mpp: ring count overflow for n_procs={n_procs}"))
}

#[inline]
fn wakeup_slots_offset() -> usize {
    std::mem::size_of::<DsmMeshHeader>()
}

#[inline]
fn rings_base_offset(n_procs: u32) -> usize {
    wakeup_slots_offset() + align_up_64((n_procs as usize) * std::mem::size_of::<AtomicU32>())
}

#[inline]
fn single_ring_stride(ring_capacity: usize) -> usize {
    std::mem::size_of::<DsmRingHeader>() + align_up_64(ring_capacity)
}

/// Compute the total DSM byte size required for `n_procs` participants and `ring_capacity` bytes
/// per SPSC ring.
pub fn dsm_region_bytes(n_procs: u32, ring_capacity: usize) -> Result<usize, String> {
    if !ring_capacity.is_power_of_two() || ring_capacity < 1024 {
        return Err(format!(
            "mpp: ring_capacity must be a power of two >= 1024, got {ring_capacity}"
        ));
    }
    let n_rings = total_spsc_rings(n_procs)?;
    let stride = single_ring_stride(ring_capacity);
    let rings_bytes = n_rings
        .checked_mul(stride)
        .ok_or_else(|| "mpp: DSM region size overflow".to_string())?;
    rings_base_offset(n_procs)
        .checked_add(rings_bytes)
        .ok_or_else(|| "mpp: DSM region size overflow".to_string())
}

/// Initialize the DSM mesh header, wakeup slot table, and all `2 * W^2` SPSC ring headers.
///
/// # Safety
/// `base` must point to an 8-byte-aligned DSM allocation of at least
/// `dsm_region_bytes(n_procs, ring_capacity)` bytes, before any concurrent reader/writer accesses
/// the rings.
pub unsafe fn init_dsm_mesh(
    base: *mut u8,
    n_procs: u32,
    ring_capacity: usize,
) -> Result<(), String> {
    let _total_bytes = dsm_region_bytes(n_procs, ring_capacity)?;
    if !(base as usize).is_multiple_of(std::mem::align_of::<AtomicU64>()) {
        return Err(format!("mpp: DSM base pointer {:p} is misaligned", base));
    }

    let hdr_ptr = base as *mut DsmMeshHeader;
    unsafe {
        ptr::write(
            hdr_ptr,
            DsmMeshHeader {
                magic: DSM_MESH_MAGIC,
                n_procs,
                ring_capacity: ring_capacity as u32,
                shutdown_flag: AtomicBool::new(false),
                _pad: [0u8; 47],
            },
        );
    }

    let slots_ptr = unsafe { base.add(wakeup_slots_offset()) as *mut AtomicU32 };
    for i in 0..(n_procs as usize) {
        unsafe {
            ptr::write(slots_ptr.add(i), AtomicU32::new(WakeupSlot::UNSET.0));
        }
    }

    let n_rings = total_spsc_rings(n_procs)?;
    let stride = single_ring_stride(ring_capacity);
    let rings_base = unsafe { base.add(rings_base_offset(n_procs)) };
    for r in 0..n_rings {
        let ring_hdr_ptr = unsafe { rings_base.add(r * stride) as *mut DsmRingHeader };
        unsafe {
            ptr::write(
                ring_hdr_ptr,
                DsmRingHeader {
                    producer: RingProducerLine {
                        write_pos: AtomicU64::new(0),
                        sleeping: AtomicBool::new(false),
                        closed: AtomicBool::new(false),
                        _pad: [0u8; 54],
                    },
                    consumer: RingConsumerLine {
                        read_pos: AtomicU64::new(0),
                        sleeping: AtomicBool::new(false),
                        closed: AtomicBool::new(false),
                        _pad: [0u8; 54],
                    },
                    capacity: ring_capacity as u64,
                    _pad: [0u8; 56],
                },
            );
        }
    }

    fence(Ordering::SeqCst);
    Ok(())
}

unsafe fn validate_mesh_header<'a>(base: *const u8) -> Result<&'a DsmMeshHeader, String> {
    let hdr = unsafe { &*(base as *const DsmMeshHeader) };
    if hdr.magic != DSM_MESH_MAGIC {
        return Err(format!(
            "mpp: invalid DSM mesh magic {:#x}, expected {:#x}",
            hdr.magic, DSM_MESH_MAGIC
        ));
    }
    Ok(hdr)
}

unsafe fn ring_ptrs(
    base: *mut u8,
    n_procs: u32,
    ring_capacity: usize,
    ring_idx: usize,
) -> (*const DsmRingHeader, *mut u8) {
    let stride = single_ring_stride(ring_capacity);
    let ring_base = unsafe { base.add(rings_base_offset(n_procs) + ring_idx * stride) };
    let hdr = ring_base as *const DsmRingHeader;
    let data = unsafe { ring_base.add(std::mem::size_of::<DsmRingHeader>()) };
    (hdr, data)
}

unsafe fn wakeup_slot_ptr(base: *const u8, proc_idx: u32) -> *const AtomicU32 {
    let slots_ptr = unsafe { base.add(wakeup_slots_offset()) as *const AtomicU32 };
    unsafe { slots_ptr.add(proc_idx as usize) }
}

/// Full-duplex `AsyncRead + AsyncWrite` stream backed by a pair of DSM SPSC byte rings.
pub struct DsmAsyncStream {
    tx_hdr: *const DsmRingHeader,
    tx_data: *mut u8,
    rx_hdr: *const DsmRingHeader,
    rx_data: *const u8,
    peer_wakeup_slot: *const AtomicU32,
    read_waker: Arc<AtomicWaker>,
    write_waker: Arc<AtomicWaker>,
    dsm_detached: Arc<AtomicBool>,
}

// SAFETY: `DsmAsyncStream` accesses shared memory exclusively through atomic operations and
// SPSC-partitioned ring slices.
unsafe impl Send for DsmAsyncStream {}
unsafe impl Sync for DsmAsyncStream {}

impl Connected for DsmAsyncStream {
    type ConnectInfo = ();

    fn connect_info(&self) -> Self::ConnectInfo {}
}

impl DsmAsyncStream {
    #[inline]
    fn wake_peer(&self) {
        if self.dsm_detached.load(Ordering::Acquire) {
            return;
        }
        // SAFETY: `peer_wakeup_slot` points into the DSM wakeup slot table for the lifetime of the
        // query.
        let raw_slot = unsafe { (*self.peer_wakeup_slot).load(Ordering::SeqCst) };
        wake_slot(WakeupSlot(raw_slot));
    }

    /// Close both the outgoing writer and incoming reader sides of this stream and wake the peer.
    pub fn close_rings(&self) {
        if self.dsm_detached.load(Ordering::Acquire) {
            return;
        }
        // SAFETY: `tx_hdr` and `rx_hdr` are valid while `dsm_detached` is false.
        unsafe {
            (*self.tx_hdr)
                .producer
                .closed
                .store(true, Ordering::Release);
            (*self.rx_hdr)
                .consumer
                .closed
                .store(true, Ordering::Release);
            fence(Ordering::SeqCst);
            let wake_rx = (*self.tx_hdr)
                .consumer
                .sleeping
                .swap(false, Ordering::SeqCst);
            let wake_tx = (*self.rx_hdr)
                .producer
                .sleeping
                .swap(false, Ordering::SeqCst);
            if wake_rx || wake_tx {
                self.wake_peer();
            }
        }
    }
}

// NOTE: We intentionally do NOT use `impl_safe_drop!` here because `close_rings` only flips
// atomic booleans in the DSM ring headers (guarded by `dsm_detached`) and writes a wakeup byte to
// a non-blocking `socketpair` without touching Postgres FFI, and `DsmAsyncStream` is exercised in
// standalone `#[test]` binaries that do not link Postgres globals.
impl Drop for DsmAsyncStream {
    fn drop(&mut self) {
        self.close_rings();
    }
}

impl AsyncRead for DsmAsyncStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if buf.remaining() == 0 || self.dsm_detached.load(Ordering::Acquire) {
            return Poll::Ready(Ok(()));
        }

        let hdr = unsafe { &*self.rx_hdr };
        let capacity = hdr.capacity as usize;
        let mask = hdr.capacity - 1;

        loop {
            if self.dsm_detached.load(Ordering::Acquire) {
                return Poll::Ready(Ok(()));
            }
            if cancel_pending() {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "mpp: query interrupted",
                )));
            }

            let write_pos = hdr.producer.write_pos.load(Ordering::Acquire);
            let read_pos = hdr.consumer.read_pos.load(Ordering::Relaxed);
            let avail = write_pos.wrapping_sub(read_pos) as usize;

            if avail > 0 {
                let to_read = avail.min(buf.remaining());
                let offset = (read_pos & mask) as usize;
                let first_len = to_read.min(capacity - offset);
                let second_len = to_read - first_len;

                // SAFETY: producer has published `write_pos` with `Release`, and `read_pos..read_pos+to_read`
                // will not be overwritten by producer until `read_pos` is updated below.
                unsafe {
                    let first_slice =
                        std::slice::from_raw_parts(self.rx_data.add(offset), first_len);
                    buf.put_slice(first_slice);
                    if second_len > 0 {
                        let second_slice = std::slice::from_raw_parts(self.rx_data, second_len);
                        buf.put_slice(second_slice);
                    }
                }

                hdr.consumer
                    .read_pos
                    .store(read_pos.wrapping_add(to_read as u64), Ordering::Release);
                fence(Ordering::SeqCst);
                if hdr.producer.sleeping.swap(false, Ordering::SeqCst) {
                    self.wake_peer();
                }
                return Poll::Ready(Ok(()));
            }

            if hdr.producer.closed.load(Ordering::Acquire) {
                // Re-check `write_pos` after observing `closed == true` so any bytes written right
                // before `closed` are drained before returning EOF.
                if hdr.producer.write_pos.load(Ordering::Acquire) != read_pos {
                    continue;
                }
                return Poll::Ready(Ok(()));
            }

            // Register our local waker before publishing `sleeping = true`.
            self.read_waker.register(cx.waker());
            hdr.consumer.sleeping.store(true, Ordering::SeqCst);

            let write_pos_recheck = hdr.producer.write_pos.load(Ordering::SeqCst);
            let closed_recheck = hdr.producer.closed.load(Ordering::SeqCst);
            if write_pos_recheck != read_pos || closed_recheck {
                hdr.consumer.sleeping.store(false, Ordering::Relaxed);
                continue;
            }

            return Poll::Pending;
        }
    }
}

impl AsyncWrite for DsmAsyncStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        src: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.poll_write_vectored(cx, &[io::IoSlice::new(src)])
    }

    fn is_write_vectored(&self) -> bool {
        true
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        if bufs.iter().all(|b| b.is_empty()) {
            return Poll::Ready(Ok(0));
        }
        if self.dsm_detached.load(Ordering::Acquire) {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "mpp: DSM segment already detached",
            )));
        }

        let hdr = unsafe { &*self.tx_hdr };
        let capacity = hdr.capacity as usize;
        let mask = hdr.capacity - 1;

        loop {
            if self.dsm_detached.load(Ordering::Acquire) {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "mpp: DSM segment already detached",
                )));
            }
            if cancel_pending() {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "mpp: query interrupted",
                )));
            }

            if hdr.consumer.closed.load(Ordering::Acquire)
                || hdr.producer.closed.load(Ordering::Relaxed)
            {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "mpp: peer closed DSM stream",
                )));
            }

            let read_pos = hdr.consumer.read_pos.load(Ordering::Acquire);
            let write_pos = hdr.producer.write_pos.load(Ordering::Relaxed);
            let used = write_pos.wrapping_sub(read_pos) as usize;
            let free = capacity.saturating_sub(used);

            if free > 0 {
                let mut remaining_free = free;
                let mut cur_write_pos = write_pos;
                let mut total_written = 0usize;

                // SAFETY: consumer has published `read_pos` with `Release`, and `write_pos..write_pos+total_written`
                // will not be read by consumer until `write_pos` is updated below.
                for slice in bufs {
                    if slice.is_empty() {
                        continue;
                    }
                    let to_write = remaining_free.min(slice.len());
                    if to_write == 0 {
                        break;
                    }
                    let offset = (cur_write_pos & mask) as usize;
                    let first_len = to_write.min(capacity - offset);
                    let second_len = to_write - first_len;

                    unsafe {
                        ptr::copy_nonoverlapping(
                            slice.as_ptr(),
                            self.tx_data.add(offset),
                            first_len,
                        );
                        if second_len > 0 {
                            ptr::copy_nonoverlapping(
                                slice.as_ptr().add(first_len),
                                self.tx_data,
                                second_len,
                            );
                        }
                    }

                    cur_write_pos = cur_write_pos.wrapping_add(to_write as u64);
                    total_written += to_write;
                    remaining_free -= to_write;
                    if remaining_free == 0 {
                        break;
                    }
                }

                hdr.producer
                    .write_pos
                    .store(cur_write_pos, Ordering::Release);
                fence(Ordering::SeqCst);
                if hdr.consumer.sleeping.swap(false, Ordering::SeqCst) {
                    self.wake_peer();
                }
                return Poll::Ready(Ok(total_written));
            }

            // Register our local waker before publishing `sleeping = true`.
            self.write_waker.register(cx.waker());
            hdr.producer.sleeping.store(true, Ordering::SeqCst);

            let read_pos_recheck = hdr.consumer.read_pos.load(Ordering::SeqCst);
            let closed_recheck = hdr.consumer.closed.load(Ordering::SeqCst);
            if read_pos_recheck != read_pos || closed_recheck {
                hdr.producer.sleeping.store(false, Ordering::Relaxed);
                continue;
            }

            return Poll::Pending;
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.dsm_detached.load(Ordering::Acquire) {
            return Poll::Ready(Ok(()));
        }
        let hdr = unsafe { &*self.tx_hdr };
        hdr.producer.closed.store(true, Ordering::Release);
        fence(Ordering::SeqCst);
        if hdr.consumer.sleeping.swap(false, Ordering::SeqCst) {
            self.wake_peer();
        }
        Poll::Ready(Ok(()))
    }
}

/// Leader handle used at query teardown to signal workers to shut down their Tonic servers and
/// close all leader-side rings before unmapping DSM.
pub struct DsmLeaderShutdownHandle {
    base: *mut u8,
    n_procs: u32,
    ring_capacity: usize,
    dsm_detached: Arc<AtomicBool>,
}

// SAFETY: Only touches atomic fields in the DSM header and ring headers.
unsafe impl Send for DsmLeaderShutdownHandle {}
unsafe impl Sync for DsmLeaderShutdownHandle {}

impl DsmLeaderShutdownHandle {
    pub fn detached_flag(&self) -> &Arc<AtomicBool> {
        &self.dsm_detached
    }

    /// Set `shutdown_flag = true`, mark all `2 * W^2` rings in the mesh closed so every worker's
    /// HTTP/2 server/client tasks observe immediate EOF, and wake all workers (`1..n_procs`).
    pub fn signal_shutdown(&self) {
        if self.dsm_detached.load(Ordering::Acquire) {
            return;
        }
        unsafe {
            let hdr = &*(self.base as *const DsmMeshHeader);
            hdr.shutdown_flag.store(true, Ordering::SeqCst);

            if let Ok(n_rings) = total_spsc_rings(self.n_procs) {
                for r in 0..n_rings {
                    let (ring_hdr, _) = ring_ptrs(self.base, self.n_procs, self.ring_capacity, r);
                    (*ring_hdr).producer.closed.store(true, Ordering::Release);
                    (*ring_hdr).consumer.closed.store(true, Ordering::Release);
                }
            }

            fence(Ordering::SeqCst);

            for s in 1..self.n_procs {
                let slot_ptr = wakeup_slot_ptr(self.base, s);
                let raw_slot = (*slot_ptr).load(Ordering::SeqCst);
                wake_slot(WakeupSlot(raw_slot));
            }
        }
    }
}

/// Watcher polled by a worker's `serve_with_incoming_shutdown` to exit when the leader signals
/// query teardown or when a Postgres interrupt/cancel or postmaster death occurs.
pub struct DsmWorkerShutdownWatcher {
    base: *const u8,
    waker: Arc<AtomicWaker>,
    dsm_detached: Arc<AtomicBool>,
}

// SAFETY: `base` points to the shared DSM header and is only accessed via atomics.
unsafe impl Send for DsmWorkerShutdownWatcher {}
unsafe impl Sync for DsmWorkerShutdownWatcher {}

impl DsmWorkerShutdownWatcher {
    pub fn is_shutdown(&self) -> bool {
        if self.dsm_detached.load(Ordering::Acquire) || cancel_pending() {
            return true;
        }
        let hdr = unsafe { &*(self.base as *const DsmMeshHeader) };
        hdr.shutdown_flag.load(Ordering::Acquire)
    }

    pub async fn wait(self) {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(20));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let flag_fut = std::future::poll_fn(|cx| {
            if self.is_shutdown() {
                return Poll::Ready(());
            }
            self.waker.register(cx.waker());
            if self.is_shutdown() {
                return Poll::Ready(());
            }
            Poll::Pending
        });

        let poll_timer = async {
            loop {
                interval.tick().await;
                if self.is_shutdown() {
                    break;
                }
            }
        };

        tokio::select! {
            _ = flag_fut => {}
            _ = poll_timer => {}
        }
    }
}

/// Initialize the leader's wakeup slot in an already-initialized DSM mesh and construct the `W`
/// client streams (`ClientProc(0) -> ServerProc(1..=W)`) plus the [`DsmLeaderShutdownHandle`].
///
/// # Safety
/// `base` must point to a DSM mesh initialized via [`init_dsm_mesh`].
pub unsafe fn build_leader_streams(
    base: *mut u8,
    dispatcher: &WakeupDispatcher,
) -> Result<(Vec<(ServerProc, DsmAsyncStream)>, DsmLeaderShutdownHandle), String> {
    let hdr = unsafe { validate_mesh_header(base)? };
    let n_procs = hdr.n_procs;
    let ring_capacity = hdr.ring_capacity as usize;
    let dsm_detached = Arc::new(AtomicBool::new(false));

    let leader_slot_ptr = unsafe { wakeup_slot_ptr(base, 0) };
    unsafe {
        (*leader_slot_ptr).store(dispatcher.slot().0, Ordering::SeqCst);
    }

    let mut client_streams = Vec::with_capacity((n_procs - 1) as usize);
    for s in 1..n_procs {
        let server = ServerProc(s);
        let conn_idx = connection_index(n_procs, ClientProc(0), server)?;
        let (tx_hdr, tx_data) = unsafe { ring_ptrs(base, n_procs, ring_capacity, 2 * conn_idx) };
        let (rx_hdr, rx_data) =
            unsafe { ring_ptrs(base, n_procs, ring_capacity, 2 * conn_idx + 1) };
        let peer_wakeup_slot = unsafe { wakeup_slot_ptr(base, s) };
        let stream = DsmAsyncStream {
            tx_hdr,
            tx_data,
            rx_hdr,
            rx_data: rx_data as *const u8,
            peer_wakeup_slot,
            read_waker: dispatcher.register_waker(),
            write_waker: dispatcher.register_waker(),
            dsm_detached: Arc::clone(&dsm_detached),
        };
        client_streams.push((server, stream));
    }

    let shutdown = DsmLeaderShutdownHandle {
        base,
        n_procs,
        ring_capacity,
        dsm_detached,
    };
    Ok((client_streams, shutdown))
}

/// Bundles a worker's peer client streams, incoming server streams, and shutdown watcher.
pub struct WorkerDsmStreams {
    pub client_streams: Vec<(ServerProc, DsmAsyncStream)>,
    pub incoming_server_streams: Vec<DsmAsyncStream>,
    pub shutdown_watcher: DsmWorkerShutdownWatcher,
}

/// Register `worker_proc` (`1..=W`) in the DSM wakeup slot table and construct its `W - 1`
/// peer client streams, `W` incoming server streams, and [`DsmWorkerShutdownWatcher`].
///
/// # Safety
/// `base` must point to a DSM mesh initialized via [`init_dsm_mesh`], and `worker_proc` must be
/// in `1..n_procs`.
pub unsafe fn build_worker_streams(
    base: *mut u8,
    worker_proc: ServerProc,
    dispatcher: &WakeupDispatcher,
) -> Result<WorkerDsmStreams, String> {
    let hdr = unsafe { validate_mesh_header(base)? };
    let n_procs = hdr.n_procs;
    let ring_capacity = hdr.ring_capacity as usize;
    let w = worker_proc.0;
    if w == 0 || w >= n_procs {
        return Err(format!("mpp: worker_proc {w} out of range 1..{n_procs}"));
    }
    let dsm_detached = Arc::new(AtomicBool::new(false));

    // Store our wakeup slot BEFORE constructing any streams or registering `sleeping = true`
    // flags, and before checking `shutdown_flag` in `shutdown_watcher`.
    let my_slot_ptr = unsafe { wakeup_slot_ptr(base, w) };
    unsafe {
        (*my_slot_ptr).store(dispatcher.slot().0, Ordering::SeqCst);
    }

    let mut client_streams = Vec::with_capacity((n_procs - 2) as usize);
    for s in 1..n_procs {
        if s == w {
            continue;
        }
        let server = ServerProc(s);
        let conn_idx = connection_index(n_procs, ClientProc(w), server)?;
        let (tx_hdr, tx_data) = unsafe { ring_ptrs(base, n_procs, ring_capacity, 2 * conn_idx) };
        let (rx_hdr, rx_data) =
            unsafe { ring_ptrs(base, n_procs, ring_capacity, 2 * conn_idx + 1) };
        let peer_wakeup_slot = unsafe { wakeup_slot_ptr(base, s) };
        client_streams.push((
            server,
            DsmAsyncStream {
                tx_hdr,
                tx_data,
                rx_hdr,
                rx_data: rx_data as *const u8,
                peer_wakeup_slot,
                read_waker: dispatcher.register_waker(),
                write_waker: dispatcher.register_waker(),
                dsm_detached: Arc::clone(&dsm_detached),
            },
        ));
    }

    let mut incoming_server_streams = Vec::with_capacity((n_procs - 1) as usize);
    for c in 0..n_procs {
        if c == w {
            continue;
        }
        let conn_idx = connection_index(n_procs, ClientProc(c), worker_proc)?;
        // For the server side of `(ClientProc(c), ServerProc(w))`:
        // - ring `2 * conn_idx` is `client -> server` (so server's `rx`)
        // - ring `2 * conn_idx + 1` is `server -> client` (so server's `tx`)
        let (rx_hdr, rx_data) = unsafe { ring_ptrs(base, n_procs, ring_capacity, 2 * conn_idx) };
        let (tx_hdr, tx_data) =
            unsafe { ring_ptrs(base, n_procs, ring_capacity, 2 * conn_idx + 1) };
        let peer_wakeup_slot = unsafe { wakeup_slot_ptr(base, c) };
        incoming_server_streams.push(DsmAsyncStream {
            tx_hdr,
            tx_data,
            rx_hdr,
            rx_data: rx_data as *const u8,
            peer_wakeup_slot,
            read_waker: dispatcher.register_waker(),
            write_waker: dispatcher.register_waker(),
            dsm_detached: Arc::clone(&dsm_detached),
        });
    }

    let shutdown_watcher = DsmWorkerShutdownWatcher {
        base: base as *const u8,
        waker: dispatcher.register_waker(),
        dsm_detached,
    };

    Ok(WorkerDsmStreams {
        client_streams,
        incoming_server_streams,
        shutdown_watcher,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[repr(C, align(64))]
    struct AlignedBuf([u8; 64]);

    fn alloc_aligned_region(bytes: usize) -> Vec<AlignedBuf> {
        let blocks = bytes.div_ceil(64);
        vec![AlignedBuf([0u8; 64]); blocks]
    }

    impl Clone for AlignedBuf {
        fn clone(&self) -> Self {
            Self(self.0)
        }
    }

    #[tokio::test]
    async fn spsc_stream_transfers_payload_larger_than_ring_capacity() {
        let n_procs = 3; // leader + 2 workers
        let ring_cap = 1024; // 1 KiB ring to force many wrap-arounds
        let total = dsm_region_bytes(n_procs, ring_cap).unwrap();
        let mut storage = alloc_aligned_region(total);
        let base = storage.as_mut_ptr() as *mut u8;

        unsafe {
            init_dsm_mesh(base, n_procs, ring_cap).unwrap();
        }

        let handle = tokio::runtime::Handle::current();
        let mut leader_disp = WakeupDispatcher::new(WakeupSlot(0));
        leader_disp.start(&handle).unwrap();
        let mut w1_disp = WakeupDispatcher::new(WakeupSlot(1));
        w1_disp.start(&handle).unwrap();

        let (mut leader_clients, shutdown) =
            unsafe { build_leader_streams(base, &leader_disp).unwrap() };
        let w1_streams = unsafe { build_worker_streams(base, ServerProc(1), &w1_disp).unwrap() };

        let (_, mut c_to_w1) = leader_clients.remove(0);
        let mut w1_from_c = w1_streams
            .incoming_server_streams
            .into_iter()
            .next()
            .unwrap();

        // Send 256 KiB (256x the 1 KiB ring capacity) in both directions concurrently.
        let payload: Vec<u8> = (0..(256 * 1024)).map(|i| (i % 251) as u8).collect();
        let expected = payload.clone();

        let writer = tokio::spawn(async move {
            c_to_w1.write_all(&payload).await.unwrap();
            let mut reply = [0u8; 5];
            c_to_w1.read_exact(&mut reply).await.unwrap();
            assert_eq!(&reply, b"ack!!");
            c_to_w1.shutdown().await.unwrap();
        });

        let reader = tokio::spawn(async move {
            let mut received = vec![0u8; expected.len()];
            w1_from_c.read_exact(&mut received).await.unwrap();
            assert_eq!(received, expected);
            w1_from_c.write_all(b"ack!!").await.unwrap();
            let mut eof = Vec::new();
            w1_from_c.read_to_end(&mut eof).await.unwrap();
            assert!(eof.is_empty());
        });

        writer.await.unwrap();
        reader.await.unwrap();

        shutdown.signal_shutdown();
        w1_streams.shutdown_watcher.wait().await;
    }
}

# Feedback: Data Transport & Signaling

## Summary

The data transport layer correctly implements backpressure and signaling using a hybrid approach of Shared Memory ring buffers and Unix Domain Sockets (`SocketBridge`).

## Strengths

1.  **Backpressure:** The chain from `DsmSharedMemoryWriter` (`WouldBlock`) -> `DsmExchangeExec` (`yield_now`) -> `DsmStream` (`Poll::Pending`) works correctly. It ensures that a slow consumer eventually halts the producer without unbounded buffering.
2.  **Signaling:** `SocketBridge` provides a reliable, async-friendly wake-up mechanism that works around the limitations of standard Postgres latches in a `tokio` environment.

## Issues & Recommendations

### 1. Busy-Wait in Producer Task

**Observation:**
In `exchange.rs`, `DsmExchangeExec::producer_task` handles `WouldBlock` by calling `tokio::task::yield_now()`.

```rust
Err(datafusion::error::DataFusionError::IoError(ref msg)) if msg.kind() == std::io::ErrorKind::WouldBlock => {
    blocked = true;
    tokio::task::yield_now().await;
}
```

**Impact:**
This is a **busy loop** (spin wait). While `yield_now()` prevents the task from monopolizing the thread (allowing the Control Service to run), it still burns 100% CPU on that core while waiting for the consumer to read data.
**Recommendation:**
_Short-term:_ This is acceptable for a first iteration given the complexity of adding waker support to the shared memory ring buffer (which requires cross-process waker signalling).
_Long-term:_ Implement a "Space Available" signal. The Reader could signal the Writer (via `SocketBridge`) when it advances `read_pos`. The Writer could wait on `SocketBridge` notifications instead of spinning.

### 4. macOS Socket Path Safety (New)

**Observation:**
The current implementation hardcodes `/tmp/pdb_mpp_{uuid}_{idx}.sock`.
**Risk:**

1.  **Sandbox Issues:** Hardcoded `/tmp` is unreliable on macOS due to Seatbelt/App Sandbox virtualization.
2.  **Path Length:** Switching to `std::env::temp_dir()` (recommended) risks exceeding the 104-byte limit for `sockaddr_un` on macOS.
    _ `temp_dir()`: ~70 chars
    _ `pdb_mpp_{uuid}_{idx}.sock`: ~57 chars \* Total: ~127 chars > 104 chars.
    **Recommendation:**
    Implement the **Shortened Path Strategy**:
3.  Use `std::env::temp_dir()` for correctness.
4.  Shorten the filename to fit: `mpp_{short_uuid}_{idx}.sock`.
    - Use the first 8-12 characters of the UUID.
    - Validate the total length is < 104 bytes at runtime.

### 5. Failure Propagation & Timeouts (New)

**Observation:**
If a producer crashes, the consumer hangs.
**Recommendation:**
Implement a **Timeout** in `DsmStream::poll_next`:

1.  Track `last_read_time`.
2.  If `poll_next` returns Pending and `elapsed > timeout` (e.g., 30s), return `Err("Producer Timeout")`.
3.  This prevents "zombie" queries if a background worker panics without signaling.

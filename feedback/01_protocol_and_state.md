# Feedback: Core Protocol & State Management

## Summary

The transition to a "Lazy Request" / RPC-style model is a significant improvement that directly addresses the "Stream Reuse" bug. By decoupling the logical plan from the execution lifecycle, you ensure that every stream has a unique, ephemeral existence tied to a specific request.

## Strengths

1.  **Framing Protocol:** The simple `[stream_id: u32][len: u32][payload]` framing is efficient and sufficient for the byte-stream nature of the transport.
2.  **Control Channel:** Implementing a reverse control channel (`Reader -> Writer`) is the correct architectural choice for a "Pull" based system. It allows the consumer to drive the execution, which is essential for accurate backpressure and resource management (e.g., `LIMIT` clauses).
3.  **StreamRegistry:** The registry provides a clean separation between "Planning" (registering `StreamSource`) and "Execution" (spawning `producer_task`). This idempotency is key to preventing race conditions.

## Issues & Recommendations

### 1. StartStream Payload Ambiguity

**Observation:**
In `dsm_stream.rs`, `ControlMessage::StartStream(u32)` carries a `u32`. In `exchange.rs`, `trigger_stream` uses this ID to look up the source.
**Verification:**
I verified in `dsm_transfer.rs` that the Reader constructs this ID as the **Physical Stream ID** (`(logical << 16) | participant_index`). This matches the key used in `StreamRegistry`.
**Recommendation:**
Add a comment to `ControlMessage::StartStream` explicitly stating that the payload is the **Physical Stream ID**, not the Logical ID. This avoids future confusion.

### 2. Private Method Usage in `MultiplexedDsmWriter`

**Observation:**
The method `check_cancellations` in `MultiplexedDsmWriter` is private and documented as "problematic" because it consumes control messages that the Control Service might miss.
**Recommendation:**
As noted in your code comments, you should strictly enforce that **only** the Control Service (via `read_control_messages`) consumes messages. `MultiplexedDsmWriter::write_message` should rely on an external signal (e.g., a shared `AtomicBool` or just the `cancelled_streams` set updated publicly) rather than trying to peek/read the stream itself.
_Action:_ Remove `check_cancellations` calls from `write_message` and `close_stream`. Let the Control Service handle all message consumption and update the writer's state.

### 3. SocketBridge Task Leak (Minor)

**Observation:**
`SocketBridge::spawn_acceptor` spawns a loop that accepts connections. Inside, it spawns a task for each connection. If a peer connects but never sends data (and doesn't close), that task hangs in `stream.read()`.
**Impact:**
Low. The number of connections is bounded by `total_participants` (which is small).
**Recommendation:**
Consider adding a keep-alive or timeout, but strictly speaking, it's not critical for the query lifecycle as these tasks are bound to the `Runtime` which is dropped at the end of the query.

### 4. `StreamRegistry` Cleanup

**Observation:**
`cancel_triggered_stream` removes entries from `running_tasks` and `abort_handles`, but `sources` remain populated.
**Impact:**
Memory usage grows with plan size \* parallelism. For normal queries, this is negligible.
**Recommendation:**
Acceptable as is. Explicit cleanup of `sources` isn't necessary since the entire `DsmMesh` is dropped at the end of the query.

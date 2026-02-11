# DSM Stream Overhaul: RPC-Style Lazy Execution

## Objective

Transition the distributed JoinScan execution from an "Eager Push" model (where workers immediately execute the full plan) to a "Lazy Request" model (where workers wait for explicit `StartStream` requests from consumers before spawning specific sub-plans).

This overhaul eliminates `ProducerState` (and its associated race conditions) and solves the "Stream Reuse" bug by strictly enforcing a unique lifecycle for every stream connection.

## Core Concepts

### 1. Unique Physical Stream ID

We distinguish between the **Logical Stream** (the shuffle operation in the plan) and the **Physical Stream** (the point-to-point connection).

- **Logical Stream ID:** Assigned by `EnforceDsmShuffle` optimizer rule.
- **Physical Stream ID:** `(Logical ID << 16) | Sender Index`.
  - Example: Logical Stream 1 from Worker 3 has Physical ID `0x00010003`.

### 2. Control Channel Protocol

The existing "Control Channel" (reverse direction from Reader -> Writer) is upgraded to support an RPC-like protocol.

```rust
enum ControlMessage {
    /// Request the writer to start producing data for this stream.
    /// Payload: [u8; 4] = Physical Stream ID
    StartStream(u32),

    /// Request the writer to stop producing.
    /// Payload: [u8; 4] = Physical Stream ID
    CancelStream(u32),
}
```

### 3. Stream Registry (The "Listener")

Each process (Leader and Workers) maintains a `StreamRegistry`.

- **Registration Phase:** During startup (plan deserialization), we traverse the physical plan using `collect_dsm_writers`. Every `DsmWriterExec` encountered is "registered" but not executed. We store its input plan and configuration.
- **Listening Phase:** The process runs a `Control Service` loop (spawned on `tokio::task::LocalSet`) that polls the Control Channels of all its `MultiplexedDsmWriter`s.
- **Execution Phase:** When a `StartStream(id)` message arrives:
  1.  The Control Service calls `trigger_stream(id)`.
  2.  It looks up the registered plan for `id`.
  3.  It spawns a background Tokio task to execute that sub-plan.
  4.  The task writes data to the DSM buffer.

## Detailed Architecture

### A. `pg_search/src/postgres/customscan/joinscan/dsm_stream.rs`

- Implements the `ControlMessage` protocol.
- `MultiplexedDsmWriter` exposes `read_control_messages` to drain requests.
- `MultiplexedDsmReader` exposes `start_stream` to send requests.

### B. `pg_search/src/postgres/customscan/joinscan/exchange.rs`

- **StreamRegistry:** Replaces `ProducerState`. Manages the mapping of `PhysicalStreamId` -> `Plan`.
- **Control Service:** A background task spawned via `spawn_control_service`. It integrates with `SocketBridge` to sleep until woken by an incoming control message (via `bridge.signal()`).
- **DsmWriterExec:** Now a passive data structure. `execute()` is a no-op. Its logic is moved to `producer_task` which is invoked by the Registry.
- **DsmReaderExec:** In `execute()`, it creates a lazy stream. When the stream is first polled (in `dsm_transfer.rs`), it sends `StartStream` to the producer.

### C. `pg_search/src/postgres/customscan/joinscan/dsm_transfer.rs`

- **Lazy Start:** `dsm_shared_memory_reader` sends the `StartStream` control message and signals the `SocketBridge` to wake the writer process before beginning to read data.

### D. `pg_search/src/postgres/customscan/joinscan/parallel.rs` & `mod.rs`

- **Plan Traversal:** Both `JoinWorker::run` (Workers) and `exec_custom_scan` (Leader) call `collect_dsm_writers` to populate the registry.
- **Service Lifecycle:** Both spawn the `Control Service` on the `LocalSet` before beginning main execution.

## Analogy: RPC Tree

The system can be visualized as a tree of RPC calls.

1.  **Leader** executes the Root Plan.
2.  When it hits a `DsmReaderExec`, it makes an "RPC Call" (`StartStream`) to a specific Worker node (or itself).
3.  The **Worker** receives the call. It looks up the "Procedure" (the sub-plan corresponding to that stream) and executes it.
4.  If that sub-plan contains more `DsmReaderExec` nodes, the Worker recursively makes more RPC calls to other nodes.
5.  Data flows back up the tree as the response stream.

This "Pull-Based" / "Lazy" execution ensures that only necessary work is performed and that the lifecycle of every data stream is strictly coupled to the request that initiated it.

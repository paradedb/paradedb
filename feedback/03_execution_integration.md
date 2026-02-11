# Feedback: Execution Integration

## Summary

The integration with DataFusion and the Postgres worker lifecycle is well-architected. The use of `LocalSet` to multiplex the Control Service and the Execution Plan on a single thread is a clever solution to Postgres's single-threaded extension requirement.

## Strengths

1.  **Orchestration:** `JoinWorker::run` correctly initializes the environment (`SocketBridge`, `DsmMesh`) before starting execution. The barrier synchronization (waiting for sockets) ensures the mesh is fully connected before data starts flowing.
2.  **Plan Traversal:** `collect_dsm_writers` allows for arbitrary placement of `DsmWriterExec` nodes within the plan, supporting complex distributed plans beyond simple shuffles.
3.  **Gather Mode Support:** The logic handles `ExchangeMode::Gather` correctly by having workers execute a "dummy" plan that waits for the local producer task (triggered by the leader) to complete.

## Issues & Recommendations

### 1. Signal Loopback Safety

**Observation:**
In `Gather` mode, the Leader (Node 0) consumes data from itself. This involves:

1. Main Task: `DsmReaderExec` reading from DSM.
2. Background Task: `DsmWriterExec` writing to DSM.
   Both run on the same `LocalSet`.
   **Analysis:**
   This relies on `DsmWriterExec` yielding (`yield_now`) when the buffer is full. If it blocked, the Main Task would never run to read data, causing a deadlock.
   **Verification:**
   The code in `dsm_transfer.rs` correctly returns `WouldBlock`, and `exchange.rs` correctly yields.
   **Recommendation:**
   Add a specific comment in `producer_task` explaining _why_ `yield_now()` is critical there (to prevent loopback deadlocks).

### 2. Runtime Shutdown Order

**Observation:**
In `end_custom_scan`, resources are dropped.

```rust
state.custom_state_mut().unified_stream = None;
state.custom_state_mut().local_set = None;
state.custom_state_mut().runtime = None;
```

**Analysis:**
Dropping `unified_stream` first ensures that `DsmStream` destructors run while the Runtime and Mesh are still valid. This allows `CancelStream` messages to be sent.
**Status:**
Correct.

### 3. Slicing Logic

**Observation:**
`PgSearchTableProvider` uses `crate::parallel_worker::chunk_range` to slice segments.
**Assumption:**
This assumes that `reader.segment_readers()` returns segments in a deterministic order across all workers.
**Recommendation:**
Verify that `segment_readers()` is deterministic. Tantivy's segment ID ordering usually is, but explicit sorting by Segment ID might be safer if not already guaranteed.

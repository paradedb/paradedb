# In-Depth Review Plan: DSM Stream Overhaul

## Objective

To verify the correctness, safety, and maintainability of the DSM Stream Overhaul, specifically ensuring the new RPC-style lazy execution eliminates the "Stream Reuse" bug and race conditions without introducing deadlocks or performance regressions.

## Phase 1: Core Protocol & State Management (The "Brain")

- **Focus:** The new control plane logic.
- **Files:**
  - `pg_search/src/postgres/customscan/joinscan/dsm_stream.rs`
  - `pg_search/src/postgres/customscan/joinscan/exchange.rs`
- **Review Goals:**
  - Verify the `StartStream`/`CancelStream` serialization and handling.
  - Audit `StreamRegistry` for thread-safety and lifecycle management (registration vs. execution).
  - Analyze the `Control Service` loop for potential stalls or blocking operations.
  - **Output:** `feedback/01_protocol_and_state.md`

## Phase 2: Data Transport & Signaling (The "Heart")

- **Focus:** The actual movement of data and the signaling mechanism between processes.
- **Files:**
  - `pg_search/src/postgres/customscan/joinscan/dsm_transfer.rs`
  - `pg_search/src/postgres/customscan/joinscan/scan_state.rs`
- **Review Goals:**
  - Check `dsm_shared_memory_reader` for correct integration with the lazy start signal.
  - Verify `SocketBridge` interactions: ensure signals are never lost and wake-ups are reliable.
  - Review backpressure handling: does the reader correctly throttle the writer?
  - **Output:** `feedback/02_data_transport.md`

## Phase 3: Execution Integration & Plan Traversal (The "Body")

- **Focus:** How the new system hooks into DataFusion and the Postgres worker lifecycle.
- **Files:**
  - `pg_search/src/postgres/customscan/joinscan/parallel.rs`
  - `pg_search/src/postgres/customscan/joinscan/mod.rs`
  - `pg_search/src/scan/execution_plan.rs`
  - `pg_search/src/scan/table_provider.rs`
- **Review Goals:**
  - Validate `collect_dsm_writers`: Does it correctly traverse arbitrary DataFusion plans?
  - Check the initialization sequence in `exec_custom_scan` and `JoinWorker::run`.
  - Ensure `LocalSet` usage is correct for the `Control Service` alongside the main execution.
  - **Output:** `feedback/03_execution_integration.md`

## Phase 4: Concurrency, Safety, & Testing (The "Immune System")

- **Focus:** Edge cases, error propagation, and verification.
- **Files:**
  - `pg_search/src/postgres/customscan/joinscan/dsm_test.rs`
  - `tests/pg_regress/sql/join_custom_scan_parallel.sql`
- **Review Goals:**
  - **Deadlock Analysis:** Look for circular dependencies between the Control Channel and Data Channel.
  - **Error Handling:** If a "lazy" stream fails, does the error propagate correctly to the leader?
  - **Test Coverage:** Do tests explicitly verify the "laziness" (i.e., streams _not_ starting if not polled)?
  - **Output:** `feedback/04_safety_and_tests.md`

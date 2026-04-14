# MPP Plan Partitioning for JoinScan

**Issue:** [#4152](https://github.com/paradedb/paradedb/issues/4152)
**PRs:** [#4768](https://github.com/paradedb/paradedb/pull/4768) (infrastructure), [#4773](https://github.com/paradedb/paradedb/pull/4773) (wiring + tests)
**Based on:** Draft [#4184](https://github.com/paradedb/paradedb/pull/4184) by Stu Hood
**GUC:** `paradedb.enable_mpp_join` (default: off)

---

## Problem

JoinScan's current parallel execution uses a **Broadcast Join** strategy: the largest table is partitioned across workers by Tantivy segments, while every other table is fully replicated. This means non-partitioned tables are scanned N times (once per worker). With 8 workers joining two large tables, ~87% of total row reads are redundant.

Additionally, for SEMI/ANTI join correctness the preserved side _must_ be the partitioned side, which sometimes forces partitioning the smaller table — the opposite of what's optimal.

## Solution

Replace the broadcast model with an **MPP (Massively Parallel Processing)** model where all tables are hash-partitioned across workers and data is shuffled via shared memory.

```text
Before (Broadcast Join):                After (MPP Plan Partitioning):
  Worker 1: A[seg1-3] JOIN ALL(B)         Worker 1: A[hash1] JOIN B[hash1]
  Worker 2: A[seg4-6] JOIN ALL(B)         Worker 2: A[hash2] JOIN B[hash2]
  Worker 3: A[seg7-9] JOIN ALL(B)         Worker 3: A[hash3] JOIN B[hash3]
  B scanned 3x total                      Every row scanned exactly once
```

## Architecture

### Process Model

The leader acts as a scheduler. Workers act as RPC servers.

1. **Leader** builds the full physical plan, serializes it, and broadcasts to all workers via shared memory control channels.
2. Each **worker** deserializes the plan, registers all `DsmExchangeExec` nodes as "procedures" in a local stream registry, and parks — waiting for `StartStream` RPC messages.
3. When the leader executes its plan and hits a `DsmExchangeExec` (consumer side), it sends `StartStream` to the producing worker. That worker wakes up, executes the sub-plan, and streams Arrow IPC batches back through a shared memory ring buffer.

### Key Components

| Component                 | File(s)                  | Role                                                                                                                       |
| ------------------------- | ------------------------ | -------------------------------------------------------------------------------------------------------------------------- |
| **Transport**             | `joinscan/transport/`    | Shared memory ring buffers, Arrow IPC, Unix Domain Socket signaling, multiplexed streams                                   |
| **DsmExchangeExec**       | `joinscan/exchange.rs`   | DataFusion `ExecutionPlan` node at shuffle boundaries. Consumer sends `StartStream`, producer writes batches to DSM.       |
| **EnforceDsmShuffle**     | `joinscan/exchange.rs`   | Physical optimizer rule. Replaces DataFusion's in-process `RepartitionExec` with cross-process `DsmExchangeExec`.          |
| **DsmSanitizeExec**       | `joinscan/sanitize.rs`   | Deep-copy wrapper. Prevents deadlocks when blocking operators (Sort, HashJoin) pin shared memory buffers.                  |
| **Parallel coordination** | `joinscan/parallel.rs`   | Worker lifecycle via `parallel_worker` framework. Plan broadcast, transport mesh setup, control service.                   |
| **Physical codec**        | `scan/codec.rs`          | Serializes/deserializes `DsmExchangeExec` and `DsmSanitizeExec` for cross-process plan distribution.                       |
| **Segment slicing**       | `scan/table_provider.rs` | Each participant reads `MppParticipantConfig` from the session and opens only its slice of segments via `chunk_range()`.   |
| **Session profile**       | `joinscan/scan_state.rs` | `SessionContextProfile::JoinMpp` sets `target_partitions=N`, forces hash-join repartitioning, injects MPP optimizer rules. |

### Data Flow

```text
Planning:
  JoinCSClause → LogicalPlan → serialize (stored in CustomScan.custom_private)

Execution (exec_mpp_path):
  1. launch_join_workers()        → spawn N workers, init ring buffers
  2. deserialize LogicalPlan      → with JoinMpp session context
  3. build_physical_plan()        → optimizer injects DsmExchangeExec + DsmSanitizeExec
  4. serialize PhysicalPlan       → via PgSearchPhysicalCodec
  5. broadcast to workers         → control channel: BroadcastPlan message
  6. register_dsm_mesh() on leader
  7. leader deserialize round-trip → registers leader's stream sources
  8. spawn_control_service()      → listens for StartStream from workers
  9. plan.execute(0, ctx)         → leader runs partition 0, gathers results

Workers (JoinWorker::run):
  1. wait for broadcast plan
  2. deserialize (registers DsmExchangeExec as stream sources)
  3. spawn_control_service()
  4. park thread — wake on StartStream, execute sub-plan, write to DSM
```

### Correctness Constraints

The MPP model is correct for **INNER**, **LEFT OUTER** (left partitioned), **SEMI**, and **ANTI** joins. It is **incorrect** for RIGHT OUTER and FULL OUTER joins (unmatched rows from the replicated side would be emitted by every worker).

### Deadlock Prevention

When a blocking operator (e.g., `SortExec`, `HashJoinExec`) sits downstream of a `DsmExchangeExec`, it can pin shared memory buffers indefinitely — blocking the producer from writing new data. The `EnforceSanitization` optimizer rule detects these patterns and wraps the exchange with `DsmSanitizeExec`, which deep-copies incoming batches to heap memory before passing them downstream.

## Gating

The MPP path is behind `SET paradedb.enable_mpp_join = on` (default: off). When the GUC is off or `planned_workers = 0`, the existing broadcast-join path is used unchanged.

## Known Limitations & Future Work

- **Ring buffer overhead**: ~30% of runtime in the draft PR was spent writing to shared memory. Late materialization (already on main) should reduce this since fewer bytes cross process boundaries.
- **Dynamic filter passthrough**: `SegmentedTopKExec`'s dynamic threshold doesn't currently propagate across process boundaries. This limits pruning effectiveness for Top-K queries.
- **Range partitioning**: Hash partitioning shuffles all data. For sorted joins, range partitioning could keep most data local to a single worker.
- **Tokio blocking**: The leader blocks the tokio thread while reading DSM. A `spawn_blocking`-style helper for Postgres API calls would improve throughput.

## Testing

- **Regression test** (`mpp_join.sql`): GUC toggle, inner join with search predicate, multi-predicate cross-table queries, result comparison between MPP on/off.
- **qgen**: `PgGucs.enable_mpp_join` field enables property-based testing with randomized join queries.
- **Unit test** (`pg_test_dsm_gather_execution`): End-to-end test launching a real parallel worker, executing a `DsmExchangeExec` gather, and verifying both leader and worker produce expected results.

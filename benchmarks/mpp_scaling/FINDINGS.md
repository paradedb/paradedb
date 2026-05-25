# MPP linear-scaling baseline (1M parent / 5M child)

Run on PG18 / pg_search 0.23.5 (chain head, on top of #5136). 3 runs per config, median reported.

## Numbers

| Query                         | baseline (MPP off) | mpp_n2 |     mpp_n4 | mpp_n8 |
| ----------------------------- | -----------------: | -----: | ---------: | -----: |
| Q1 join_count (scalar)        |               1196 |   1206 |       1188 |   1186 |
| Q2 join_low_gb (10 groups)    |               1393 |   1183 | **10,410** |  4,575 |
| Q3 join_high_gb (100K groups) |               1457 |   1359 | **11,294** |  4,857 |
| Q4 join_multi_agg (4 scalars) |               1343 |   1319 |       1314 |   1305 |

All times in milliseconds. **baseline** = `paradedb.enable_mpp=off` + `max_parallel_workers_per_gather=0`. **mpp_nN** = N producer workers + leader.

## Observations

Two distinct bottlenecks:

1. **Scalar-aggregate queries (Q1, Q4) are flat across all configs.** Adding MPP workers does nothing. The join probe phase saturates the single-process DataFusion plan; gathering scalar aggregates to the leader doesn't help. Need to identify what's actually on the critical path (probably HashJoinExec build, which is serial).

2. **GROUP BY queries (Q2, Q3) explode at N=4.** At N=2 they're slightly faster than baseline (~1.18×). At N=4 they're 8× slower than baseline. At N=8 they partially recover but still 3-3.5× slower. This is the pgrx single-thread FFI bottleneck documented in `project_mpp_g7mt.md`: every fragment future runs on one current-thread Tokio runtime, and at N≥4 the post-agg shuffle saturates that single thread.

Q1/Q4 vs Q2/Q3 differ in one thing: Q2/Q3 have GROUP BY, which generates a post-agg shuffle mesh. Q1/Q4 only need to gather scalars to the leader (one mesh edge per worker).

## Reproduction

```bash
# Setup data (1M parent / 5M child rows; 130MB / 608MB)
psql -h localhost -p 28818 -U $USER -d pg_search \
  -v rows=1000000 \
  -f benchmarks/mpp_scaling/setup.sql

# Run benchmark
./benchmarks/mpp_scaling/run.sh 3
```

GUC configs tested are encoded in `run.sh`. Both `enable_aggregate_custom_scan` and `enable_join_custom_scan` are forced on; parallel costs are zeroed so the planner picks parallel even on small data.

## What "linear scaling" requires

Target: at N workers, runtime ≈ baseline / N. Today:

- Q1, Q4: must drop from 1190ms to 600ms (N=2), 300ms (N=4), 150ms (N=8). Currently flat.
- Q2, Q3: must drop from 1393/1457ms to ~350/365ms (N=4) and ~175/180ms (N=8). Currently 8-10× slower.

Per memory `project_mpp_g7mt.md`, the path forward is the G7-MT plan:

1. **GUC snapshot** (4h): stash all reads at backend on `exec_mpp_worker` entry.
2. **FFI service + relay channel** (1-2 days): pin Postgres-FFI service task to backend thread via Tokio LocalSet. Compute futures send `ShmMqTrySend` / `ShmMqTryRecv` ops over a channel; the service replays them on the backend thread. Migrate `MppSender::send_batch_traced_*` to async.
3. **Multi-thread Tokio in worker** (4h): switch `exec_mpp_worker`'s runtime from `new_current_thread()` to `new_multi_thread().worker_threads(N)`.
4. **Test + bench** (1 day): re-run pgrx regress + this benchmark at N=2/4/8.

Total: ~3 days of focused work. Tracked in `project_mpp_g7mt.md`.

## Per-query investigation needed before fixing

- Q1, Q4 (flat): profile to find why parallel workers don't help. Hypothesis: HashJoinExec build phase is serial; probe phase is parallel but build dominates at this scale.
- Q2, Q3 (regress): profile the post-agg shuffle mesh. Hypothesis: G7-MT pgrx FFI bottleneck is the root cause.

## Update 2026-05-24 — build_filters cache landed, Q2/Q3 cliff is gone

After wiring `paradedb.mpp_trace` to emit per-seat shuffle stats, the trace revealed `send_wait_ms = 0` everywhere — the pgrx-FFI / shm_mq backpressure hypothesis was wrong at this scale. Stage 2 (child probe) had `pull_ms ≈ 9000ms` while the actual data transfer was instant.

`sample` on a parallel worker pinned 91% of CPU to `pg_search::scan::execution_plan::build_filters` → `DynamicFilterPhysicalExpr::current()` → `remap_children()` → `transform_up()` rebuilding the dynamic filter's `CaseExpr` tree on **every batch poll**. With N=4 producers the tree grew complex enough to dominate scan wall time.

Fix: cache the `Vec<PreFilter>` across iterations, rebuild only when `datafusion::physical_expr_common::physical_expr::snapshot_generation` changes on any filter (the dynamic filter increments its generation counter on every `update()`).

New numbers at 1M / 5M:

| Query                         | baseline (MPP off) | mpp_n2 | mpp_n4 | mpp_n8 |
| ----------------------------- | -----------------: | -----: | -----: | -----: |
| Q1 join_count (scalar)        |               1179 |   1182 |   1213 |   1188 |
| Q2 join_low_gb (10 groups)    |               1389 |   1168 |   1229 |   1500 |
| Q3 join_high_gb (100K groups) |               1441 |   1287 |   1340 |   1609 |
| Q4 join_multi_agg (4 scalars) |               1296 |   1306 |   1303 |   1306 |

The 8× regression at N=4 is gone (Q2: 10,900ms → 1,229ms). Now no query regresses. But at 1M scale baseline (MPP off) is already using the optimal `HashJoinExec mode=CollectLeft` plan that needs no shuffle, so MPP and baseline are near-tied.

## 5M parent / 25M child numbers

Same harness scaled up so the parent build side is large enough that baseline can't use CollectLeft and falls back to the same Partitioned path MPP uses:

| Query                         | baseline | mpp_n2 |   mpp_n4 | mpp_n8 |
| ----------------------------- | -------: | -----: | -------: | -----: |
| Q1 join_count (scalar)        |     7106 |   7105 |     7600 |   6537 |
| Q2 join_low_gb (10 groups)    |     7242 |   6024 | **5137** |   6456 |
| Q3 join_high_gb (100K groups) |     7786 |   6750 | **5796** |   6772 |
| Q4 join_multi_agg (4 scalars) |     7215 |   6580 |     6454 |   6448 |

Speedups at producers=4: Q1 0.94×, Q2 1.41×, Q3 1.34×, Q4 1.12×. Real wins on GROUP BY shapes. Beyond producers=4 the N²-edge mesh overhead takes back what was won.

## What linear scaling would actually need

The remaining gap to linear is per-row shuffle overhead: Arrow IPC encode → shm_mq → decode adds ~1-2 µs/row, and shuffles run N² edges per stage. At producers > 3 the added edge cost exceeds the per-producer gain.

Three architectural options:

1. **Zero-copy in-process shuffle** _(investigated, deferred)_. Post-fix per-seat trace showed `send_ms = 0.4ms` out of `wall_ms = 1097ms` — encode/transport is 0.04% of stage wall. Not the bottleneck at this scale. Revisit only when shuffle bytes dominate (much larger batches or wider rows).
2. **Mesh-edge reduction (N² → N)** _(landed as opt-in GUC)_. `paradedb.mpp_target_partitions` pins the inner fanout instead of scaling with `mpp_worker_count`. Default 0 keeps historical behavior. At `paradedb.mpp_target_partitions = 2` on 5M/25M, producers=4 medians (5 runs):

   | Query           | baseline | n4 target=0 | n4 target=2 |                                     win |
   | --------------- | -------: | ----------: | ----------: | --------------------------------------: |
   | Q2 low-card GB  |     6893 |        5312 |    **4814** | 1.10× over t=0; **1.43×** over baseline |
   | Q3 high-card GB |     7288 |        5806 |    **5349** |     1.09× over t=0; 1.36× over baseline |

   At producers=8 the low-card win holds but high-card regresses ~15% — so this stays an opt-in knob per workload rather than a default change.

3. **Multi-thread compute within a producer**. G7-MT plan per memory `project_mpp_g7mt.md`. Switches `exec_mpp_worker`'s current-thread Tokio runtime to multi-thread + an FFI relay so producers actually parallelize internal partitions. Limited by pgrx 0.18 single-thread FFI invariant; 3 days of work to break that. Still the highest-leverage path remaining.

   **Phase 1 (GUC snapshot) landed.** `MppRuntimeGucs` (`pg_search/src/postgres/customscan/mpp/runtime_gucs.rs`) is a `ConfigExtension` that snapshots every GUC compute paths read on the backend thread at `exec_mpp_worker_impl` entry and stashes it on the per-query `SessionConfig`. Converted callsites: `paradedb.mpp_trace`, `paradedb.mpp_debug`, `paradedb.dynamic_filter_batch_size`, the five `hash_join_inlist_pushdown_*` / `term_set_*` knobs in `pre_filter.rs` (bundled into `InListPushdownConfig`). Each callsite falls back to the live reader when no snapshot is installed so the non-MPP serial DataFusion path keeps working.

   **Phase 2 (FFI relay scaffold + LocalSet) landed.** `ffi_relay.rs` defines `FfiOp` / `FfiRelay` / `FfiRelayService`: a producer-side handle, an mpsc-backed service, and a `oneshot`-based round-trip for `ShmMqTrySend`. `run_mpp_worker` now stands the relay + service up before the dispatcher `block_on`, spawning the service on a `tokio::task::LocalSet` and driving the dispatcher under `local_set.run_until`. Today nothing routes through the relay so this is a no-op overhead path: the service stays idle for the whole query, the dispatcher behaves identically, regress still green, bench unchanged. The plumbing proves the LocalSet pattern works under `current_thread` tokio.

   **Phase 3 (remaining):** (a) plumb the relay handle into `MppSender` so per-fragment senders know about it, (b) modify the spin loop in `MppSender::send_with_scratch` to route through `relay.shm_mq_try_send(...)` when attached, (c) convert the still-direct compute-path GUC reads (`segmented_topk_rule::optimize`, `is_columnar_sort_enabled` callsites, `aggregatescan/exec.rs`'s `max_term_agg_buckets` / `adjust_work_mem`), (d) flip `host.rs:77` from `new_current_thread()` to `new_multi_thread().worker_threads(N)`. Each step is independently testable; (d) is the only one with deadlock blast radius and should be the last commit before re-bench.

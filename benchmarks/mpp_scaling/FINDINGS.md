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

   **Phase 2 (FFI relay scaffold + LocalSet) landed.** `ffi_relay.rs` defines `FfiOp` / `FfiRelay` / `FfiRelayService`: a producer-side handle, an mpsc-backed service, and a `oneshot`-based round-trip for `ShmMqTrySend`. `run_mpp_worker` stands the relay + service up before the dispatcher `block_on`, spawning the service on a `tokio::task::LocalSet` and driving the dispatcher under `local_set.run_until`. The plumbing proves the LocalSet pattern works under `current_thread` tokio.

   **Phase 3a (plumb FfiRelay through MppSender) landed.** `with_ffi_relay` builder, `clone_with_header` forwards the handle, `run_mpp_worker` attaches the relay to each per-fragment sender.

   **Phase 3b (spin loop routes through relay when GUC is on) landed.** New `FfiOp` variants `TryDrainPass` + `CheckForInterrupts`, three `spin_*` helpers on `MppSender` that branch on `self.ffi_relay.as_ref()`. New GUC `paradedb.mpp_use_ffi_relay` (default off) gates attachment. Bench at producers=4 (5M/25M): relay=off Q2 4842, relay=on Q2 6232 (+29% overhead from channel-hop + oneshot round-trip on the same OS thread — that's the fundamental cost the multi-thread runtime would have to amortize over).

   **Phase 3 (remaining):** (c) convert the still-direct compute-path GUC reads (verified mostly not required — `joinscan/scan_state.rs` is in builders, `aggregatescan/exec.rs` is non-MPP path), (d) restructure to dedicated current_thread driver runtime + multi_thread compute runtime (the reviewer-flagged BLOCKER B1 must ship with this).

## Path C validation (2026-05-25)

Sweeps 3 build widths × 6 configs to decide between continuing G7-MT (Path A) and pivoting to a different lever (Path B). `paradedb.mpp_target_partitions=2` throughout, median of 3 runs, 5M parent / 25M child.

### Scalar count — all configs flat

| Query                         | baseline | pg_n4 | pg_n8 | mpp_n4 | mpp_n8 | mpp_n12 |
| ----------------------------- | -------: | ----: | ----: | -----: | -----: | ------: |
| q_narrow_count (1.5M parents) |     5887 |  5857 |  5837 |   5873 |   5859 |    5862 |
| q_medium_count (3.5M parents) |     7970 |  7997 |  7999 |   8004 |   8011 |    8064 |
| q_wide_count (5M parents)     |     7897 |  8417 |  7871 |   7898 |   7855 |    7830 |

**Nothing helps.** PG-native parallel doesn't help (DataFusion's plan ignores PG workers without MPP). MPP doesn't help either — even at producers=12. The bottleneck for scalar count is somewhere downstream of parallelism — most likely the serial Final aggregate gathering scalars from N producers, or HashJoinExec's serial build phase.

### GROUP BY — MPP wins, peaking at n=4

| Query                      | baseline | pg_n4 | pg_n8 |   mpp_n4 | mpp_n8 | mpp_n12 |
| -------------------------- | -------: | ----: | ----: | -------: | -----: | ------: |
| q_narrow_gb (1.5M parents) |     6853 |  6857 |  6857 | **4488** |   5021 |    6565 |
| q_medium_gb (3.5M parents) |    10323 | 10306 | 10309 | **6821** |   8511 |   11887 |
| q_wide_gb (5M parents)     |    11300 | 11211 | 11198 | **8531** |  11128 |   17640 |

Speedups vs baseline at the mpp_n4 sweet spot:

- q_narrow_gb: **1.53×**
- q_medium_gb: **1.51×**
- q_wide_gb: **1.32×**

Above n=4 the N²-edge mesh overhead grows faster than the parallelism gain — at n=12 the wide-build GROUP BY is 1.56× _slower_ than baseline. n=16 crashes the server (DSM overflow).

### Decision

The data points away from G7-MT (Path A):

1. **MPP DSM mesh scales O(N²); parallelism scales O(N).** The DSM allocates `n_procs² × mpp_queue_size` queues regardless of `mpp_target_partitions` (the GUC only affects DataFusion's inner fanout, not the DSM mesh — every proc attaches to every other proc). At producers=12 → 169 queues × 64 MiB ≈ 10.8 GiB DSM allocated up front. Crossover at n=4 today. Multi-thread compute per producer (G7-MT) doesn't shrink the mesh — it would have to come with a much-reduced shuffle topology to be net positive at higher N.

2. **Scalar count is flat under every parallelism config — but not for the original "serial Final agg" reason.** Baseline's plan is `HashJoinExec mode=CollectLeft` wrapped under DataFusion's `CooperativeExec`, which already parallelizes the BM25 scan across segments inside one DataFusion task via the parallel segment-claim mechanism. The scan IS using multiple cores even at "baseline serial". MPP's `mode=Partitioned` adds shuffle overhead without unlocking new parallelism — the per-producer scan still runs on a single-thread Tokio runtime, just over fewer segments per producer. Net is a wash. G7-MT wouldn't touch this either; the lever is somewhere else (force CollectLeft when the build fits, or skip MPP entirely on small builds).

3. **The 1.5× GROUP BY speedup at n=4 is what's deployable today.** Further G7-MT investment would optimistically add another 1.5× (multi-thread within producer), but only if the +25% relay overhead can be removed first — and tantivy buffer reads through the PG buffer manager remain an unmeasured risk for the multi-thread path.

### Known bugs uncovered by the bench

- **n=16 crashes the server** with the existing half-MPP fallback bug (see memory `project_mpp_aggregatescan_half_mpp_crash.md`). `compute_dsm_layout` returns `Err` when `n_procs² × mpp_queue_size > MPP_DSM_MAX_BYTES = 16 GiB` (17² × 64 MiB = 18.5 GiB). `estimate_dsm_custom_scan` catches the error, warns, and returns 0 — but `maybe_flip_mpp_parallel` has already committed `set_parallel(nworkers)` at planning time. PG launches workers that attach an empty DSM region and segfault on the missing `MPP_DSM_MAGIC`. Fix: gate `maybe_flip_mpp_parallel` on a cheap upfront DSM-size check, or move the fallback into the existing plan-time `init_mpp_strip_dynamic_filters` hook.

- **PG-native parallel does nothing** (`pg_parallel_n4`/`n8` ≈ `baseline_serial`) because AggregateScan's custom path leaves `parallel_workers=0` in the non-MPP branch. PG never launches workers for the path; it's not "allocates them and discards" — they're never requested. Worth documenting in the AggregateScan README but not a separate bug.

### Next: Path B — pivot to a different lever

The 1.5× at producers=4 on GROUP BY shapes is the achievable speedup with the current architecture. Two levers to explore:

- **For scalar shapes (q\_\*\_count flat):** the planner picks `mode=Partitioned` even when CollectLeft would win. Detect "parent build side fits in memory" and either force CollectLeft in MPP (already supported via the fork's `with_distributed_broadcast_joins` flag, but apparently not firing in our test cases) or skip MPP entirely and let the serial CollectLeft + `CooperativeExec` path run.

- **For GROUP BY shapes at n>4:** the DSM mesh is the wall. Lowering `mpp_queue_size` per query could let n=8 or n=12 actually scale — worth a 1h spike with `paradedb.mpp_queue_size = '8MB'` to measure where DSM allocation latency caps the parallelism gain.

- **G7-MT scaffolding stays parked, not deleted.** The MppRuntimeGucs snapshot, FfiRelay, and spin-loop routing all sit behind `paradedb.mpp_use_ffi_relay=off` by default. If a future investigation pinpoints a scalar-shape bottleneck that needs shared per-producer state across threads, G7-MT is the right substrate.

### Harness sensitivity

3 runs / config gives ±2-5% noise on GROUP BY (the 1.5× wins and 1.5× n=12 regressions are both ≫ 3σ). On scalar count the spread is ±0.4% — at noise floor, not enough resolution to detect a 5% effect that future G7-MT might deliver. Bump to ≥10 runs for n>8 configs when re-benching.

### Verified during review

- **Dynamic filter pushdown** (`try_dynamic_filter_pushdown` in `pg_search/src/scan/pre_filter.rs:824`) DOES fire in MPP plans — the bench-time plan annotation shows `PgSearchScan: ... dynamic_filters=1` on the child scan, and the earlier 2026-05-24 trace showed only ~3.1M child rows reach the consumer (vs 25M total in the table), confirming the TermSet filter cuts upstream. So Stu's "push dynamic filter to Tantivy across the wire" lever is already on; no Path B work needed there.
- **Reviewer hypothesis that broadcast-build was redundantly rebuilding per producer** turned out not to apply to these queries — the MPP plan uses `mode=Partitioned`, not `CollectLeft`/Broadcast. `BroadcastBuildSideOneTaskEstimator` only caps actual `BroadcastExec` nodes, which the Partitioned path never inserts. The real scalar-count flatness cause is point 2 above (baseline's segment-claim scan already uses multiple cores).

# MS MARCO vector CI

The MS MARCO workflow restores the existing 113,520,750-row PostgreSQL 18 heap
backup `20261102-224214F`, builds pg_search from the requested revision, and
creates an eight-segment cosine index with a 0.32 training sample ratio and
quantized vector routing. It uses the same runner and Docker setup as HN.

Apply `benchmark-cohere-msmarco` to a PR to benchmark its head. The label is
removed when the run starts so it can be applied again. This large dataset is
opt-in on PRs; the generic `benchmarks` and Cohere labels do not start it.
Scheduled runs execute daily on main. After merging, a manual main run can
establish the first baseline immediately:

```sh
gh workflow run benchmark-pg_search-benchmarker-msmarco.yml \
  --repo paradedb/paradedb --ref main
```

The optional `commit` input benchmarks another revision using the workflow's
benchmark code. Backfills and PR runs do not update the main baseline.

The pinned pdbench revision includes the configured-neighbor-count recall fix
needed for top-100. It uses 100 held-out query vectors from the published 1,677
fixtures and independently sweeps top-10 and top-100 to R90/R95/R99. The current
configuration uses integer `paradedb.vector_cluster_max_probe` work budgets
from 1 to 100000, starting at 100, with adaptive probing enabled. SOAR replication
from #7315 is not part of this baseline.

The workflow reuses Cohere's handoff helpers and k6 script. Plan checks require
cosine TopK execution inside ParadeDB with the correct neighbor count. Each
reached operating point runs with one VU, 10 seconds of discarded warmup, and
50 seconds of measurement. Unreached targets remain explicit in the results.

The shared publisher records latency percentiles, QPS, buffer hits, build time,
index size, parallel workers, and peak container memory with and without reclaimable file cache
under `benchmarker msmarco-113m`. The index is built with `pdbench-load finalize`
after restoring the heap; no heap load is performed. See
[build memory diagnostics](../vector/README.md) for retrieving the timeline
after an OOM or another build failure. Scheduled and manual main runs establish
and refresh the baseline; subsequent PRs compare with it. Diagnostics retain
extension/tool revisions, configuration, the verified query hash, segment
statistics, plans, raw pdbench reports, k6 dashboards, and database logs.

## pg_vectorsearch

Apply `benchmark-msmarco-pg_vectorsearch` to run MS MARCO with PRISM. Manual runs accept
`engine: pg_vectorsearch`; scheduled runs continue to use `pg_search`.

Both engines use the shared Docker setup and index-build memory tracking.

The PRISM profile reuses the same heap, query vectors, ground truth, runner,
worker limits, and k6 measurement procedure. It pins the parallel-build memory
fix at `paradedb/pg_vectorsearch@1bc1c77b2a07611d57c8a1eae767939bd1e39ee1`, uses
automatic `nlist` and default replication, and sweeps integer `prism.nprobe`
from 1 to 4096. The build must use parallel workers, and executed query plans
must use the expected cosine PRISM index. k6 checks the correct neighbor count
for both top-10 and top-100.

Artifacts and baseline history use `msmarco-113m-pg-vectorsearch`, separately
from ParadeDB. The shared publisher retains the same QPS/latency time series.

The pg_vectorsearch profile uses pdbench fast search: 25 evenly spaced pilot
vectors per candidate, followed by full 100-vector validation before selecting
an operating point. Candidate searches have no warmups or EXPLAIN. Selected
settings receive 10 EXPLAIN diagnostics each, shared across recall targets using
the same setting; k6 retains its separate warmup and measurement procedure.

Each query shape gets at most 20 pilot/validation evaluations and a 15-minute
search/diagnostic budget, with a 60-second statement timeout. Progress is logged
per query, completed evaluations are checkpointed in the diagnostics artifact,
and an exhausted budget lets CI continue other shapes. The summary distinguishes
time/probe budget exhaustion from an unreachable target. Only fully validated
operating points reach k6. CI restores and saves only prior selected values as
starting hints, keyed by dataset, engine revision, profile, and query assets;
pdbench checks its fingerprint and fully revalidates each hint.

Run the shared handoff tests locally with:

```sh
python3 -m unittest discover -s benchmarks/cohere -p 'test_*.py'
```

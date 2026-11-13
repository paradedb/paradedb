# MS MARCO vector CI

The MS MARCO workflow restores the existing 113,520,750-row PostgreSQL 18 heap
backup `20261102-224214F`, builds pg_search from the requested revision, and
creates an eight-segment cosine index with a 0.32 training sample ratio and
quantized vector routing. It uses the same runner and cluster setup as Cohere.

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
index size, and peak container memory during the build under
`benchmarker msmarco-113m`. Container memory excludes inactive page cache.
Scheduled and manual main runs establish and
refresh the baseline; subsequent PRs compare with it. Diagnostics retain
extension/tool revisions, configuration, the verified query hash, segment
statistics, plans, raw pdbench reports, k6 dashboards, and database logs.

For memory time series, download the `msmarco-113m-diagnostics` artifact:

- Build samples are in `benchmark-artifacts/load/*/*_load/points.jsonl`.
  Select records with `type == "Point"` and `metric == "container_memory_usage"`;
  `time` is a UTC timestamp and `value` is bytes. The samples cover the entire
  finalize step, including index creation and `VACUUM ANALYZE`.
- Query samples are in `benchmarker/results/<operating-point>.json` under
  `containers.paradedb.memory`. Each sample has `time` in Unix milliseconds
  and `value` in bytes; subtract the dashboard's top-level `startTime` for
  elapsed milliseconds. A dedicated collector covers both warmup and
  measurement; the HTML dashboard also displays memory and CPU usage.

These two memory series use the same definition: usage including page cache on
cgroup v2, or usage minus cache on cgroup v1. The build's separate
`container_memory` series excludes inactive page cache and supplies the published
peak build metric.

Run the shared handoff tests locally with:

```sh
python3 -m unittest discover -s benchmarks/cohere -p 'test_*.py'
```

## VectorChord comparison profiles

The VectorChord profiles and SQL assets use 100,000 lists for MS MARCO,
hierarchical k-means in 100 dimensions, sampling factor 64, eight build threads,
`build.pin=2`, residual quantization, and spherical centroids. Probe sweeps are
capped at the list count. Both build and search use the shared 32 GiB container
configuration, with 8 GiB shared buffers and 8 GiB `maintenance_work_mem`.

These profiles accompany the comparison workflows in [PR #7410](https://github.com/paradedb/paradedb/pull/7410);
the existing pg_search-only workflow remains unchanged.

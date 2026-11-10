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
main configuration uses fractional `paradedb.vector_cluster_max_probe` values
from 0.0001 to 1.0, starting at 0.005, with adaptive probing enabled. Experimental
absolute probe budgets and SOAR replication from #7315 are not part of this
baseline.

The workflow reuses Cohere's handoff helpers and k6 script. Plan checks require
cosine TopK execution inside ParadeDB with the correct neighbor count. Each
reached operating point runs with one VU, 10 seconds of discarded warmup, and
50 seconds of measurement. Unreached targets remain explicit in the results.

The shared publisher records latency percentiles, QPS, buffer hits, build time,
index size, worker count, and peak summed RSS under `benchmarker msmarco-113m`.
RSS includes shared mappings. Scheduled and manual main runs establish and
refresh the baseline; subsequent PRs compare with it. Diagnostics retain
extension/tool revisions, configuration, the verified query hash, segment
statistics, plans, raw pdbench reports, k6 dashboards, and database logs.

Run the shared handoff tests locally with:

```sh
python3 -m unittest discover -s benchmarks/cohere -p 'test_*.py'
```

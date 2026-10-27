# Cohere vector CI

Cohere keeps the native pgrx cluster, pgBackRest heap snapshots, and index build
used by the existing CI. `paradedb/pdbench` selects recall operating points;
`paradedb/benchmarker` measures search latency and throughput.

The workflow runs the 1M and 10M snapshots independently. Each uses the existing
100 held-out vectors, exact ground-truth fixtures, and unfiltered, approximately
10% (`year`), and approximately 1% (`battle`) full-text filters. The index retains
eight segments, a 0.32 training sample ratio, and English stemming/stopwords on
`text`. The shared cluster setup retains 12 GiB shared buffers and eight parallel
workers. Both tools use the same database and index throughout a run.

1. Restore the existing Cohere heap snapshot, refresh the installed pg_search
   extension, build `datasets/cohere/indexes/pg_search.sql`, and vacuum as before.
   Snapshot generation continues to use `datasets/cohere/config.toml` and
   `create_tables.sql`.
2. Import the published ground-truth Parquets into pdbench's fixture table.
   `pdbench search` loads the query vectors and sweeps
   `paradedb.vector_cluster_max_probe` separately for each filter to r90/r95/r99.
   Its required positive tolerance is 0.000001, below one neighbor's contribution
   to recall over 100 top-10 queries (0.001).
3. `prepare_k6.py` exports pdbench's selected points, SQL, and session settings.
   It rejects selections below their target and does not select probes or
   calculate latency. Unreached targets remain visible and do not run in k6.
4. k6 cycles over the same 100 vectors with one VU: 30 seconds of discarded
   warmup and 300 seconds of measurement per filter/target. The vector is bound
   as a parameter, with custom plans and no prepared-statement cache. The
   selected max_probe and all fixed search settings apply to every connection.
   CI verifies the parameterized plans and filter pushdown before measuring.
5. Results and recall context enter the shared publication queue. The
   Wikipedia/StackExchange publisher posts p50/p95/p99 and QPS with baseline
   comparisons and dashboard links. Recall, max_probe, and mean
   whole-query shared buffer hits/reads appear above those results. Buffer
   diagnostics come from pdbench's separate EXPLAIN pass over all 100 vectors.

The workflow runs on main and via `benchmarks`, `benchmark-cohere-pg_search`, or
`benchmark-benchmarker-cohere`. Manual runs can select a size, pg_search commit,
and tool refs. Competitor vector CI implementations and labels are retired.
The pdbench checkout uses the ParadeDB GitHub App's read access to the private
repository. Both tool revisions are pinned and recorded in diagnostics. The
fixture import only populates pdbench's ground-truth table; the corpus is restored
from its snapshot.

Results begin new `benchmarker cohere-1m` and `benchmarker cohere-10m` histories.
The old server-time samples are not comparable to k6's client-observed latency
and throughput. PR runs compare with main and do not update its baseline.

Artifacts retain index-build logs, complete pdbench search reports (including all
sweep points, recall and buffer diagnostics), selected operating points, query
vectors, checked plans, and k6 JSON/HTML dashboards. The full workload is opt-in
on PRs; the small handoff tests can be run with:

```sh
python3 -m unittest discover -s benchmarks/cohere -p 'test_*.py'
```

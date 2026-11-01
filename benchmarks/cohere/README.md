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

This branch sets `boundary_epsilon = 0.35` for both corpus sizes. A vector gets
at most one extra posting in its second-nearest cluster when the relative
distance gap is within epsilon. Search removes duplicate documents before
scoring and updating adaptive admission evidence. Epsilon stays fixed while
the existing effort sweep selects recall operating points; the index option
defaults to zero outside these benchmark profiles.

1. Restore the existing Cohere heap snapshot, refresh the installed pg_search
   extension, and run `pdbench build` with `datasets/cohere/indexes/pg_search.sql`.
   pdbench records build time, index size, and segment diagnostics before vacuum.
   Snapshot generation continues to use `datasets/cohere/config.toml` and
   `create_tables.sql`.
2. Import the published ground-truth Parquets into pdbench's fixture table.
   `admission_experiment.py` first sweeps `paradedb.vector_cluster_max_probe` with
   adaptive stopping disabled to establish nonadaptive controls. Adaptive runs
   sweep only `paradedb.vector_search_effort` and select measured recall goals
   of 90%, 95%, and 99% for each filter. Every measurement uses all 100 vectors,
   three timed repetitions, and a separate EXPLAIN pass.
   All adaptive runs use `max_probe=0` (automatic headroom).
   One router window scales with effort, cluster count, and the existing
   filter-aware cluster cost; admission evidence can stop probing earlier.
   There is no router re-query or per-segment 16-cluster floor in adaptive mode.
   The stopping fraction is `10^(-effort)`. Global quantized scans use base window
   fraction `b = min(1, 0.00075 * 10^(0.32 * effort))`; segment-local scans and
   global full-precision scans use `b = min(1, 0.002 * 10^(0.625 * effort))`.
   With cluster cost `c`, the adjusted window fraction is
   `b + min(0.4, 0.05 * (1/sqrt(c) - 1)) * b^0.125 * (1-b) * min(1, (1-b)/0.9)^2`.
   The window also covers the expected-work estimate under that cluster cost,
   subject to an explicit work ceiling. Expected work and bypass selection use
   `0.005 * effort / 4.4` of cluster capacity, bounded to the available capacity.
   All coefficients are shared across datasets and filters.
3. pdbench sweeps effort over [0.05, 10], starting at 3, and exports each
   selected operating point. Recall goals are evaluation criteria, not another
   query setting. Higher effort generally trades latency for recall; it is not
   a recall probability or guarantee. Unreached goals remain visible and do not
   run in k6. The complete measurements and nonadaptive controls are retained.
   Calibration and evaluation use the same query set, so these results do not
   establish calibration on unseen queries or datasets.
   CI also downloads the latest successful main push preceding this run with
   diagnostics for the corpus size. Its sweep reports compare the nonadaptive
   controls in `fixed-budget/` with recorded mainline sweeps at matched recall;
   adaptive results appear in the operating-point table and k6 comparisons.
   Missing baselines and incompatible configuration fingerprints are reported
   as skipped comparisons. Reports and commit provenance are retained under
   `comparison/` in the diagnostics artifact and published as Sweep reports.
4. k6 cycles over the same 100 vectors with one VU: 10 seconds of discarded
   warmup and 50 seconds of measurement per filter/target. The vector is bound
   as a parameter, with custom plans and no prepared-statement cache. The
   selected effort and fixed search settings
   apply to every connection.
   CI verifies the parameterized plans and filter pushdown before measuring.
5. Results and recall context enter the shared publication queue. The
   Wikipedia/StackExchange publisher posts p50/p95/p99, QPS, index build time,
   and index size with baseline comparisons and dashboard links. Recall,
   search effort, probe ceiling setting, and mean whole-query shared buffer hits/reads appear above those
   results. Buffer diagnostics come from pdbench's separate EXPLAIN pass over all
   100 vectors.

The workflow runs on main and via `benchmarks`, `benchmark-cohere-pg_search`, or
`benchmark-benchmarker-cohere`. Manual runs can select a size, pg_search commit,
and tool refs. Competitor vector CI implementations and labels are retired.
The pdbench checkout uses the ParadeDB GitHub App's read access to the private
repository. Both tool revisions are pinned and recorded in diagnostics. The
fixture import only populates pdbench's ground-truth table; the corpus is restored
from its snapshot.

Results begin new `benchmarker cohere-1m` and `benchmarker cohere-10m` histories.
The old server-time samples are not comparable to k6's client-observed latency
and throughput. Index build time (minutes) and size (MiB) have a separate
`benchmarker cohere-<size> (index build)` history. Its first run has no previous
baseline; subsequent PR runs compare with main and do not update its baseline.

Artifacts retain index-build logs, pdbench build and search reports (including
all sweep points, recall and buffer diagnostics), selected operating points,
query vectors, checked plans, and k6 JSON/HTML dashboards. The full workload is opt-in
on PRs; the small handoff tests can be run with:

```sh
python3 -m unittest discover -s benchmarks/cohere -p 'test_*.py'
```

For global search, admission evidence follows the global cluster order across
segments. The opening window, decayed recent window, and scored-row count are
shared by all routed segments in a collector batch. Located scans still score
all their filter matches. At geometric eligible-row checkpoints, global adaptive
scans exact-score up to k promising live candidates and reevaluate admission
evidence against the tighter threshold. Those rows are removed from the pending
shortlist to avoid scoring them twice. Remaining refinement runs after probing
stops. Filtered parallel participants each maintain their own admission state
and retain refinement after probing.

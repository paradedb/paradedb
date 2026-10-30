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
   extension, and run `pdbench build` with `datasets/cohere/indexes/pg_search.sql`.
   pdbench records build time, index size, and segment diagnostics before vacuum.
   Snapshot generation continues to use `datasets/cohere/config.toml` and
   `create_tables.sql`.
2. Import the published ground-truth Parquets into pdbench's fixture table.
   `admission_experiment.py` first sweeps `paradedb.vector_cluster_max_probe` with
   adaptive stopping disabled to establish nonadaptive controls. It measures
   the recall-target knob with one exponent, 1.25, across all three filters.
   Every measurement uses all 100 vectors, three timed repetitions and a separate
   EXPLAIN pass. All
   adaptive runs use `max_probe=0` (automatic headroom); no per-case budget is
   selected. One router window scales with target, exponent, segment size, and
   the existing filter-aware cluster-cost estimate; probing stops early on admission evidence
   and does not re-query the router. The stopping fraction `(1 - target)^exponent`
   also scales the window, so stricter stopping requests more routing headroom.
   The base fraction is `b = min(1, 0.002 / fraction^0.625)`. With the existing
   cluster cost `c`, the window fraction is
   `b + min(0.4, 0.05 * (1/sqrt(c) - 1)) * b^0.125 * (1-b) * min(1, (1-b)/0.9)^2`.
   The final taper reduces the filter allowance once the base window exceeds
   10% of the segment, preserving narrower windows below that point.
   The requested window also covers the existing expected-work estimate under
   the filter-aware cluster cost, subject to the explicit work ceiling. This
   only sets the ranked window; admission evidence can stop probing earlier.
   These coefficients are shared across segments and filters. Bypass selection
   retains its existing estimate based on the target and segment size.
3. For each recall goal (90%, 95%, 99%), bisect the knob over [0.01, 1.0] on a
   0.001 grid. Every candidate is measured across all three filters; it passes
   only when all three reach that goal. Reuse previously measured candidates.
   Export the lowest measured passing value shared across filters at each goal,
   keeping configured values separate from achieved recall. This empirical
   calibration is not a recall guarantee or proof of an optimal setting.
   Unreached targets remain visible and do not run in k6. The complete set of
   measurements and nonadaptive controls are retained. These measurements
   select and evaluate the knob on the same query set; they do not establish
   calibration on unseen queries or datasets.
4. k6 cycles over the same 100 vectors with one VU: 10 seconds of discarded
   warmup and 50 seconds of measurement per filter/target. The vector is bound
   as a parameter, with custom plans and no prepared-statement cache. The
   shared recall exponent, selected knob, and fixed search settings
   apply to every connection.
   CI verifies the parameterized plans and filter pushdown before measuring.
5. Results and recall context enter the shared publication queue. The
   Wikipedia/StackExchange publisher posts p50/p95/p99, QPS, index build time,
   and index size with baseline comparisons and dashboard links. Recall,
   recall exponent, probe ceiling setting, and mean whole-query shared buffer hits/reads appear above those
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

# Cohere vector CI

Cohere restores its existing pgBackRest heap snapshots into a source-built
ParadeDB Docker image. `paradedb/pdbench` selects recall operating points;
`paradedb/benchmarker` measures search latency and throughput.

The workflow runs the 1M and 10M snapshots independently. Each uses the existing
100 held-out vectors, exact ground-truth fixtures, and unfiltered, approximately
10% (`year`), and approximately 1% (`battle`) full-text filters. The index retains
eight segments, a 0.32 training sample ratio, and English stemming/stopwords on
`text`. Cohere, MS MARCO, HN, Wikipedia, and Stack Exchange share
`setup-docker-benchmark-cluster` and the same Docker Compose configuration:
CPUs 0–7, 64 GiB memory, 16 GiB shared memory, 24 GiB shared buffers and maintenance
memory, a 48 GiB effective cache, and eight parallel workers in a 16-process pool.
k6 runs on CPUs 8–11. Vector containers share the host PID namespace so pdbench
can still sample build RSS. Both tools use the same database and index
throughout a run.

The benchmark index enables `vector_router_quantization = true` in its
`CREATE INDEX ... WITH (...)` clause. This boolean index option defaults to false
and is stored in the index definition, so parallel builds, REINDEX, and
`VACUUM FULL` use the same setting without session configuration. Document-vector
quantization is independent. To change the router for an existing index, use
`ALTER INDEX ... SET (vector_router_quantization = true)` followed by REINDEX;
changing the option alone does not rewrite existing router data.

Router code and sidecar columns are file-backed: open reads only headers and
list topology, and scoring fetches the row ranges for the lists actually probed.
This avoids decoding the entire router during query preparation. Payload
validation occurs when each batch is read; the persisted format is unchanged.

With `paradedb.vector_stats` enabled, per-segment
`EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)` reports:

- `routing_kind` and `routing_lists_scanned` identify the router and its work.
- `routing_approximate_scored` counts one-bit scores across all router levels.
- `routing_approximate_members_scored` counts bottom-router member scores only.
- `routing_approximate_pruned` counts scored members excluded from the padded
  shortlist, before any raw vector is read for reranking.
- `routing_exact_scored`, `routing_exact_pruned`, and `routing_candidate_count`
  count reranked members, members rejected by reranking, and returned candidates.

For finite queries, approximate members scored equals approximate members
pruned plus exact scores; exact scores equals exact pruned plus candidates.
Unopened lists and members skipped by APS are excluded from the pruning count.
Non-finite queries fall back to exact scoring and report zero approximate work.
Flat and located scans do not invoke a router and omit these counters.

Router search policies can be adjusted at query time without rebuilding:

| Setting                                    | Default | Effect                                                           |
| ------------------------------------------ | ------: | ---------------------------------------------------------------- |
| `paradedb.vector_router_rerank_factor`     |     2.0 | Fractional candidate multiplier before exact reranking           |
| `paradedb.vector_router_min_beam_fraction` |     0.5 | Minimum list fraction at each quantized router level             |
| `paradedb.vector_router_aps`               |      on | Stop hierarchical routing after quiet lists                      |
| `paradedb.vector_router_audit`             |     off | Diagnostic: `stored` or `flat` routing with every segment routed |

The rerank factor accepts fractional values from 1.0 through 16.0. Retention
rounds up to whole candidates: for example, `1.25` retains up to four candidates
for `router_k = 3`. A factor of `1.0` removes padding but still computes exact
scores, which document pruning and quantized distance estimation require.

The beam setting leaves full-precision-only routing unchanged. Quantized routing
defaults to a 50% beam floor with `2 * router_k` retained members. Global routing
shares this beam and the padded shortlist across segments, including mixed raw
and quantized routers and coarse partition centroids. It reranks retained
quantized members before the global `router_k` selection.
Wider selection compensates for errors in quantized centroid ordering;
retaining more members cannot recover members in unopened lists.

1. Restore the existing Cohere heap snapshot, refresh the installed pg_search
   extension, and run `pdbench build` with `datasets/cohere/indexes/pg_search.sql`.
   pdbench records build time, index size, and segment diagnostics before vacuum.
   Snapshot generation continues to use `datasets/cohere/config.toml` and
   `create_tables.sql`.
2. Import the published ground-truth Parquets into pdbench's fixture table.
   `pdbench search` loads the query vectors and sweeps
   `paradedb.vector_cluster_max_probe` in absolute cluster-work units separately
   for each filter to r90/r95/r99. The sweep starts at 100 units and searches
   between 1 and 100,000 units, using only whole-number budgets.
   Its required positive tolerance is 0.000001, below one neighbor's contribution
   to recall over 100 top-10 queries (0.001).
   After the sweeps, CI downloads diagnostics from the latest successful main
   push preceding this workflow run with an unexpired artifact for that corpus
   size. `pdbench compare --html` compares each query at matched recall
   and saves offline HTML reports, text deltas, labeled JSON inputs, and commit
   provenance under `comparison/` in the diagnostics artifact. The job summary
   identifies the baseline run and report filenames. The PR comment's **Reports**
   section links the sweep comparisons under **Sweep** and the existing k6
   graphs under **Search**. Missing baselines or queries
   and differing config fingerprints are reported as skipped comparisons; CI
   does not force comparisons across benchmark configurations. Main runs compare
   with the previous successful main run. This reuses recorded mainline sweeps
   rather than rebuilding and measuring main on the current runner.
3. `prepare_k6.py` exports pdbench's selected points, SQL, and session settings.
   It rejects selections below their target and does not select probes or
   calculate latency. Unreached targets remain visible and do not run in k6.
4. k6 cycles over the same 100 vectors with one VU: 10 seconds of discarded
   warmup and 50 seconds of measurement per filter/target. The vector is bound
   as a parameter, with custom plans and no prepared-statement cache. The
   selected max_probe and all fixed search settings apply to every connection.
   CI verifies the parameterized plans and filter pushdown before measuring.
5. Results and recall context enter the shared publication queue. The
   Wikipedia/StackExchange publisher posts p50/p95/p99, QPS, index build time,
   and index size with baseline comparisons and dashboard links. Recall,
   max_probe, and mean whole-query shared buffer hits/reads appear above those
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

Diagnostics upload after plan verification, before the longer k6 pass, so recall
sweeps can be reviewed while latency measurements continue.
Artifacts retain index-build logs, pdbench build and search reports (including
all sweep points, recall and buffer diagnostics), selected operating points,
query vectors, checked plans, and k6 JSON/HTML dashboards. The full workload is opt-in
on PRs; the small handoff tests can be run with:

```sh
python3 -m unittest discover -s benchmarks/cohere -p 'test_*.py'
```

## Router quality

After k6, CI compares the index's stored router with a flat ranking of every
centroid on the same index. `paradedb.vector_router_audit` selects the arm:
`stored` routes every segment through its stored router, and `flat` scores every
centroid of every segment exactly from the raw centroid rows. Both disable the
located path, so every segment routes, and both keep the caller's `router_k`,
budget, and charging. The scan bounds both rankings the same way: a parallel
plan keeps the best `router_k` candidates, and a serial plan takes the whole
ranking. The setting is a diagnostic and is not documented for users.

`router_quality.py configs` writes three arm profiles from the size's profile,
each adding only the audit setting and `paradedb.vector_adaptive_probing`:
`flat` and `stored` with adaptive probing off, and `stored-adaptive` with it on.

`router_quality.py sweep` finds each arm's budget at r90/r95/r99 with
`pdbench search` (no HTML, no prewarm), each run in its own directory under
`benchmark-artifacts/router/<arm>/<query>/` so no run reuses another's cached
sweep values. Per query, the stored arm runs first:

- On the unfiltered query it is seeded from the budgets the existing sweep
  selected. That sweep is the stored router with adaptive probing on, and with
  it off the router probes a superset of the clusters at the same budget, so
  each target's range ends at the seed and starts one below it.
- On the filtered queries the existing sweep may take the located path, so the
  stored arm sweeps the profile's full range.

Flat and `stored-adaptive` are then seeded from the budgets the stored arm
selected, one pdbench run per target. `stored-adaptive` needs at least the
stored budget, so its range starts at the seed with the minimum one below it;
flat's range spans half to twice the seed. The span is a policy. A target a
seeded range does not bracket (no exact hit and no probe below the selection,
unless it sits at the profile's minimum) is swept again over the full range.

On the unfiltered query the existing sweep's report stands in for
`stored-adaptive` when the query has no filter, every segment routed at every
point, and its configuration matches the stored arm's beyond the audit and
adaptive settings; otherwise the arm is swept like the filtered queries.

pdbench saves each run's report before building its console projection, which
can fail on a parallel plan with "representative layer0 has no scan stage". The
sweep keeps such a run when it saved its report, notes it in the job summary and
the full report, and fails on any other error. `sweeps.json` records every run
(kind, seed, range, probes, seconds) and the point behind each selected budget.

pdbench stops at the first probe whose recall equals the target, so a seeded
sweep can select another budget on a recall plateau than a full-range sweep,
with the same recall.

`router_quality.py measure` reruns all 100 vectors at each selected budget under
`EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)` with `paradedb.vector_stats` on, after
one discarded warm pass, and sums each query's per-segment statistics. It checks
that each statistic means what the report says: every segment routes, only the
flat arm reports `routing_mode = flat`, filtered queries satisfy
`vectors_visited = layer0_eligible + pruned_filter + pruned_dead`, fixed-probe
arms never stop adaptively, and each query runs the same plan shape (parallel or
serial) in every arm. Segment count and K come from every segment id the stored
arm reports.

`router_quality.py render` writes:

- `router_quality.md`, the PR-comment section under the operating-point table: a
  verdict line, one table, one footnote paragraph and a link to the full report.
  The table has a row per filter level and target (Unfiltered, Filter 10%, Filter
  1%, each at r90, r95, r99) and these columns: Flat and Router clusters probed
  and their Ratio (Router / Flat); Flat and Router vectors and their Ratio, where
  vectors are vectors probed on unfiltered rows and filter-valid vectors on
  filtered rows; Adaptive on vs off, `stored-adaptive` over the stored arm on the
  row's vectors; and Routing ms, the stored arm's routing time with its share of
  query time in brackets. Flat is the flat arm and Router the stored arm, both
  with adaptive probing off. A target an arm did not reach prints `not reached`
  in that arm's cells and leaves the ratios that depend on it empty.
- `router_quality.html`, the full report: every arm's clusters, vectors probed,
  valid vectors, empty clusters, layer-0 scored, exact rows, routing time,
  centroids scored and adaptive stops per query and target, this commit's values
  beside main's, the checks, every sweep run with its seed, range and probes
  (including full-range reruns), tolerated runs, plan shapes and config. CI copies it
  into the sweep reports directory, so the publisher puts it on gh-pages under
  `benchmarker-runs/<commit>/cohere-<size>/sweep/router_quality.html`.
- `router-metrics.json` and `router-detail-metrics.json`, unfiltered values per
  target for the benchmark history. The publisher stores them as two suites:
  `benchmarker cohere-<size> (router)` holds the clusters Ratio and Adaptive on
  vs off on vectors probed, and alerts like the other suites (115%, commenting
  only on runs without a PR);
  `benchmarker cohere-<size> (router detail)` holds Flat vectors probed,
  adaptive-on vectors probed and Router routing time, without alerts. Neither
  adds rows to the PR comment's results table.
- `router_quality.json`, every number above with the commit and config
  fingerprint; the next run on main reads it as the baseline.

Only Unfiltered rows flag. Either Ratio above 1.05 turns bold with ❌ at r90
and r95 and ⚠️ at r99; with a main baseline it must also exceed main's ratio by
more than 0.05, so a router already above 1.05 on main does not flag every run.
Against main, at r90 and r95, a cell turns bold with ⚠️ and shows main's value
when Flat clusters or Flat vectors rose by more than 10%, when Adaptive on vs off
rose by more than 0.05, or when Routing ms rose by more than 25% and 0.1 ms.
Main's values appear in the comment only in flagged cells. These are policy
thresholds for review, not derived bounds.

The verdict names the worst flag: a ❌ Ratio as the factor Router needs over flat
(clusters when the clusters Ratio flags, otherwise vectors); otherwise the
against-main cell with the largest relative rise, followed by where to look
(router for a Ratio, clustering for Flat, adaptive stopping for Adaptive on vs
off, router speed for Routing ms). With nothing flagged it gives the worst Ratio,
or, when a Ratio sits above 1.05 within 0.05 of main's, that Ratio beside main's.
Filtered rows are informational: every segment is forced to route, their plans
are parallel, and routing time includes row gating. Nothing here fails the job.

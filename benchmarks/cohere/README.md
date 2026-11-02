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

This branch enables `paradedb.vector_centroid_quantization` during index
creation. The setting defaults to off and is persisted with the index, so
later build workers, merges, and search sessions retain the one-bit router
after the SQL script resets the GUC. Document quantization is independent.
Changing the setting requires rebuilding the index with CREATE INDEX or
REINDEX to select a different router. `VACUUM FULL` also rebuilds indexes, so
the workflow enables the setting again in its vacuum session before that
rebuild. `index-vacuum.log` records the setting used; a setting from the
initial build session alone does not establish the router used by searches.

Router code and sidecar columns are file-backed: open reads only headers and
list topology, and scoring fetches the row ranges for the lists actually probed.
This avoids decoding the entire router during query preparation. Payload
validation occurs when each batch is read; the persisted format is unchanged.

Plan verification enables `paradedb.vector_stats` and requires the unfiltered
query to exercise `quantized_stacked` routing with nonzero approximate and exact
scores. Per-segment `EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)` reports:

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
`router-metrics.json` and `router-metrics.md` summarize the first fixture vector
at each selected operating point, with a table in the CI summary and PR results.
These diagnostic counts are separate from pdbench's averages over all vectors.

Router composition can be selected at query time without rebuilding:

| Setting                                    | Default | Effect                                                  |
| ------------------------------------------ | ------: | ------------------------------------------------------- |
| `paradedb.vector_router_rerank_factor`     |       2 | Approximate members retained per requested centroid     |
| `paradedb.vector_router_min_beam_fraction` |     0.5 | Minimum list fraction at each quantized router level    |
| `paradedb.vector_router_aps`               |      on | Stop hierarchical routing after quiet lists             |
| `paradedb.vector_router_flat`              |     off | Score all quantized posting-centroid codes as one batch |

The flat path uses the same scoring, retention, and exact-rerank primitives;
`routing_mode` distinguishes it from hierarchical traversal in EXPLAIN. The
beam and flat settings leave full-precision routing unchanged.
Quantized routing defaults to a half-width beam with 2× member retention.
Wider selection compensates for errors in quantized centroid ordering;
retaining more members cannot recover members in unopened lists.
`router_tuning.py` runs controlled unfiltered sweeps on the existing index:
quarter/half/full beams, APS off at the full beam, flat scoring, and 2×/4×
retention. It validates that only these query settings differ and that every
report references the same index build. Exact selected recall targets and p50
appear in `router-tuning/summary.md`; HTML comparisons and verified router
counters remain in the diagnostics artifact. k6 continues to measure the
default configuration.

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

Diagnostics upload after plan verification, before the longer k6 pass, so sweeps
and router counters can be reviewed while latency measurements continue.
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

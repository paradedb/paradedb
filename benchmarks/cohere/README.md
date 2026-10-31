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
| `paradedb.vector_router_min_beam_fraction` |    0.25 | Minimum list fraction at each quantized router level    |
| `paradedb.vector_router_aps`               |      on | Stop hierarchical routing after quiet lists             |
| `paradedb.vector_router_flat`              |     off | Score all quantized posting-centroid codes as one batch |

The flat path uses the same scoring, retention, and exact-rerank primitives;
`routing_mode` distinguishes it from hierarchical traversal in EXPLAIN. The
beam and flat settings leave full-precision routing unchanged.
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
   `pdbench search` loads the query vectors and sweeps
   `paradedb.vector_cluster_max_probe` separately for each filter to r90/r95/r99.
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

Diagnostics upload after plan verification, before the longer k6 pass, so sweeps
and router counters can be reviewed while latency measurements continue.
Artifacts retain index-build logs, pdbench build and search reports (including
all sweep points, recall and buffer diagnostics), selected operating points,
query vectors, checked plans, and k6 JSON/HTML dashboards. The full workload is opt-in
on PRs; the small handoff tests can be run with:

```sh
python3 -m unittest discover -s benchmarks/cohere -p 'test_*.py'
```

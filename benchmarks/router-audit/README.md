# Router quality audit

Measures how many clusters each router needs to cover a query's exact top-10, against an exact
flat ranking of the same clusters, and what routing costs per vector against posting layer 0.

- **Coverage recall**: the share of a query's ground-truth documents inside the first `m` clusters
  of a ranking, the clusters scanned exactly. `m_r` is the smallest integer `m` with mean recall at
  least `r`: bisected between the grid points that bracket it, then scanned 8 requests either side,
  reporting any step where a larger request lost recall. The grid only draws curves.
- **FLAT**: exact scores over every stored centroid.
- **K3-2, K3-4**: the stored quantized router at `rerank_factor` 2 and 4, patience on and off.
- **K1**: a full-precision stacked router rebuilt from the exported centroids.
- **K3r**: a quantized router rebuilt the same way, 40 times. Each stored K3 row gets its percentile
  among those builds, which validates the other rebuilt routers.
- **S, L**: today's structure, and one whose levels come from the segment's own clusterer, under
  each list-open rule: `c` members of beam per requested member, list floor `f`, patience.

K1 and L are built 5 times and K3r 40 times, over input rows shuffled with seeds from 0, with output
ids mapped back to exported clusters, and reported as median and `[min, max]` `m_r`: input order
alone moves `m_r` by far more than the comparisons here need to resolve.

The index is built with one segment. Nothing here changes a product default: the export function
exists only in a `pg_search` built with the `vector_audit` feature.

## Running

Local Cohere 1M, in Docker:

```bash
benchmarks/router-audit/docker.sh
```

Docker Desktop needs at least 16 GB and 8 CPUs. The first run takes the table from the benchmarking
server with `pg_dump` into the `router-audit-data` volume. `docker.sh` lists the environment
variables that locate the inputs. Results land in `benchmarks/router-audit/out/1m`, which is
gitignored.

Cohere 10M, in CI: add the `benchmark-router-audit` label to a pull request, or dispatch
`.github/workflows/benchmark-router-audit.yml` with `size`. The job uploads `router-audit-<size>`
and puts `REPORT.md` in its summary.

`run.sh` is the same either way. Each step leaves a `.done-*` marker under `--out`, so a rerun
resumes at the first unfinished step. It stops on any of these:

- the index is not exactly one IVF segment;
- exported cluster sizes differ from `paradedb.vector_clusters`, or the document map from the row
  count;
- the stored K3 is above K3r's 95th percentile on every held-out, APS on, factor 2 row;
- S(2, 0.25, on) does not reproduce K1 on every build;
- online recall differs from the stored router's offline coverage recall by more than 0.01.

## Outputs

| File          | What it holds                                                     |
| ------------- | ----------------------------------------------------------------- |
| `REPORT.md`   | every table below                                                 |
| `audit.json`  | curves, targets, work, attribution, rules, gate                   |
| `perf.json`   | per-point stage costs at the online operating points              |
| `online.json` | offline coverage recall at those points' per-query cluster counts |
| `export/`     | what `paradedb.vector_router_export` wrote                        |
| `pdbench/`    | pdbench's build and search reports                                |

## Reading REPORT.md

- **A1**: FLAT's `m_r`, `rows_r` (rows those clusters hold) and the clusters a query needs for 9 and
  10 of its 10 documents. This is the floor every router is judged against.
- **A2**: each router's `m_r` and its ratio to FLAT; rebuilt routers show medians over builds. `grid
below` and `grid above` are the bracket, `dips` the non-monotone steps around `m_r`. At FLAT's
  `m_r`, the documents FLAT covers and the router misses split into `unopened` (the router list was
  never opened), `dropped` (scored one-bit, not retained for the rerank) and `late` (kept, ranked
  past `m`); the three are shares of `missed/q`. `gained/q` counts the reverse. `units_r` is the
  router's bytes in mean posting clusters at layer 0 (140 B per row), pricing 136 B per one-bit row
  and 4,096 B per exact row at d = 1024.
- **A3**: online latency and per-stage cost at pdbench's r90, r95 and r99 operating points, measured
  warm under `EXPLAIN` with `paradedb.vector_stats` on. `offline recall` is the stored router's
  coverage recall at the same per-query cluster counts.
- **B**: each rule's median and range of `m_r`, lists opened and vectors scored per level. `units_r`
  here prices every vector at 140 B, as a quantized router list would read it. The gate names the
  cheapest rule, by median vectors scored summed over the three targets, with median `m_r` within
  1.02 of FLAT at all three. `~` and `straddling` mark rules whose range holds builds on both sides
  of that line.
- **Non-monotone steps**: requests around `m_r` where recall fell as the request grew.

# MS MARCO vector benchmarks

The MS MARCO workflow compares pg_search, pg_vectorsearch, and VectorChord on
113,520,750 rows with 1,024-dimensional vectors. Each engine restores the same
PostgreSQL 18 heap backup (`20261102-224214F`) and runs on a `c8gd.metal-24xl`
runner with a 64-CPU / 128-GiB database container. k6 runs on cores 64–67.

## Run

Apply a PR label to run one engine:

| Engine            | PR label                            |
| ----------------- | ----------------------------------- |
| `pg_search`       | `benchmark-msmarco-pg_search`       |
| `pg_vectorsearch` | `benchmark-msmarco-pg_vectorsearch` |
| `vectorchord`     | `benchmark-msmarco-vectorchord`     |

The label is removed when the run starts. Reapply it to rerun. For an on-demand
runner, add `benchmark-nonspot` before the engine label. Remove it to use Spot.

Daily runs benchmark all three engines. Manual runs select one:

```sh
gh workflow run benchmark-pg_search-benchmarker-msmarco.yml \
  --repo paradedb/paradedb --ref main -f engine=vectorchord
```

Set `commit` to benchmark another pg_search revision. Scheduled and manual runs
on main update each engine's baseline unless `commit` is set.

## Measurements

All engines use the same first 100 held-out vectors and published ground truth
for unfiltered cosine top-10 and top-100 queries returning `_id, title`.
`pdbench-load finalize` builds and vacuums the index. `pdbench search` tunes
probes for 90%, 95%, and 99% recall. Reached targets run in k6 with one VU,
10 seconds of warmup, and 50 seconds of measurement. Targets that miss recall
have no QPS or latency result.

Plan checks require the expected vector index, and k6 checks the returned
neighbor count. Results include QPS, latency, buffer counts, build time, index
size, and peak build memory. Each engine has its own artifacts and baseline
history.

The TOML profiles contain index and recall settings. The workflow pins tool and
extension revisions. Cluster setup, extension installation, build reporting,
and k6 profiling use the same actions as HN. VectorChord's 80,000-list profile
is a starting configuration, not a tuned 113M result.

## Diagnostics

Download the engine's diagnostics artifact for plans, raw reports, k6 dashboards,
database logs, and memory traces:

- Build samples: `benchmark-artifacts/load/*/*_load/points.jsonl`. The published
  peak uses `container_memory`, which excludes inactive page cache. Timestamps
  are UTC and values are bytes.
- Query samples: `containers.paradedb.memory` in
  `benchmarker/results/<operating-point>.json`. Timestamps are Unix milliseconds
  and values are bytes. The collector subtracts cache when available, so these
  values use a different memory definition from the build peak.

Run the shared Python checks locally:

```sh
python3 -m unittest discover -s benchmarks/cohere -p 'test_*.py'
```

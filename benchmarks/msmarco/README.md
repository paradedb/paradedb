# MS MARCO vector benchmarks

The MS MARCO workflow compares pg_search, pg_vectorsearch, and VectorChord on
113,520,750 rows with 1,024-dimensional vectors. Each engine restores the same
PostgreSQL 18 heap backup (`20261102-224214F`) and runs on a `c8gd.metal-24xl`
runner with a 64-CPU / 32-GiB database container on cores 0–63. All engines use
16 GiB shared memory, 8 GiB shared buffers, 8 GiB `maintenance_work_mem`,
24 GiB `effective_cache_size`, and 256 MiB `work_mem`. k6 runs on cores 64–67.

## Run

Apply a PR label to run one engine:

| Engine                  | PR label                                  |
| ----------------------- | ----------------------------------------- |
| `pg_search`             | `benchmark-msmarco-pg_search`             |
| `pg_vectorsearch`       | `benchmark-msmarco-pg_vectorsearch`       |
| `vectorchord`           | `benchmark-msmarco-vectorchord`           |
| `vectorchord-prefilter` | `benchmark-msmarco-vectorchord-prefilter` |

The label is removed when the run starts. Reapply it to rerun. For an on-demand
runner, add `benchmark-nonspot` before the engine label. Remove it to use Spot.

Daily runs benchmark all three engines with VectorChord prefilter off. Manual runs
can also select `vectorchord-prefilter` to enable it; this variant has separate
results. Both VectorChord modes use the same unfiltered MS MARCO queries.

Manual runs select one engine and default to an on-demand runner. Use the
`runner_type` selector in **Run workflow**, or pass it through the CLI:

```sh
gh workflow run benchmark-pg_search-benchmarker-msmarco.yml \
  --repo paradedb/paradedb --ref main \
  -f engine=vectorchord -f runner_type=on-demand
```

Set `runner_type=spot` to opt into interruptible instances. Manual runs use this
input rather than PR labels. Daily scheduled runs continue to use Spot.

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
and k6 profiling use the same actions as HN. VectorChord's 100,000-list
profile uses the memory-saving training settings described below; a completed
113M run with this configuration is still pending.

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

## VectorChord comparison profiles

The VectorChord profiles use 100,000 lists for MS MARCO,
hierarchical k-means in 100 dimensions, sampling factor 64, eight build threads,
`build.pin=2`, residual quantization, and spherical centroids. Probe sweeps are
capped at the list count. Both build and search use the shared 32 GiB container
configuration, with 8 GiB shared buffers and 8 GiB `maintenance_work_mem`.

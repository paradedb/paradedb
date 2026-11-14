# MS MARCO vector CI

The MS MARCO workflow benchmarks ParadeDB, pg_vectorsearch, and VectorChord
in `.github/workflows/benchmark-pg_search-benchmarker-msmarco.yml`.
Each restores the same PostgreSQL 18 heap backup
`20261102-224214F`: 113,520,750 rows with 1,024-dimensional vectors. Every job has
its own `c8gd.metal-24xl` runner and a 64-CPU / 128-GiB database container; k6 runs
on separate cores (64–67).

All engines use the same pipeline: `pdbench-load finalize` builds the index and
vacuums the restored heap, `pdbench search` calibrates recall, and k6 measures the
selected settings. Python handles fixtures, the report handoff, and plan checks;
calibration runs in pdbench's Rust implementation.

| Engine          | Index configuration                                                                         | Integer recall sweep                                           |
| --------------- | ------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| ParadeDB        | Requested pg_search revision; eight segments, 0.32 training sample ratio, quantized routing | `paradedb.vector_cluster_max_probe`: 1–100000, starting at 100 |
| pg_vectorsearch | Pinned ParadeDB fork; PRISM cosine index with automatic `nlist`                             | `prism.nprobe`: 1–4096, starting at 64                         |
| VectorChord     | Pinned source revision; 80,000 lists, residual quantization, spherical centroids            | `vchordrq.probes`: 1–80000, starting at 100                    |

The VectorChord partition configuration is a starting point, not a tuned 113M
result. ParadeDB uses adaptive probing; SOAR replication from #7315 is not part
of this baseline. The pg_vectorsearch job copies its local load SQL into pdbench's
MS MARCO dataset and uses the existing `timescale` loader backend.

pg_vectorsearch is built from `paradedb/pg_vectorsearch` commit
`1bc1c77b2a07611d57c8a1eae767939bd1e39ee1`, which fixes retained detoast memory
during parallel index builds. CI authenticates to the private fork, builds it
with the same `setup-pg-vectorsearch` action as HN, and checks the installed
extension's commit before building the index. The source revision is retained in
diagnostics and k6 metadata. VectorChord uses HN's `setup-vectorchord` action and pinned
source revision. All engines use the shared ParadeDB benchmark image and cluster
setup; comparison runs install their extension into that container.

The pinned pdbench commit includes loading from
[pdbench #56](https://github.com/paradedb/pdbench/pull/56), calibration from
[pdbench #57](https://github.com/paradedb/pdbench/pull/57), and the recall denominator
fix for top-100. It also raises the database CPU allocation from the previous 16
to 64. Establish a fresh main baseline before interpreting PR regression ratios
against the earlier allocation.

## Running CI

Apply an extension-specific label to a ParadeDB PR to run that engine:

- `benchmark-msmarco-pg_search`
- `benchmark-msmarco-pg_vectorsearch`
- `benchmark-msmarco-vectorchord`

CI removes the label when the run starts; reapply it to rerun. Each label starts
only its selected engine. The generic `benchmarks` and Cohere labels do not
trigger this large dataset.

For on-demand runners, apply `benchmark-nonspot` to the PR **before** adding
the benchmark trigger label. This applies to every engine in this workflow.
The modifier stays on the PR for subsequent runs; remove it to use Spot again.
It does not start a benchmark by itself or change an already queued or running
job. Scheduled and manual runs use Spot because they have no PR labels.

Daily runs benchmark all three engines. For a manual run, select one engine:

```sh
gh workflow run benchmark-pg_search-benchmarker-msmarco.yml \
  --repo paradedb/paradedb --ref main -f engine=vectorchord
```

The optional `commit` input benchmarks another pg_search revision using the
workflow's benchmark code. The `pdbench_ref` input overrides the pinned tool
revision, which must support the selected engine. Only scheduled and manual main
runs without a `commit` override update the rolling baselines.

## Measurements and reports

Each engine uses the same first 100 held-out query vectors from the 1,677
published fixtures, verified by hash. Queries return `_id, title` for unfiltered
cosine top-10 and top-100 searches. pdbench independently selects settings for
90%, 95%, and 99% recall against the same published ground truth.

Plan checks require cosine TopK inside ParadeDB, or an ordered scan of the
expected PRISM/VectorChord index without a fallback sort or sequential scan.
k6 checks the returned neighbor count and uses each engine's backend manifest.
Each reached target gets one virtual user, 10 seconds of discarded warmup, and
50 seconds of measurement. Unreached targets remain explicit and have no latency
or QPS measurements.

Results include p50/p95/p99 latency, QPS, buffer hits, index build time and size,
and peak build memory excluding inactive page cache. Each engine has separate
PR reports, artifacts, and baseline histories:

- `benchmarker msmarco-113m` (ParadeDB)
- `benchmarker msmarco-113m-pg_vectorsearch`
- `benchmarker msmarco-113m-vectorchord`

Diagnostics retain configuration, extension/tool versions, query hashes, index
settings, plans, raw pdbench reports,
k6 dashboards, and database logs.

Memory traces for every engine are included in its diagnostics artifact:

- Build samples: `benchmark-artifacts/load/*/*_load/points.jsonl`, with UTC
  timestamps and byte values. The published peak uses `container_memory`,
  which excludes inactive page cache.
- Query samples: `containers.paradedb.memory` in each
  `benchmarker/results/<operating-point>.json`, with Unix millisecond timestamps
  and byte values. A dedicated container collector runs throughout the
  10-second warmup and 50-second measurement for pg_search, pg_vectorsearch,
  and VectorChord. The HTML dashboard also displays memory and CPU usage.

The pinned Benchmarker query collector subtracts `cache` when available,
otherwise it reports full container usage. This differs from the build peak's
inactive-file subtraction; these are container metrics, not per-query memory
allocations.

Run the shared handoff tests locally with:

```sh
python3 -m unittest discover -s benchmarks/cohere -p 'test_*.py'
```

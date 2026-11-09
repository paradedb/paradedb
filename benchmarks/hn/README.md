# Hacker News vector CI

The HN workflow restores the existing PostgreSQL 18 heap from
`s3://paradedb-benchmark/snapshots/hackernews/vector/v1/28m/39152428323-1/pg_backrest/`,
stanza `bench`, backup `20261018-214625F`. No copy or snapshot generation is
needed. The heap contains 28,736,557 HN rows, including 24,138,153 non-null
1,536-dimensional `text-embedding-3-small` embeddings of the `text` column.

Apply `benchmark-hn-vector` to a PR to benchmark its head. The label is removed
when the run starts so it can be applied again. Runs also execute daily on main.
After merging, start a manual main run to establish the initial baseline:

```sh
gh workflow run benchmark-pg_search-benchmarker-hn.yml \
  --repo paradedb/paradedb --ref main
```

The optional `commit` input benchmarks another revision using this workflow's
benchmark code. Only scheduled and manual main runs without that override update
the rolling baseline, under `benchmarker hackernews-vector-28m`.

Fixtures come from
`s3://paradedb-benchmark/datasets/hackernews/vector/v1/`. The workflow pins both
manifests, verifies artifact checksums, checks the restored schema and exact row
and vector counts, and rejects any overlap with the 1,000 held-out HN IDs. The
published SQL predicates and top-10 ground truth are unchanged. Migration renamed
the corpus size to `28m`; the original manifest is retained as provenance.

Like Cohere and MS MARCO, CI uses the first 100 held-out query IDs, ordered by ID
(0–99). All 1,000 published query vectors and ground-truth rows are verified and
cached locally. The exported k6 vectors must match those first 100 embeddings
exactly as float32. Ground truth is imported for the `hn_items` corpus; it is
never recomputed with the approximate index.

The ten workloads cover unfiltered search, `score >= 10`, `type = 'story'`, their
conjunction, three timestamp thresholds, and three exact authors. Author and
type columns use `pdb.literal`; score is an integer and time is `timestamptz`.
The shared EXPLAIN gate requires every filtered field and cosine TopK inside
ParadeDB, rejecting residual PostgreSQL filters and heap filtering.

The runner, cluster settings, pinned pdbench and Benchmarker revisions, and k6
script match MS MARCO. Each run builds eight segments with a 0.32 training sample
ratio and quantized routing, then independently sweeps max_probe to R90/R95/R99
for each workload. Reached points run with one VU, 10 seconds of warmup, and
50 seconds of measurement. Published boundary-tie diagnostics are retained:
strict ID recall can leave targets unreached when multiple vectors tie at the
top-10 boundary. Those points have no latency or QPS measurement.

The shared publisher records latency percentiles, QPS, buffer hits, build time,
index size, workers, and peak summed RSS. Diagnostics retain manifests, verified
fixtures, configuration, tool revisions, plans, pdbench reports, k6 dashboards,
and database logs.

Run local checks with Python 3.11+ and `duckdb==2.0.0` installed:

```sh
python3 -m unittest discover -s benchmarks/cohere -p 'test_*.py'
python3 -m unittest discover -s benchmarks/hn -p 'test_*.py'
python3 benchmarks/hn/prepare.py --output /tmp/hn-fixtures
```

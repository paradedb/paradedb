# Hacker News vector benchmarks

The HN workflow benchmarks filtered top-10 vector search on 28.7 million rows,
including 24.1 million 1,536-dimensional `text-embedding-3-small` embeddings.
It restores the PostgreSQL 18 heap and uses the first 100 held-out query vectors
with published ground truth. Fixtures, row counts, and holdout exclusion are
checked before each run.

## Run

Apply one of these PR labels, or select its engine in a manual workflow run:

| Engine                  | PR label                             |
| ----------------------- | ------------------------------------ |
| `pg_search`             | `benchmark-hn-vector`                |
| `pg_vectorsearch`       | `benchmark-hn-pg_vectorsearch`       |
| `vectorchord`           | `benchmark-hn-vectorchord`           |
| `vectorchord-prefilter` | `benchmark-hn-vectorchord-prefilter` |

```sh
gh workflow run benchmark-pg_search-benchmarker-hn.yml \
  --repo paradedb/paradedb --ref main -f engine=pg_search
```

Labels are removed when runs start. Daily runs use `pg_search`. Scheduled and
manual runs on main update each engine's baseline unless `commit` is set to
benchmark another revision.

## Workloads and results

Six workloads cover timestamp ranges at 10%, 1%, and 0.1% selectivity, and exact
authors at 0.1%, 0.01%, and 0.001%. All engines use the same heap, predicates,
query vectors, ground truth, and Docker resources.

Each workload tunes probes for R90/R95/R99. Settings that meet the target on all
100 vectors run in k6 with one VU, 10 seconds of warmup, and 50 seconds of
measurement. Ties at the top-10 boundary can leave strict ID recall targets
unreached. Targets that miss recall or exhaust their tuning budget have no QPS
or latency result.

`pg_search` uses eight segments and quantized routing. PRISM uses PostgreSQL
post-filtering; VectorChord runs with prefiltering either on or off. Comparison
plans must use the expected ANN index. Short result counts are reported for
comparison engines; query errors fail the run.

The TOML profiles and workflow contain index settings, tuning limits, and pinned
revisions. Results include QPS, latency, buffer counts, and build metrics.
Diagnostics include fixtures, plans, raw reports, k6 dashboards, and database logs.

## Fixtures

- Heap: `s3://paradedb-benchmark/snapshots/hackernews/vector/v1/28m/39152428323-1/pg_backrest/`
  (stanza `bench`, backup `20261018-214625F`).
- Queries and ground truth: `s3://paradedb-benchmark/datasets/hackernews/vector/v1/`.

To check fixtures locally, use Python 3.11+ with `duckdb==2.0.0`:

```sh
python3 benchmarks/hn/prepare.py --output /tmp/hn-fixtures
```

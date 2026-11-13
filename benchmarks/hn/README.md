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

The six workloads cover three timestamp ranges and three exact authors.
Names use approximate selectivity among the 24,138,153 rows with embeddings:

| Query name             | Published GT workload | Matching vectors |
| ---------------------- | --------------------- | ---------------: |
| `range_10pct`          | `knn_top10_time_1`    |        2,413,816 |
| `range_1pct`           | `knn_top10_time_2`    |          241,382 |
| `range_0.1pct`         | `knn_top10_time_3`    |           24,139 |
| `single_term_0.1pct`   | `knn_top10_author_1`  |           24,150 |
| `single_term_0.01pct`  | `knn_top10_author_2`  |            2,414 |
| `single_term_0.001pct` | `knn_top10_author_3`  |              241 |

Decimal points are preserved in query filenames, pdbench identities, and result
labels. GT imports map the published workload names to these CI names; SQL
contents and neighbor IDs stay unchanged. The index includes the exact author
column with `pdb.literal`, the `timestamptz` time column, the ID, and the embedding.
The shared EXPLAIN gate requires every filtered field and cosine TopK inside
ParadeDB, rejecting residual PostgreSQL filters and heap filtering.

The runner, cluster settings, pinned pdbench and Benchmarker revisions, and k6
script match MS MARCO. Each run builds eight segments with a 0.32 training sample
ratio and quantized routing, then independently sweeps integer max_probe work
budgets from 1 to 100,000, starting at 100, to reach R90/R95/R99 for each workload.
Reached points run with one VU, 10 seconds of warmup, and
50 seconds of measurement. Published boundary-tie diagnostics are retained:
strict ID recall can leave targets unreached when multiple vectors tie at the
top-10 boundary. Those points have no latency or QPS measurement.

The shared publisher records latency percentiles, QPS, buffer hits, build time,
index size, and peak container memory during the build. Diagnostics retain manifests, verified
fixtures, configuration, tool revisions, plans, pdbench reports, k6 dashboards,
and database logs.

Run local checks with Python 3.11+ and `duckdb==2.0.0` installed:

```sh
python3 -m unittest discover -s benchmarks/cohere -p 'test_*.py'
python3 benchmarks/hn/prepare.py --output /tmp/hn-fixtures
```

## Comparison engines

The HN workflow also accepts these engines and PR labels:

| Dispatch engine         | PR label                             | Index / filter mode              |
| ----------------------- | ------------------------------------ | -------------------------------- |
| `pg_vectorsearch`       | `benchmark-hn-pg_vectorsearch`       | PRISM, PostgreSQL post-filtering |
| `vectorchord`           | `benchmark-hn-vectorchord`           | VectorChord, prefilter off       |
| `vectorchord-prefilter` | `benchmark-hn-vectorchord-prefilter` | VectorChord, prefilter on        |

Scheduled runs retain pg_search. Each comparison uses the same heap, six timestamp
and exact-author predicates, 100 held-out vectors, and Docker resources as
pg_search. Results and recall hints are separated by engine and filter mode.

pg_vectorsearch is pinned to `paradedb/pg_vectorsearch@1bc1c77b2a07611d57c8a1eae767939bd1e39ee1`,
including the parallel-build memory fix. Its global PRISM index uses automatic
`nlist` and integer `prism.nprobe` from 1 to 4096. VectorChord 1.1.1 is pinned to
`f9b5e234c44cee69b266b8431f92b7deed4e99d3`. Both VectorChord profiles use one global
cosine index, 80,000 lists, residual quantization, spherical centroids, hierarchical
k-means in 100 dimensions, sampling factor 64, and eight build threads. They differ
only in `vchordrq.prefilter`; integer probes range from 1 to 80,000.

The comparison profiles use 25-vector pilots, full validation on all 100 vectors,
and 10 diagnostic EXPLAIN queries per selected setting. Each shape gets at most
64 evaluations, three hours, and a five-minute statement timeout; jobs allow
24 hours. Cached probes are revalidated starting hints. Only settings passing
R90/R95/R99 validation reach k6, with its existing warmup and measurement periods.
Budget exhaustion is reported separately from unreachable recall.

Executed plans must retain every predicate and use the expected ordered ANN
index scan. VectorChord's prefilter setting is checked in both EXPLAIN and k6
sessions. Short result counts remain diagnostic for these HN comparison engines
after the recall gate; query errors still fail k6. The pg_search checks remain
unchanged. The shared runtime guard records and verifies container limits and
PostgreSQL settings before benchmarking.

# MS MARCO vector comparison

Apply `benchmark-msmarco-pg_search` to this PR to run three independent jobs
against the completed 113,520,750-row heap snapshot. The workflow removes the
label so another run can be requested explicitly. No benchmark changes need
to be merged first.

The `benchmark-msmarco-main` label starts only the pinned main baseline on a
separate runner. It can replace a failed main job while the other variants
continue. Manual dispatch also accepts `variant=main`, `pr7160`, or
`pg_vectorsearch`; `all` remains the default. Benchmarker builds enable
verbose xk6 logging so dependency-resolution failures retain their cause.

| Variant         | Extension revision                                            | Recall tuning                                           |
| --------------- | ------------------------------------------------------------- | ------------------------------------------------------- |
| main            | `c9030485d8c388f1cc56f8c808e80315fe47323b`                    | `paradedb.vector_cluster_max_probe`                     |
| PR #7160        | `e3c06a5c611003c5c9d55b4cea4d636f37163e51`                    | `paradedb.vector_search_effort`, automatic probe budget |
| pg_vectorsearch | private memory fix `1bc1c77b2a07611d57c8a1eae767939bd1e39ee1` | integer `prism.nprobe`                                  |

The private `paradedb/pg_vectorsearch` build fixes per-tuple detoast retention
in parallel index scans. It uses the existing engine support from pdbench
PR #45. All three jobs pin pdbench to
`ac64c0a34a1558ca72e07a342b51b52076befa50` and Benchmarker to
`ecdca03f4da6e6e2d66ed01df2e5c33d4062d0d3`. The workflow and assets come from
this PR; each pg_search extension is built from its exact revision above.
The pg_vectorsearch job uses main's pg_search installation to restore the
cluster, then builds only the PRISM index.

## Workload and resources

Each job restores
`s3://paradedb-benchmark/snapshots/msmarco/v1/113m/pg_backrest/` and builds a
fresh cosine vector index over the same 1,024-dimensional embeddings.
The completed full backup is `20261102-224214F`. The snapshot has no search
index, and its exact row count was verified before backup.

Only unfiltered top-10 and top-100 queries run. pdbench selects settings
separately for R90, R95, and R99 using the first 100 held-out query vectors,
ordered by ID, and the published ground truth under
`s3://paradedb-benchmark/datasets/msmarco/v1/queries/`. An unreached target
is reported explicitly and receives no k6 latency measurement.

All jobs use PostgreSQL 18 on separate ARM64 `c8gd.metal-24xl` runners, with
12 GiB shared buffers, 12 GiB maintenance memory, and eight available parallel
workers. pg_search uses eight target segments and a 0.32 training sample
ratio. pg_vectorsearch uses automatic `nlist` and requests eight parallel
build workers through both PostgreSQL and the table setting. Its build must
record at least one worker to pass the parallel-build check. pdbench saves
worker and RSS samples alongside build time and index size; summed process
RSS can include shared mappings.

The full benchmark has a 12-hour job limit. The first full-corpus run still
needs to establish index-build runtime, memory, and disk requirements.

## Validation and results

After `VACUUM ANALYZE`, the workflow imports both published ground-truth
fixtures, runs recall sweeps, and checks parameterized `EXPLAIN ANALYZE`
plans before timing selected points. pg_search must perform cosine TopK
inside its scan; pg_vectorsearch must use the expected PRISM index. The
PR #7160 job also verifies quantized routing counters and exact reranking.

k6 runs one VU, with 10 seconds of warmup followed by 50 seconds of measurement
per reached recall target. Each job retains its configuration, source and
tool revisions, build diagnostics, recall reports, plans, database log, and
k6 latency/QPS dashboards in `msmarco-113m-<variant>-diagnostics` for 30 days.
The Actions summary includes selected settings and achieved recall.
These comparison runs do not publish a rolling main baseline or post PR
comments.

The shared helpers in `benchmarks/cohere/` support both engines and preserve
each workload's k, dimensions, and query-vector operand. Local checks:

```sh
python3 -m unittest discover -s benchmarks/cohere -p 'test_*.py'
pre-commit run --files .github/workflows/benchmark-pg_search-benchmarker-msmarco.yml \
  benchmarks/cohere/prepare_k6.py benchmarks/cohere/verify_plans.py
```

# MS MARCO vector CI

The opt-in `benchmark-msmarco-pg_search` label runs unfiltered top-10 and
top-100 cosine search over all 113,520,750 MS MARCO v2.1 passages with
1,024-dimensional Cohere Embed English v3 vectors. The corpus and published
ground truth live under `s3://paradedb-benchmark/datasets/msmarco/v1/`.

CI uses the first 100 of the 1,677 held-out queries, ordered by query ID, for
both pdbench and k6. Each workload independently selects max_probe for
90%, 95%, and 99% recall. A target that is not reached remains in the report
without a k6 measurement. Filtered workloads are not part of this suite.

The benchmark uses the same PostgreSQL 18 ARM64 runner, 12 GiB shared buffers,
12 GiB maintenance memory, eight workers, eight target segments, and 0.32
training sample ratio as Cohere CI. The index includes text and vectors; the
queries return `_id` and `title`. The corpus table is named `msmarco` to match
the native heap loader's Parquet directory convention.

## Initial setup

The [heap-generation changes](../datasets/msmarco/README.md) must be on main.
The benchmark workflow can remain in a PR while it is tested.

1. Run the snapshot workflow once. MS MARCO must be selected explicitly;
   the snapshot workflow's `all` selection excludes it.

   ```sh
   gh workflow run generate-benchmark-snapshots.yml --ref main \
     -f dataset=msmarco -f size=113m
   ```

   The native `load-heap` command imports the corpus, verifies its exact row
   count, freezes and analyzes the heap, then publishes pgBackRest data at
   `s3://paradedb-benchmark/snapshots/msmarco/v1/113m/pg_backrest/`.
   The snapshot contains no search index. Generation is restricted to main
   and uses the existing benchmark write role.

2. After snapshot generation succeeds, apply `benchmark-msmarco-pg_search` to
   the PR containing the benchmark workflow. This runs restore, index build,
   recall sweeps, plan checks, k6, and result publication before merging the
   benchmark changes. Main comparisons are skipped until a baseline exists.
   The workflow removes the label so it can be reapplied for another run.
   It is not triggered by the generic `benchmarks` label or a schedule.

3. Once the benchmark workflow is merged, run it on main to establish the first
   baseline:

   ```sh
   gh workflow run benchmark-pg_search-benchmarker-msmarco.yml --ref main
   ```

   A successful main run with no `commit` override and `publish_baseline=true`
   publishes metrics and a 90-day sweep baseline artifact. Manual runs can
   select another extension commit or disable baseline publication. Those
   runs do not replace the main baseline.

The initial load and full benchmark need substantial disk and runtime: raw
vectors alone occupy approximately 465 GB, in addition to text, PostgreSQL
storage, WAL, and the rebuilt index. The first full run must confirm the
runner capacity and six-hour job limit. No full-corpus runtime is assumed.

## Run and results

Each label run restores the heap, installs the selected extension, and uses
`pdbench build` to rebuild the index. `VACUUM ANALYZE` runs after index
creation; the heap was already frozen during snapshot preparation.
The workflow imports both published ground-truth fixtures before the sweeps.
The shared helpers in `benchmarks/cohere/` accept `--dataset msmarco`, preserve
each workload's k and query-vector operand, and check the parameterized plans
before k6 measures them. k6 uses one VU, 10 seconds of discarded warmup, and
50 seconds of measurement per reached recall target.

The existing publisher reports p50/p95/p99, QPS, index build time and size,
recall, max_probe, and shared buffer diagnostics under `msmarco-113m`.
The latest successful main baseline preceding the run supplies matched-recall
sweep comparisons; differing configurations are skipped rather than forced.
The `msmarco-113m-diagnostics` artifact retains build logs, plans, pdbench
reports, operating points, and comparisons for 30 days.

This native CI setup reuses Cohere's snapshot/build/search path. The standalone
`pdbench-load` Docker configuration in the pdbench repository remains usable
for separate loading and index-building experiments.

## Local checks

```sh
python3 -m unittest discover -s benchmarks/cohere -p 'test_*.py'
pre-commit run --files .github/workflows/benchmark-pg_search-benchmarker-msmarco.yml \
  .github/workflows/generate-benchmark-snapshots.yml
```

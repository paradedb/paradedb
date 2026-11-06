# MS MARCO 113m benchmark

Benchmarks a pg_search commit on the 113,520,750-row MS MARCO heap snapshot
(`s3://paradedb-benchmark/snapshots/msmarco/v1/113m/pg_backrest/`): restore the
snapshot, build the vector index with the profile in `113m.toml`, import the
published ground truth, sweep `paradedb.vector_cluster_max_probe` to recall
0.90 / 0.95 / 0.99 for unfiltered top-10 and top-100, then measure each reached
operating point with k6. The run takes 6 to 12 hours.

## Running it

- On a pull request: add the `benchmark-msmarco-pg_search` label. The workflow
  benchmarks the PR's own head with the PR's own copy of this directory and
  removes the label once it has started.
- By hand: dispatch `Benchmark pg_search (Benchmarker MS MARCO)` from the
  Actions tab. `commit` (default: the triggering ref) picks the pg_search
  commit; the benchmark code is this workflow's. `pdbench_ref` and
  `benchmarker_ref` override the tool commits.

There is no schedule.

## Profile

`113m.toml` is the profile pg_search runs with its index defaults: eight
parallel build and query workers, a 32-process worker pool, eight target
segments, a 32% training sample, 100 held-out query vectors, sweep targets
0.90 / 0.95 / 0.99. The Cohere profile differs from it in one line it does not
carry: `vector_router_quantization`.

## Baseline

The comparison step looks for the latest successful `workflow_dispatch` run on
`main` carrying an `msmarco-113m-diagnostics` artifact. With none, the step is
skipped and the summary says so. A run publishes baseline metrics only when it
is a `workflow_dispatch` on `main`.

## Files

- `113m.toml`: the pdbench profile.
- `assets/msmarco/`: the table, index and query definitions pdbench renders.
- `prepare_k6.py`: turns the sweep's reached operating points into k6 inputs
  and the step summary.
- `verify_plans.py`: checks the query plans k6 will run.
- `k6/search.js`: the k6 scenario.
- `test_*.py`: unit tests for the helpers, run at the start of every workflow.

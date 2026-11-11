# Vector index build memory

HN and MS MARCO restore their existing PostgreSQL 18 heap snapshots into a
bind-mounted directory, then run the source-built ParadeDB image with the shared
Docker configuration in this directory. Both `pg_search` and `pg_vectorsearch`
use this setup; the pinned PRISM extension is compiled inside the same runtime
distribution and installed in the image. Neither job reloads the heap.

The container has a 128 GiB memory limit, no swap, and 16 GiB of shared-memory
capacity. The runner retains memory for pdbench, k6, and artifact uploads.
The previous 12 GiB shared buffers, 12 GiB maintenance memory, eight parallel
workers, and 32-process worker pool are retained. Explicit huge pages are
disabled so shared buffers are included in Docker's memory accounting.
This changes the execution environment, so compare new Docker runs against a
new main baseline rather than interpreting the transition as a code regression.

`prepare_index_build.py` derives a finalize-only `pdbench-load` config from the
same TOML profile and index SQL used by the recall sweep. `pdbench-load finalize`
runs on the host and samples the database container about once per second while
building the index. Vacuum runs afterward, outside the measured build.

The published build metrics contain elapsed finalize time (including sampler
startup), index size, and peak container memory both excluding reclaimable file
cache and including cache on cgroup v2. Shared mappings are counted once across
database processes. These memory series replace peak summed RSS.
`build_index.py` also records parallel workers attached to the index build via
`pg_stat_activity` and `pg_stat_progress_create_index`; PRISM runs still require
an observed parallel build.

## Retrieve a failed build

Each job uploads a separate `hackernews-vector-28m-index-build` or
`msmarco-113m-index-build` artifact immediately after the build, on success or
failure, retained for 30 days. PRISM artifact names include `-pg-vectorsearch`
before `-index-build`. Download it from the Actions run's artifacts:

- `runs/load/<timestamp>_load/points.jsonl`: the timestamped CPU, memory, and I/O
  samples recorded before the failure.
- `runs/load/<timestamp>_load/run.json`: run configuration, status, and error.
- `runs/load/<timestamp>_load/report.html`: a standalone interactive timeline.
- `workers.jsonl`: parallel worker samples collected during the build.
- `build.log`, `postgres.log`, `container-state.json`, `oom-events.jsonl`,
  `cgroup-memory.log`, and `kernel.log`: build output and OOM diagnostics.

`pdbench-load` writes samples while the build runs, outside the database's
memory limit. A killed PostgreSQL backend or container therefore leaves the
already-flushed samples available. Docker OOM events and cgroup `oom_kill`
counters can identify backend kills even if the postmaster survives and
`State.OOMKilled` is false. The last sampled value may miss a short memory spike.

If the recorder itself is killed, its metadata may still say `running`; the raw
points and report remain readable, including a truncated final JSONL line. The
report can also be regenerated with `pdbench-report html <run-directory>`.
Runner loss or a hard job timeout that prevents cleanup/upload can still lose
the local artifacts; this does not provide off-runner streaming storage.

## Validation

`python3 -m unittest discover -s benchmarks/vector -p 'test_*.py'` checks that
both engines and datasets retain their index DDL and settings. The `Test Vector Benchmark
Memory` workflow also runs `test_memory_recovery.sh` against a small PostgreSQL
container: it verifies a successful index build, forces an OOM during a second
`CREATE INDEX`, and checks that samples and an HTML report survive.

The workflow also builds the PRISM Docker image and creates a small index to
check that its library, dependencies, and extension SQL load successfully.

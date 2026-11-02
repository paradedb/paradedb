# MS MARCO heap snapshot

After merging the dataset assets and workflow change into main, generate the
heap snapshot with:

```sh
gh workflow run generate-benchmark-snapshots.yml --repo paradedb/paradedb \
  --ref main -f dataset=msmarco -f size=113m
```

The existing native loader imports all 113,520,750 rows from
`s3://paradedb-benchmark/datasets/msmarco/v1/sampled/113m/parquet/msmarco/`
into PostgreSQL 18. The workflow verifies the exact row count, freezes and
analyzes the heap, and backs it up with pgBackRest to
`s3://paradedb-benchmark/snapshots/msmarco/v1/113m/pg_backrest/`.

The source contains embedded NUL bytes in text, which PostgreSQL cannot store.
`pg_null_byte_replacement = ""` removes those bytes during Parquet loading.
Datasets that omit this setting retain the default behavior of rejecting them.

`parquet_load_workers = 16` loads separate shards concurrently through independent
DuckDB/Postgres connections. Each worker uses two DuckDB threads and a 2 GB memory
limit. Every minute, a log-friendly progress bar shows committed rows and
shards, recent and average throughput, elapsed time, and estimated time remaining.
The percentage uses row counts from Parquet metadata, so unequal shard sizes do
not distort it. Rows in active, uncommitted shards are excluded; the ETA is an
estimate of the load only, excluding verification/vacuum and backup.

Set the workflow's `load_workers` input (for example, `-f load_workers=8`) to tune
the next run without a code change. The native CLI accepts
`--parquet-load-workers 8`. This does not change worker counts on a running job.
A failed shard fails the load; rerunning `load-heap` recreates the table before
loading again. Datasets that omit the setting keep the single bulk import.

The MS MARCO job allows 12 hours overall and refreshes its six-hour AWS credentials
immediately before loading and before backup. Loading is capped at five hours and
backup at five and a half hours, leaving headroom within each credential session.

The snapshot contains the heap with 1,024-dimensional vectors and no search
index. Snapshot generation runs only from main and uses the existing benchmark
write role. MS MARCO requires explicit selection and is excluded from `all`.
Regenerating replaces the existing snapshot at this destination.

Wait for the workflow to succeed before running benchmarks that restore this
snapshot. Full-corpus loading time and disk capacity need confirmation in the
first run; the raw vectors alone occupy approximately 465 GB.

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

The snapshot contains the heap with 1,024-dimensional vectors and no search
index. Snapshot generation runs only from main and uses the existing benchmark
write role. MS MARCO requires explicit selection and is excluded from `all`.
Regenerating replaces the existing snapshot at this destination.

Wait for the workflow to succeed before running benchmarks that restore this
snapshot. Full-corpus loading time and disk capacity need confirmation in the
first run; the raw vectors alone occupy approximately 465 GB.

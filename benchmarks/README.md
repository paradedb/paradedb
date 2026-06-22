# ParadeDB Benchmarks

This is a basic, single-query latency, benchmarking suite for ParadeDB. It executes all of the queries belonging to a dataset, capturing both hot and cold measurements of their performance. After setup, queries are executed until a 3-run window shows <0.1% variance/mean (or 10 runs pass). Then `--runs` samples are taken for the hot measurements. The cold measurement is always recorded from the first query run.

## Prerequisites

The benchmarking scripts require a Postgres database with [`pg_search`](/pg_search) installed. If you are building `pg_search` with
`cargo pgrx`, make sure to build in `--release` mode. It also requires AWS credentials to be available in order to load the data.

### pg_stat_staements

Query timing is done via `pg_stat_statements`, so you'll need to configure it. The import bits are:

- `pg_stat_statements` must be in `shared_preload_libraries`.
  - `ALTER SYSTEM SET shared_preload_libraries = pg_search,pg_stat_statements;`
- Then after a postgres restart, configure it:

  ```sql
  CREATE EXTENSION pg_stat_statements;
  ALTER SYSTEM SET pg_stat_statements.track_planning = on;
  ALTER SYSTEM SET pg_stat_statements.track = top;
  ```

  These will take effect after one more postgres restart.

## Usage

The `benchmark` subcommand runs benchmarks against an already-loaded heap. `load-heap` bulk-loads CSV data into Postgres, `snapshot-heap` and `restore-heap` manage pgBackRest heap snapshots, and `sample`/`convert` prepare source data. See [DATASET_PREPARATION.md](DATASET_PREPARATION.md) for the full dataset flow.

To reproduce a benchmark locally from existing CSV data, load the heap once and then run `benchmark`:

```bash
POSTGRES_URL="postgresql://localhost:28818/postgres"

cargo run --release -- load-heap \
  --url "${POSTGRES_URL}" \
  --dataset stackoverflow \
  --size 100k

cargo run --release -- benchmark \
  --url "${POSTGRES_URL}" \
  --dataset stackoverflow \
  --index bm25
```

To reuse that heap through pgBackRest, stop Postgres before snapshotting or restoring:

```bash
PGDATA="/home/runner/.pgrx/data-18"
BACKREST_ARGS=(
  --stanza bench
  --repo-bucket paradedb-ci-benchmarks
  --repo-path-prefix /snapshots
  --repo-region us-east-1
  --repo-endpoint s3.us-east-1.amazonaws.com
  --repo-s3-key-type shared
)

(cd ../pg_search && cargo pgrx stop pg18)
cargo run --release -- snapshot-heap \
  --dataset stackoverflow \
  --size 100k \
  --pgdata "${PGDATA}" \
  "${BACKREST_ARGS[@]}"
cargo run --release -- restore-heap \
  --dataset stackoverflow \
  --size 100k \
  --pgdata "${PGDATA}" \
  "${BACKREST_ARGS[@]}"
(cd ../pg_search && cargo pgrx start pg18)
```

For more options:

```bash
cargo run -- --help
```

## Notable `benchmark` Options

- `--dataset` defaults to "stackoverflow"
- `--index` (required): Selects the index to build/benchmark, `datasets/{dataset}/indexes/{index}.sql` (e.g. `bm25`, `hnsw`, `ivfflat`).
- `--clear-caches` must be set to `false` if you're running on a non-Linux system. (It defaults to `true`).
- `--skip-index`: Including this skips index creation (and the after-create-index hook). Useful for iterating on queries against an already-indexed database.
- `--runs`: How many warm samples to capture from each query. Defaults to 3.
- `--vacuum`: Controls whether `VACUUM FULL ANALYZE` is ran before running the queries. Defaults to `true`.

## Notable Heap Options

- `load-heap --size`: Selects `sampled/{size}/csv` under the data source.
- `load-heap --data-source`: Overrides `s3_base_path` in `datasets/{dataset}/config.toml`.
- `snapshot-heap --pgdata` / `restore-heap --pgdata`: Points pgBackRest at the stopped Postgres data directory. This can also be provided with `PGDATA`.
- `snapshot-heap --config` / `restore-heap --config`: Uses an existing pgBackRest config instead of generating one.
- `snapshot-heap --repo-*` / `restore-heap --repo-*`: Provides the S3 repository settings when generating a pgBackRest config.

## Datasets

Each benchmark run uses a single dataset located under `datasets/$name`. The heap must already be present — loaded by `load-heap` (which reads from the dataset's `data-source` at the given `--size`) or restored from a snapshot.

The queries that are benchmarked for a dataset are located at `datasets/$name/queries/*.sql`. Each query file represents a single query: when a single file contains multiple queries, the first query in the file is considered to be the canonical/idiomatic way to write the query, and any additional queries in the file are considered alternative ways to write the query. The canonical query may not always be the fastest (yet!) but we strive to make the canonical query perform as well as a non-idiomatic, slightly contorted query might.

### Dataset Directory Layout

`datasets/{name}/`:

- `config.toml`
- `create_tables.sql`
- `indexes/{index}.sql` (one file per index variant, e.g. `bm25`, `hnsw`, `ivfflat`; chosen with `benchmark --index`)
- `prewarm.sql`
- `queries/*.sql`
- `after_create_index.sql` (optional)

### Preparing Datasets

For preparing and managing non-synthetic datasets (loading source data, sampling, and conversion), see [DATASET_PREPARATION.md](DATASET_PREPARATION.md).

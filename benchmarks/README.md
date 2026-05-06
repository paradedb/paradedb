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

The `benchmark` subcommand runs benchmarks. `sample` and `convert` are also available. See [DATASET_PREPARATION.md](DATASET_PREPARATION.md) for their usage.

The following command loads the 100k row Stack Overflow dataset, builds a BM25 index, runs benchmarking queries, and outputs the results to a Markdown file.

```bash
cargo run -- benchmark --url POSTGRES_URL --size 100k
```

For more options:

```bash
cargo run -- --help
```

## Notable `benchmark` Options

- `--size` is a path label, not an exact row count. It maps to a dataset on s3 of which the root table has approximately that many rows.
- `--data-source` overwrites the configured data source. For example, we use this in CI to load from the CI account bucket, instead of the default prod account bucket.
- `--dataset` defaults to "stackoverflow"
- `--clear-caches` must be set to `false` if you're running on a non-Linux system. (It defaults to `true`).
- `--skip-setup`: Including this skips data load and index creation. Useful for testing new queries locally.
- `--runs`: How many warm samples to capture from each query. Defaults to 3.
- `--vacuum`: Controls whether `VACUUM FULL ANALYZE` is ran before running the querys. Defaults to `true`.

## Datasets

Each benchmark run uses a single dataset located under `datasets/$name`, with data loaded from the specified `data-source` (which defaults to the value in the dataset's `config.toml`), of the size specified by `--size`.

The queries that are benchmarked for a dataset are located at `datasets/$name/queries/*.sql`. Each query file represents a single query: when a single file contains multiple queries, the first query in the file is considered to be the canonical/idiomatic way to write the query, and any additional queries in the file are considered alternative ways to write the query. The canonical query may not always be the fastest (yet!) but we strive to make the canonical query perform as well as a non-idiomatic, slightly contorted query might.

### Dataset Directory Layout

`datasets/{name}/`:

- `config.toml`
- `create_tables.sql`
- `create_index.sql`
- `prewarm.sql`
- `queries/*.sql`
- `after_create_index.sql` (optional)

### Preparing Datasets

For preparing and managing non-synthetic datasets (loading source data, sampling, and conversion), see [DATASET_PREPARATION.md](DATASET_PREPARATION.md).

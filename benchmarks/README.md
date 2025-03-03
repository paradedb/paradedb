# ParadeDB Benchmarks

Benchmarking suite for ParadeDB. Executes a series of common full text and faceted queries over a generated table,
with text, numeric, timestamp and JSON columns.

## Prerequisites

The benchmarking scripts require a Postgres database with [`pg_search`](pg_search) installed. If you are building `pg_search` with
`cargo pgrx`, make sure to build in `--release` mode.

## Usage

1. Run `create-table.sql` to generate the test table. The following command populates it with `1000000` rows.

```bash
psql POSTGRES_URL -f create-table.sql -v num_rows=1000000
```

2. Run `index.sh` to create a BM25 index over the table. Once completed, an `index.md` file will be created with stats on indexing time, index size, etc.

```bash
./index.sh POSTGRES_URL
```

3. Run `benchmark.sh` to benchmark against the test queries. Once completed, a `benchmark.md` file will be created with benchmark results.

```bash
./benchmark.sh POSTGRES_URL
```

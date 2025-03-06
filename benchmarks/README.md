# ParadeDB Benchmarks

Benchmarking suite for ParadeDB. Executes a series of common full text and faceted queries over a generated table,
with text, numeric, timestamp and JSON columns.

## Prerequisites

The benchmarking scripts require a Postgres database with [`pg_search`](pg_search) installed. If you are building `pg_search` with
`cargo pgrx`, make sure to build in `--release` mode.

## Usage

The following command generates a test table, builds a BM25 index, runs benchmarking queries, and outputs the results to a Markdown file.

```bash
cargo run -- --url POSTGRES_URL
```

For more options:

```bash
cargo run -- --help
```

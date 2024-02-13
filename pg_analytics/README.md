<h1 align="center">
  <img src="../docs/logo/pg_analytics.svg" alt="pg_analytics" width="500px">
<br>
</h1>

## Overview

`pg_analytics` is an extension that accelerates analytical query processing inside Postgres. The performance of analytical queries that leverage `pg_analytics` is comparable to the performance of dedicated OLAP databases — without the need to extract, transform, and load (ETL) the data from your Postgres instance into another system. The purpose of `pg_analytics` is to be a drop-in solution for fast analytics in Postgres with zero ETL.

The primary dependencies are:

- [x] [Apache Arrow](https://github.com/apache/arrow) for column-oriented memory format
- [x] [Apache DataFusion](https://github.com/apache/arrow-datafusion) for vectorized query execution with SIMD
- [x] [Apache Parquet](https://github.com/apache/parquet-mr/) for persistence
- [x] [Delta Lake](https://github.com/delta-io/delta-rs) as a storage framework with ACID properties
- [x] [pgrx](https://github.com/pgcentralfoundation/pgrx), the framework for creating Postgres extensions in Rust

## How It Works

These libraries are the building blocks of many modern analytical databases and enable column-oriented storage, efficient data compression, and vectorized query execution within Postgres. Please see our [blog post](https://blog.paradedb.com/pages/introducing_analytics) for a deep dive into how it works.

## Benchmarks

With `pg_analytics` installed, ParadeDB is the fastest Postgres-based analytical database and outperforms many specialized OLAP systems. On Clickbench, ParadeDB is 94x faster than regular Postgres, 8x faster than Elasticsearch, and almost ties Clickhouse.

<img src="../docs/images/clickbench_results.png" alt="Clickbench Results" width="1000px">

For an apples-to-apples comparison, these benchmarks were run on a c6a.4xlarge with 500GB storage. None of the databases were tuned. The (Parquet, single) Clickhouse variant was selected because it most closely matches ParadeDB's Parquet storage.

You can view the ParadeDB ClickBench results against other Postgres-compatible OLAP databases [here](https://benchmark.clickhouse.com/#eyJzeXN0ZW0iOnsiQWxsb3lEQiI6dHJ1ZSwiQXRoZW5hIChwYXJ0aXRpb25lZCkiOnRydWUsIkF0aGVuYSAoc2luZ2xlKSI6dHJ1ZSwiQXVyb3JhIGZvciBNeVNRTCI6dHJ1ZSwiQXVyb3JhIGZvciBQb3N0Z3JlU1FMIjp0cnVlLCJCeUNvbml0eSI6dHJ1ZSwiQnl0ZUhvdXNlIjp0cnVlLCJjaERCIjp0cnVlLCJDaXR1cyI6dHJ1ZSwiQ2xpY2tIb3VzZSBDbG91ZCAoYXdzKSI6dHJ1ZSwiQ2xpY2tIb3VzZSBDbG91ZCAoZ2NwKSI6dHJ1ZSwiQ2xpY2tIb3VzZSAyMy4xMSAoZGF0YSBsYWtlLCBwYXJ0aXRpb25lZCkiOnRydWUsIkNsaWNrSG91c2UgMjMuMTEgKGRhdGEgbGFrZSwgc2luZ2xlKSI6dHJ1ZSwiQ2xpY2tIb3VzZSAyMy4xMSAoUGFycXVldCwgcGFydGl0aW9uZWQpIjp0cnVlLCJDbGlja0hvdXNlIDIzLjExIChQYXJxdWV0LCBzaW5nbGUpIjp0cnVlLCJDbGlja0hvdXNlIDIzLjExICh3ZWIpIjp0cnVlLCJDbGlja0hvdXNlIjp0cnVlLCJDbGlja0hvdXNlICh0dW5lZCkiOnRydWUsIkNsaWNrSG91c2UgMjMuMTEiOnRydWUsIkNsaWNrSG91c2UgKHpzdGQpIjp0cnVlLCJDcmF0ZURCIjp0cnVlLCJEYXRhYmVuZCI6dHJ1ZSwiRGF0YUZ1c2lvbiAoUGFycXVldCwgcGFydGl0aW9uZWQpIjp0cnVlLCJEYXRhRnVzaW9uIChQYXJxdWV0LCBzaW5nbGUpIjp0cnVlLCJBcGFjaGUgRG9yaXMiOnRydWUsIkRydWlkIjp0cnVlLCJEdWNrREIgKFBhcnF1ZXQsIHBhcnRpdGlvbmVkKSI6dHJ1ZSwiRHVja0RCIjp0cnVlLCJFbGFzdGljc2VhcmNoIjp0cnVlLCJFbGFzdGljc2VhcmNoICh0dW5lZCkiOnRydWUsIkdyZWVucGx1bSI6dHJ1ZSwiSGVhdnlBSSI6dHJ1ZSwiSHlkcmEiOnRydWUsIkluZm9icmlnaHQiOnRydWUsIktpbmV0aWNhIjp0cnVlLCJNYXJpYURCIENvbHVtblN0b3JlIjp0cnVlLCJNYXJpYURCIjp0cnVlLCJNb25ldERCIjp0cnVlLCJNb25nb0RCIjp0cnVlLCJNeVNRTCAoTXlJU0FNKSI6dHJ1ZSwiTXlTUUwiOnRydWUsIlBhcmFkZURCIjp0cnVlLCJQaW5vdCI6dHJ1ZSwiUG9zdGdyZVNRTCAodHVuZWQpIjp0cnVlLCJQb3N0Z3JlU1FMIjp0cnVlLCJRdWVzdERCIChwYXJ0aXRpb25lZCkiOnRydWUsIlF1ZXN0REIiOnRydWUsIlJlZHNoaWZ0Ijp0cnVlLCJTZWxlY3REQiI6dHJ1ZSwiU2luZ2xlU3RvcmUiOnRydWUsIlNub3dmbGFrZSI6dHJ1ZSwiU1FMaXRlIjp0cnVlLCJTdGFyUm9ja3MiOnRydWUsIlRpbWVzY2FsZURCIChjb21wcmVzc2lvbikiOnRydWUsIlRpbWVzY2FsZURCIjp0cnVlfSwidHlwZSI6eyJDIjpmYWxzZSwiY29sdW1uLW9yaWVudGVkIjpmYWxzZSwiUG9zdGdyZVNRTCBjb21wYXRpYmxlIjp0cnVlLCJtYW5hZ2VkIjpmYWxzZSwiZ2NwIjpmYWxzZSwic3RhdGVsZXNzIjpmYWxzZSwiSmF2YSI6ZmFsc2UsIkMrKyI6ZmFsc2UsIk15U1FMIGNvbXBhdGlibGUiOmZhbHNlLCJyb3ctb3JpZW50ZWQiOmZhbHNlLCJDbGlja0hvdXNlIGRlcml2YXRpdmUiOmZhbHNlLCJlbWJlZGRlZCI6ZmFsc2UsInNlcnZlcmxlc3MiOmZhbHNlLCJhd3MiOmZhbHNlLCJSdXN0IjpmYWxzZSwic2VhcmNoIjpmYWxzZSwiZG9jdW1lbnQiOmZhbHNlLCJ0aW1lLXNlcmllcyI6ZmFsc2V9LCJtYWNoaW5lIjp7IjE2IHZDUFUgMTI4R0IiOnRydWUsIjggdkNQVSA2NEdCIjp0cnVlLCJzZXJ2ZXJsZXNzIjp0cnVlLCIxNmFjdSI6dHJ1ZSwiYzZhLjR4bGFyZ2UsIDUwMGdiIGdwMiI6dHJ1ZSwiTCI6dHJ1ZSwiTSI6dHJ1ZSwiUyI6dHJ1ZSwiWFMiOnRydWUsImM2YS5tZXRhbCwgNTAwZ2IgZ3AyIjp0cnVlLCIxOTJHQiI6dHJ1ZSwiMjRHQiI6dHJ1ZSwiMzYwR0IiOnRydWUsIjQ4R0IiOnRydWUsIjcyMEdCIjp0cnVlLCI5NkdCIjp0cnVlLCIxNDMwR0IiOnRydWUsImRldiI6dHJ1ZSwiNzA4R0IiOnRydWUsImM1bi40eGxhcmdlLCA1MDBnYiBncDIiOnRydWUsImM1LjR4bGFyZ2UsIDUwMGdiIGdwMiI6dHJ1ZSwibTVkLjI0eGxhcmdlIjp0cnVlLCJtNmkuMzJ4bGFyZ2UiOnRydWUsImM2YS40eGxhcmdlLCAxNTAwZ2IgZ3AyIjp0cnVlLCJkYzIuOHhsYXJnZSI6dHJ1ZSwicmEzLjE2eGxhcmdlIjp0cnVlLCJyYTMuNHhsYXJnZSI6dHJ1ZSwicmEzLnhscGx1cyI6dHJ1ZSwiUzIiOnRydWUsIlMyNCI6dHJ1ZSwiMlhMIjp0cnVlLCIzWEwiOnRydWUsIjRYTCI6dHJ1ZSwiWEwiOnRydWV9LCJjbHVzdGVyX3NpemUiOnsiMSI6dHJ1ZSwiMiI6dHJ1ZSwiNCI6dHJ1ZSwiOCI6dHJ1ZSwiMTYiOnRydWUsIjMyIjp0cnVlLCI2NCI6dHJ1ZSwiMTI4Ijp0cnVlLCJzZXJ2ZXJsZXNzIjp0cnVlLCJkZWRpY2F0ZWQiOnRydWUsInVuZGVmaW5lZCI6dHJ1ZX0sIm1ldHJpYyI6ImhvdCIsInF1ZXJpZXMiOlt0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlXX0=).

## Getting Started

This toy example demonstrates how to get started.

```sql
CREATE EXTENSION pg_analytics;
-- Create a parquet table
CREATE TABLE t (a int) USING parquet;
-- pg_analytics supercharges the performance of any
-- Postgres query run on a parquet table
INSERT INTO t VALUES (1), (2), (3);
SELECT COUNT(*) FROM t;
```

## Deltalake Tables

You can interact with `parquet` tables the same way as with normal Postgres tables. However, there are a few operations specific to `parquet` tables.

### Storage Optimization

When Parquet files are dropped, they remain on disk until `VACUUM` is run. This operation physically
deletes the Parquet files of dropped tables.

The `VACUUM FULL <table_name>` command is used to optimize a table's storage by bin-packing small Parquet
files into larger files, which can significantly improve query time and compression. It also deletes dropped Parquet
files.

## Roadmap

`pg_analytics` is currently in beta.

### Features Supported

- [x] `parquet` tables behave like regular Postgres tables and support most Postgres queries (JOINs, CTEs, window functions, etc.)
- [x] Vacuum and Parquet storage optimization
- [x] `INSERT`, `TRUNCATE`, `DELETE`, `COPY`
- [x] Physical backups with `pg_dump`

### Known Limitations

As `pg_analytics` becomes production-ready, many of these will be resolved.

- [ ] `UPDATE` statements
- [ ] Nested `DELETE` statements
- [ ] Partitioning tables by column
- [ ] Some Postgres types like JSON, time, and timestamp with time zone
- [ ] User-defined functions, aggregations, or types
- [ ] Referencing `parquet` and regular Postgres `heap` tables in the same query
- [ ] Write-ahead-log (WAL) support/`ROLLBACK`/logical replication
- [ ] Foreign keys
- [ ] Index scans
- [ ] Collations
- [ ] External object store integrations (S3/Azure/GCS/HDFS)
- [ ] External Apache Iceberg and Delta Lake support
- [ ] Full text search over `parquet` tables with `pg_bm25`

## Development

### Install Rust

To develop the extension, first install Rust v1.76.0 using `rustup`. We will soon make the extension compatible with newer versions of Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install 1.76.0

# We recommend setting the default version to 1.76.0 for consistency across your system
rustup default 1.76.0
```

Note: While it is possible to install Rust via your package manager, we recommend using `rustup` as we've observed inconcistencies with Homebrew's Rust installation on macOS.

Then, install the PostgreSQL version of your choice using your system package manager. Here we provide the commands for the default PostgreSQL version used by this project:

### Install Postgres

```bash
# macOS
brew install postgresql@16

# Ubuntu
wget --quiet -O - https://www.postgresql.org/media/keys/ACCC4CF8.asc | sudo apt-key add -
sudo sh -c 'echo "deb http://apt.postgresql.org/pub/repos/apt/ $(lsb_release -cs)-pgdg main" > /etc/apt/sources.list.d/pgdg.list'
sudo apt-get update && sudo apt-get install -y postgresql-16 postgresql-server-dev-16
```

If you are using Postgres.app to manage your macOS PostgreSQL, you'll need to add the `pg_config` binary to your path before continuing:

```bash
export PATH="$PATH:/Applications/Postgres.app/Contents/Versions/latest/bin"
```

### Install pgrx

Then, install and initialize `pgrx`:

```bash
# Note: Replace --pg16 with your version of Postgres, if different (i.e. --pg15, --pg14, etc.)
cargo install --locked cargo-pgrx --version 0.11.2

# macOS arm64
cargo pgrx init --pg16=/opt/homebrew/opt/postgresql@16/bin/pg_config

# macOS amd64
cargo pgrx init --pg16=/usr/local/opt/postgresql@16/bin/pg_config

# Ubuntu
cargo pgrx init --pg16=/usr/lib/postgresql/16/bin/pg_config
```

If you prefer to use a different version of Postgres, update the `--pg` flag accordingly.

Note: While it is possible to develop using pgrx's own Postgres installation(s), via `cargo pgrx init` without specifying a `pg_config` path, we recommend using your system package manager's Postgres as we've observed inconsistent behaviours when using pgrx's.

### Configure Shared Preload Libraries

This extension uses Postgres hooks to intercept Postgres queries. In order to enable these hooks, the extension
must be added to `shared_preload_libraries` inside `postgresql.conf`. If you are using Postgres 16, this file can be found under `~/.pgrx/data-16`.

```bash
# Inside postgresql.conf
shared_preload_libraries = 'pg_analytics'
```

### Run Without Optimized Build

The extension can be developed with or without an optimized build. An optimized build improves query times by 10-20x but also significantly increases build times.

To launch the extension without an optimized build, run

```bash
cargo pgrx run
```

### Run With Optimized Build

First, switch to latest Rust Nightly (as of writing, 1.77) via:

```bash
rustup update nightly
rustup override set nightly
```

Then, reinstall `pgrx` for the new version of Rust:

```bash
cargo install --locked cargo-pgrx --version 0.11.2 --force
```

Finally, run to build in release mode with SIMD:

```bash
cargo pgrx run --release
```

Note that this may take several minutes to execute.

To revert back to the stable version of Rust, run:

```bash
rustup override unset
```

### Run Benchmarks

To run benchmarks locally, enter the `pg_analytics/` directory and run `cargo clickbench`. This runs a minified version of the ClickBench benchmark suite on `pg_analytics`.

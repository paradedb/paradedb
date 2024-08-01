<h1 align="center">
  <img src="../docs/logo/pg_analytics.svg" alt="pg_analytics" width="500px">
<br>
</h1>

[![Test pg_analytics](https://github.com/paradedb/paradedb/actions/workflows/test-pg_analytics.yml/badge.svg)](https://github.com/paradedb/paradedb/actions/workflows/test-pg_analytics.yml)
[![Benchmark pg_analytics](https://github.com/paradedb/paradedb/actions/workflows/benchmark-pg_analytics.yml/badge.svg)](https://github.com/paradedb/paradedb/actions/workflows/benchmark-pg_analytics.yml)

## Overview

`pg_analytics` puts DuckDB inside Postgres.

With `pg_analytics` installed, Postgres can query foreign object stores like S3 and table formats like Iceberg or Delta Lake. Queries are pushed down to DuckDB, a high performance analytical query engine. The following object stores and table formats are supported:

### Motivation

Today, a vast amount of non-operational data — events, metrics, historical snapshots, vendor data, etc. — is ingested into data lakes like S3. Querying this data by moving it into a cloud data warehouse or operating a new query engine is expensive and time-consuming. The goal of `pg_analytics` is to enable this data to be queried directly from Postgres. This eliminates the need for new infrastructure, loss of data freshness, data movement, and non-Postgres dialects of other query engines.

`pg_analytics` uses the foreign data wrapper (FDW) API to connect to any object store or table format and the executor hook API to push queries to DuckDB. While other FDWs like `aws_s3` have existed in the Postgres extension ecosystem, these FDWs suffer from two limitations:

1. Lack of support for most object stores and table formats
2. Too slow over large datasets to be a viable analytical engine

`pg_analytics` differentiates itself by supporting a wide breadth of stores and formats and by being very fast (thanks to DuckDB).

### Roadmap

- [ ] Read support for `pg_analytics`
- [ ] Write support for `pg_analytics`
- [ ] `EXPLAIN` support
- [ ] Automatic schema detection
- [ ] Integration with the catalog providers

#### Object Stores

- [x] Amazon S3
- [x] S3-compatible stores (MinIO, R2)
- [x] Azure Blob Storage
- [x] Azure Data Lake Storage Gen2
- [x] Google Cloud Storage
- [x] HTTP server
- [x] Local file system

#### Table Formats

- [x] Parquet
- [x] CSV
- [x] Apache Iceberg
- [x] Delta Lake
- [ ] JSON (Coming Soon)

`pg_analytics` uses DuckDB v1.0.0 and is supported on Postgres 14, 15, and 16. Support for Postgres 12 and 13 is coming soon.

## Installation

### From ParadeDB

The easiest way to use the extension is to run the ParadeDB Dockerfile:

```bash
docker run \
  --name paradedb \
  -e POSTGRES_USER=<user> \
  -e POSTGRES_PASSWORD=<password> \
  -e POSTGRES_DB=<dbname> \
  -v paradedb_data:/var/lib/postgresql/ \
  -p 5432:5432 \
  -d \
  paradedb/paradedb:latest
```

This will spin up a Postgres instance with `pg_analytics` preinstalled.

### From Self-Hosted PostgreSQL

If you are self-hosting Postgres and would like to use the extension within your existing Postgres, follow the steps below.

It's **very important** to make the following change to your `postgresql.conf` configuration file. `pg_analytics` must be in the list of `shared_preload_libraries`:

```c
shared_preload_libraries = 'pg_analytics'
```

This ensures the best query performance from the extension .

#### Debian/Ubuntu

We provide prebuilt binaries for Debian-based Linux for Postgres 16, 15 and 14. You can download the latest version for your architecture from the [releases page](https://github.com/paradedb/paradedb/releases).

ParadeDB collects anonymous telemetry to help us understand how many people are using the project. You can opt out of telemetry by setting `export PARADEDB_TELEMETRY=false` (or unsetting the variable) in your shell or in your `~/.bashrc` file before running the extension.

#### macOS

We don't suggest running production workloads on macOS. As a result, we don't provide prebuilt binaries for macOS. If you are running Postgres on macOS and want to install `pg_analytics`, please follow the [development](#development) instructions, but do `cargo pgrx install --release` instead of `cargo pgrx run`. This will build the extension from source and install it in your Postgres instance.

You can then create the extension in your database by running:

```sql
CREATE EXTENSION pg_analytics;
```

Note: If you are using a managed Postgres service like Amazon RDS, you will not be able to install `pg_analytics` until the Postgres service explicitly supports it.

#### Windows

Windows is not supported. This restriction is [inherited from pgrx not supporting Windows](https://github.com/pgcentralfoundation/pgrx?tab=readme-ov-file#caveats--known-issues).

## Usage

The following example uses `pg_analytics` to query an example dataset of 3 million NYC taxi trips from January 2024, hosted in a public `us-east-1` S3 bucket provided by ParadeDB.

```sql
CREATE EXTENSION pg_analytics;
CREATE FOREIGN DATA WRAPPER parquet_wrapper HANDLER parquet_fdw_handler VALIDATOR parquet_fdw_validator;

-- Provide S3 credentials
CREATE SERVER parquet_server FOREIGN DATA WRAPPER parquet_wrapper;

-- Create foreign table with auto schema creation
CREATE FOREIGN TABLE trips ()
SERVER parquet_server
OPTIONS (files 's3://paradedb-benchmarks/yellow_tripdata_2024-01.parquet');

-- Success! Now you can query the remote Parquet file like a regular Postgres table
SELECT COUNT(*) FROM trips;
  count
---------
 2964624
(1 row)
```

To query your own data, please refer to the [documentation](https://docs.paradedb.com/analytics/object_stores).

## Shared Preload Libraries

Because this extension uses Postgres hooks to intercept and push queries down to DuckDB, it is **very important** that it is added to `shared_preload_libraries` inside `postgresql.conf`.

```bash
# Inside postgresql.conf
shared_preload_libraries = 'pg_analytics'
```

## Development

### Install Rust

To develop the extension, first install Rust via `rustup`.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install <version>

rustup default <version>
```

Note: While it is possible to install Rust via your package manager, we recommend using `rustup` as we've observed inconsistencies with Homebrew's Rust installation on macOS.

Then, install the PostgreSQL version of your choice using your system package manager. Here we provide the commands for the default PostgreSQL version used by this project:

### Install Other Dependencies

Before compiling the extension, you'll need to have the following dependencies installed.

```bash
# macOS
brew install make gcc pkg-config openssl

# Ubuntu
sudo apt-get install -y make gcc pkg-config libssl-dev

# Arch Linux
sudo pacman -S core/openssl
```

### Install Postgres

```bash
# macOS
brew install postgresql@16

# Ubuntu
wget --quiet -O - https://www.postgresql.org/media/keys/ACCC4CF8.asc | sudo apt-key add -
sudo sh -c 'echo "deb http://apt.postgresql.org/pub/repos/apt/ $(lsb_release -cs)-pgdg main" > /etc/apt/sources.list.d/pgdg.list'
sudo apt-get update && sudo apt-get install -y postgresql-16 postgresql-server-dev-16

# Arch Linux
sudo pacman -S extra/postgresql
```

If you are using Postgres.app to manage your macOS PostgreSQL, you'll need to add the `pg_config` binary to your path before continuing:

```bash
export PATH="$PATH:/Applications/Postgres.app/Contents/Versions/latest/bin"
```

### Install pgrx

Then, install and initialize `pgrx`:

```bash
# Note: Replace --pg16 with your version of Postgres, if different (i.e. --pg15, --pg14, etc.)
cargo install --locked cargo-pgrx --version 0.11.3

# macOS arm64
cargo pgrx init --pg16=/opt/homebrew/opt/postgresql@16/bin/pg_config

# macOS amd64
cargo pgrx init --pg16=/usr/local/opt/postgresql@16/bin/pg_config

# Ubuntu
cargo pgrx init --pg16=/usr/lib/postgresql/16/bin/pg_config

# Arch Linux
cargo pgrx init --pg16=/usr/bin/pg_config
```

If you prefer to use a different version of Postgres, update the `--pg` flag accordingly.

Note: While it is possible to develop using pgrx's own Postgres installation(s), via `cargo pgrx init` without specifying a `pg_config` path, we recommend using your system package manager's Postgres as we've observed inconsistent behaviours when using pgrx's.

`pgrx` requires `libclang`. To install it:

```bash
# Ubuntu
sudo apt install libclang-dev

# Arch Linux
sudo pacman -S extra/clang
```

### Running Tests

We use `cargo test` as our runner for `pg_analytics` tests. Tests are conducted using [testcontainers](https://github.com/testcontainers/testcontainers-rs) to manage testing containers like [LocalStack](https://hub.docker.com/r/localstack/localstack). `testcontainers` will pull any Docker images that it requires to perform the test.

You also need a running Postgres instance to run the tests. The test suite will look for a connection string on the `DATABASE_URL` environment variable. You can set this variable manually, or use `.env` file with contents like this:

```text
DATABASE_URL=postgres://<username>@<host>:<port>/<database>
```

## License

`pg_analytics` is licensed under the [GNU Affero General Public License v3.0](../LICENSE) and as commercial software. For commercial licensing, please contact us at [sales@paradedb.com](mailto:sales@paradedb.com).

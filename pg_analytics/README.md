# pg_analytics

`pg_analytics` is a Postgres extension that enables Clickhouse-level analytics (OLAP) performance inside Postgres. `pg_analytics` achieves this using Apache Parquet for storage, Apache Arrow for column-oriented memory, and Apache Datafusion for vectorized query execution with SIMD.

## Getting Started

The following toy example shows how to get started with `pg_analytics`.

```sql
CREATE EXTENSION pg_analytics;
-- This needs to be run once per connection
CALL paradedb.init();
-- USING analytics indicates that we are creating an analytics table
CREATE TABLE t (a int) USING analytics;
-- Now you can any Postgres query
INSERT INTO t VALUES (1), (2), (3);
SELECT COUNT(*) FROM t;
```

## Development

### Install Rust

To develop the extension, first install Rust v1.73.0 using `rustup`. We will soon make the extension compatible with newer versions of Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install 1.73.0

# We recommend setting the default version for consistency
rustup default 1.73.0
```

Note: While it is possible to install Rust via your package manager, we recommend using `rustup` as we've observed inconcistencies with Homebrew's Rust installation on macOS.

Then, install the PostgreSQL version of your choice using your system package manager. Here we provide the commands for the default PostgreSQL version used by this project:

### Install Postgres

```bash
# macOS
brew install postgresql@15

# Ubuntu
wget --quiet -O - https://www.postgresql.org/media/keys/ACCC4CF8.asc | sudo apt-key add -
sudo sh -c 'echo "deb http://apt.postgresql.org/pub/repos/apt/ $(lsb_release -cs)-pgdg main" > /etc/apt/sources.list.d/pgdg.list'
sudo apt-get update && sudo apt-get install -y postgresql-$15 postgresql-server-dev-15
```

If you are using Postgres.app to manage your macOS PostgreSQL, you'll need to add the `pg_config` binary to your path before continuing:

```bash
export PATH="$PATH:/Applications/Postgres.app/Contents/Versions/latest/bin"
```

### Install pgrx

Then, install and initialize `pgrx`:

```bash
# Note: Replace --pg15 with your version of Postgres, if different (i.e. --pg16, --pg14, etc.)
cargo install --locked cargo-pgrx --version 0.11.1
cargo pgrx init --pg15=`which pg_config`
```

### Configure Shared Preload Libraries

This extension uses Postgres hooks to intercept Postgres queries. In order to enable these hooks, the extension
must be added to `shared_preload_libraries` inside `postgresql.conf`. If you are using Postgres 15, this file can be found under `~/.pgrx/data-15`.

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
cargo install --locked cargo-pgrx --version 0.11.1 --force
```

Finally, run to build in release mode with SIMD:

```bash
cargo pgrx run --features simd --release
```

Note that this may take several minutes to execute.

To revert back to the stable version of Rust, run:

```bash
rustup override unset
```

## Benchmarks

To run benchmarks locally, first enter the `pg_analytics/` directory before running `cargo clickbench`. This runs a minified version of the ClickBench benchmark suite on `pg_analytics`.

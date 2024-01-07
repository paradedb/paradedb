# pg_columnar

`pg_columnar` enables Clickhouse-level OLAP performance inside Postgres. By embedding Apache Datafusion inside Postgres, this extension brings column-oriented storage, vectorized query execution, and SIMD instructions to Postgres tables.

## Development

### Prerequisites

#### Install Rust

To develop the extension, first install Rust v1.73.0 using `rustup`. We will soon make the extension compatible with newer versions of Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install 1.73.0

# We recommend setting the default version for consistency
rustup default 1.73.0
```

Note: While it is possible to install Rust via your package manager, we recommend using `rustup` as we've observed inconcistencies with Homebrew's Rust installation on macOS.

Then, install the PostgreSQL version of your choice using your system package manager. Here we provide the commands for the default PostgreSQL version used by this project:

#### Install Postgres

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

#### Install pgrx

Then, install and initialize pgrx:

```bash
# Note: Replace --pg15 with your version of Postgres, if different (i.e. --pg16, --pg14, etc.)
cargo install --locked cargo-pgrx --version 0.11.1
cargo pgrx init --pg15=`which pg_config`
```

The extension can be developed with or without SIMD enabled. Enabling SIMD improves query times by 10-20x but also significantly increases build times. For fast development iteration, we recommend disabling SIMD.

#### Configure Shared Preload Libraries

This extension uses Postgres hooks to intercept Postgres queries. In order to enable these hooks, the extension
must be added to `shared_preload_libraries` inside `postgresql.conf`. If you are using Postgres 15, this file can be found under `~/.pgrx/data-15`.

```bash
# Inside postgresql.conf
shared_preload_libraries = 'pg_columnar'
```

### SIMD Disabled

To launch the extension with SIMD disabled, run

```bash
cargo pgrx run
```

### SIMD Enabled

First, switch to latest Rust Nightly (as of writing, 1.77) via:

```bash
rustup update nightly
rustup override set nightly
```

Then, reinstall pgrx for the new version of Rust:

```bash
cargo install --locked cargo-pgrx --version 0.11.1 --force
```

Finally, run to enable SIMD:

```bash
# To build in development mode, with SIMD enabled
cargo pgrx run --features simd

# To build in release mode, with SIMD enabled
cargo pgrx run --features simd --release
```

Note that this may take several minutes to execute.

To revert back to the stable version of Rust, run:

```bash
rustup override unset
```

## Benchmarks

To run benchmarks locally for development, first enter the `pg_columnar/` directory before running `cargo clickbench`. This runs a minified version of the ClickBench benchmark suite on a purely in-memory version of `pg_columnar`. As of writing, this is the only functional benchmark suite as we haven't built persistence in our TableAM. Once we do, you can run the full suite using on-disk storage via `cargo clickbench cold`.

what ot do with the benchmarks

both in /benchmarks

or both in benchmark/ in their respective subproject (?)

or I put the specific for each extension benchmarking directly in it, and in the benchmarks/ I just put the benchmarks for workflows which combine both types of workloads?

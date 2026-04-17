<h1 align="center">
  <img src="../docs/logo/pg_search.svg" alt="pg_search"></a>
<br>
</h1>

This README covers **local development** of the `pg_search` extension. For installation, deployment, and usage, see the [top-level ParadeDB README](../README.md) or the [ParadeDB documentation](https://docs.paradedb.com).

`pg_search` is supported on official PostgreSQL Global Development Group Postgres versions, starting at PostgreSQL 15.

## Prerequisites

### Rust

Install stable Rust using `rustup`:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install stable
```

Note: While it is possible to install Rust via your package manager, we recommend using `rustup` as we've observed inconsistencies with Homebrew's Rust installation on macOS.

### PostgreSQL

Install the PostgreSQL version of your choice using your system package manager. Here we provide the commands for the default PostgreSQL version used by this project:

```bash
# macOS
brew install postgresql@18

# Ubuntu
wget --quiet -O - https://www.postgresql.org/media/keys/ACCC4CF8.asc | sudo apt-key add -
sudo sh -c 'echo "deb http://apt.postgresql.org/pub/repos/apt/ $(lsb_release -cs)-pgdg main" > /etc/apt/sources.list.d/pgdg.list'
sudo apt-get update && sudo apt-get install -y postgresql-18 postgresql-server-dev-18

# Arch Linux
sudo pacman -S extra/postgresql
```

If you are using Postgres.app to manage your macOS PostgreSQL, add the `pg_config` binary to your path before continuing:

```bash
export PATH="$PATH:/Applications/Postgres.app/Contents/Versions/latest/bin"
```

### pgrx

The `cargo-pgrx` version must match the `pgrx` dependency declared in [`pg_search/Cargo.toml`](Cargo.toml). Install and initialize it for your Postgres version:

```bash
# Note: Replace --pg18 with your version of Postgres, if different (i.e. --pg17, etc.)
cargo install --locked cargo-pgrx --version 0.18.0

# macOS arm64
cargo pgrx init --pg18=/opt/homebrew/opt/postgresql@18/bin/pg_config

# macOS amd64
cargo pgrx init --pg18=/usr/local/opt/postgresql@18/bin/pg_config

# Ubuntu
cargo pgrx init --pg18=/usr/lib/postgresql/18/bin/pg_config

# Arch Linux
cargo pgrx init --pg18=/usr/bin/pg_config
```

Note: While it is possible to develop using pgrx's own Postgres installation(s), via `cargo pgrx init` without specifying a `pg_config` path, we recommend using your system package manager's Postgres as we've observed inconsistent behaviours when using pgrx's.

`pgrx` requires `libclang`, which does not come by default on Linux:

```bash
# Ubuntu
sudo apt install libclang-dev

# Arch Linux
sudo pacman -S extra/clang
```

Windows is not supported. This restriction is [inherited from pgrx not supporting Windows](https://github.com/pgcentralfoundation/pgrx?tab=readme-ov-file#caveats--known-issues).

### pgvector

`pgvector` is needed for hybrid search integration tests:

```bash
# Note: Replace 18 with your version of Postgres
git clone --branch v0.8.1 https://github.com/pgvector/pgvector.git
cd pgvector/

# macOS arm64
PG_CONFIG=/opt/homebrew/opt/postgresql@18/bin/pg_config make
sudo PG_CONFIG=/opt/homebrew/opt/postgresql@18/bin/pg_config make install # may need sudo

# macOS amd64
PG_CONFIG=/usr/local/opt/postgresql@18/bin/pg_config make
sudo PG_CONFIG=/usr/local/opt/postgresql@18/bin/pg_config make install # may need sudo

# Ubuntu
PG_CONFIG=/usr/lib/postgresql/18/bin/pg_config make
sudo PG_CONFIG=/usr/lib/postgresql/18/bin/pg_config make install # may need sudo

# Arch Linux
PG_CONFIG=/usr/bin/pg_config make
sudo PG_CONFIG=/usr/bin/pg_config make install # may need sudo
```

## Running the Extension

Start an interactive Postgres session with the extension built and loaded:

```bash
cargo pgrx run
```

Inside Postgres, create the extension:

```sql
CREATE EXTENSION pg_search;
```

## Modifying the Extension

After making changes to the extension code:

1. Recompile and start Postgres:

   ```bash
   cargo pgrx run
   ```

2. Recreate the extension to load the latest changes:

   ```sql
   DROP EXTENSION pg_search;
   CREATE EXTENSION pg_search;
   ```

## Testing

Unit tests live in `pg_search/src` and run with:

```bash
cargo test -p pg_search -- a_specific_method_to_run
```

Tests marked `#[pg_test]` run inside the Postgres process and can use the full `pgrx` API. The annotation handles re-installing the extension automatically — no manual install needed.

For the other test categories (pg regress, integration tests, client property tests), see:

- [`pg_search/tests/pg_regress/README.md`](tests/pg_regress/README.md) — pg regress tests
- [`tests/README.md`](../tests/README.md) — integration tests and client property tests
- [`CONTRIBUTING.md#testing`](../CONTRIBUTING.md#testing) — overview of all test categories and when to use which

## Benchmarks

See [`benchmarks/README.md`](../benchmarks/README.md).

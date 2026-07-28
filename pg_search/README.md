<h1 align="center">
  <img src="../docs/logo/pg_search.svg" alt="pg_search"></a>
<br>
</h1>

This README covers development of the `pg_search` extension. For installation, deployment, and usage, see the [top-level ParadeDB README](../README.md) or the [ParadeDB documentation](https://docs.paradedb.com).

`pg_search` is supported on official PostgreSQL Global Development Group Postgres versions, starting at PostgreSQL 15.

## Prerequisites

### Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install stable
```

### pgrx

The `cargo-pgrx` version must match the `pgrx` dependency declared in [`pg_search/Cargo.toml`](Cargo.toml). On Linux, `pgrx` also requires `libclang`:

```bash
# Ubuntu
sudo apt install libclang-dev

# Arch Linux
sudo pacman -S extra/clang
```

Then install `cargo-pgrx` and let it bootstrap a managed PostgreSQL installation under `~/.pgrx/`:

```bash
cargo install --locked cargo-pgrx --version 0.18.1
# On macOS, if `cargo pgrx init` fails with ICU-related errors, run
# `brew install icu4c`
# and then run
# `export PKG_CONFIG_PATH="/opt/homebrew/opt/icu4c@78/lib/pkgconfig:$PKG_CONFIG_PATH"`
# (the icu4c version may differ depending on what Homebrew has installed):
cargo pgrx init
```

`cargo pgrx init` builds every supported Postgres version this project targets (currently 15–18) into `~/.pgrx/<version>/pgrx-install/` and points future `cargo pgrx` commands at it — no system Postgres required. To target only a single version, pass e.g. `cargo pgrx init --pg18 download`.

### pgvector

`pgvector` is needed for hybrid search integration tests. To build it against the pgrx-managed Postgres install (replace `18.3` with the version under `~/.pgrx/`):

```bash
git clone --branch v0.8.4 https://github.com/pgvector/pgvector.git
cd pgvector/

PG_CONFIG=~/.pgrx/18.3/pgrx-install/bin/pg_config make
PG_CONFIG=~/.pgrx/18.3/pgrx-install/bin/pg_config make install
```

## Running the Extension

`pg_search` must be loaded via `shared_preload_libraries`. Before running for the first time, add it to the pgrx-managed `postgresql.conf` (replace `18` with your target version):

```bash
echo "shared_preload_libraries = 'pg_search'" >> ~/.pgrx/data-18/postgresql.conf
```

Then start an interactive Postgres session with the extension built and loaded:

```bash
cargo pgrx run
```

Inside Postgres, create the extension (`CASCADE` also creates `pgvector`, which
`pg_search` requires):

```sql
CREATE EXTENSION pg_search CASCADE;
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
   CREATE EXTENSION pg_search CASCADE;
   ```

## Testing

Unit tests live in `pg_search/src` and run with:

```bash
cargo test -p pg_search -- a_specific_method_to_run
```

Tests marked `#[pg_test]` run inside the Postgres process and can use the full `pgrx` API. The annotation automatically reinstalls the extension — no manual install needed.

For the other test categories (pg regress, integration tests, client property tests, stress tests, upgrade tests), see:

- [`pg_search/tests/pg_regress/README.md`](tests/pg_regress/README.md) — pg_regress tests
- [`tests/README.md`](../tests/README.md) — integration tests and client property tests
- [`stressgres/README.md`](../stressgres/README.md) — Stressgres, the stress-testing tool used locally and in CI
- [`CONTRIBUTING.md#testing`](../CONTRIBUTING.md#testing) — overview of all test categories and when to use which
- [`test-pg_search-upgrade/README.md`](../.github/actions/test-pg_search-upgrade/README.md) - compatibility tests for extension version upgrades

## Benchmarks

See [`benchmarks/README.md`](../benchmarks/README.md).

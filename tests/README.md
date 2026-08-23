# Test suite for `pg_search`

This directory contains the **integration tests** and **client property tests** for the `pg_search` extension. Because these tests run outside the Postgres process, they require the extension to already be installed.

For a complete overview of ParadeDB's testing infrastructure (including unit tests and pg regress tests), please see the [Testing section in `CONTRIBUTING.md`](../CONTRIBUTING.md#testing).

## Client Property Tests

Client property tests are a particularly interesting subcategory of integration tests. Most live in [`qgen.rs`](tests/qgen.rs), but other files also use `crate::fixtures::querygen` to generate tests.

## Environment Variables

The tests require a `DATABASE_URL` environment variable to be set. The easiest way to do this is to create a `.env` file with the following contents:

```env
DATABASE_URL=postgres://USER_NAME@localhost:PORT/pg_search
```

`USER_NAME` should be replaced with your system username (for example, the output of `whoami`).

`PORT` should be replaced with 28800 plus your PostgreSQL major version (for example, 28818 for PostgreSQL 18).

Some tests also require `PG_CONFIG` to point to the `pg_config` binary for the PostgreSQL installation under test. The logical-replication test requires it, while the dump/restore test is skipped when it is not set.

## Running Tests with pgrx-managed PostgreSQL

If you are using pgrx’s bundled PostgreSQL, follow these steps from the root of the repository:

```shell
#!/bin/sh

set -x
export DATABASE_URL="postgresql://$(whoami)@localhost:28818/pg_search"
export PG_CONFIG="$HOME/.pgrx/18.6/pgrx-install/bin/pg_config"
export RUST_BACKTRACE=1
cargo pgrx stop --package pg_search
cargo pgrx install --package pg_search --pg-config "$PG_CONFIG"
cargo pgrx start --package pg_search
createdb -h localhost -p 28818 pg_search || true

cargo test --package tests
```

## Running Tests with a Self-Hosted PostgreSQL

If you are using a self-hosted PostgreSQL installation, make sure your PostgreSQL server is already running, create a `pg_search` database on it,
and install the `pg_search` extension files into that PostgreSQL instance instead of pgrx's bundled Postgres.
The example below uses Homebrew's PostgreSQL 18 path; replace `PG_CONFIG` with the path to your installation's `pg_config` binary.

```shell
#!/bin/sh

set -x
export DATABASE_URL="postgresql://$(whoami)@localhost:5432/pg_search"
export PG_CONFIG=/opt/homebrew/opt/postgresql@18/bin/pg_config
export RUST_BACKTRACE=1

createdb pg_search || true
cargo pgrx install --package pg_search --pg-config "$PG_CONFIG"

cargo test --package tests
```

To run a single test, use the following command (replace `<testname>` with the test file name without the `.rs` extension):

```shell
cargo test --package tests --test <testname>
```

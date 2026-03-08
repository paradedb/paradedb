# Test suite for `pg_search`

This directory contains the **integration tests** and **client property tests** for the `pg_search` extension. Because these tests run outside the Postgres process, they require the extension to already be installed.

For a complete overview of ParadeDB's testing infrastructure (including unit tests and pg regress tests), please see the [Testing section in `CONTRIBUTING.md`](../CONTRIBUTING.md#testing).

## Client Property Tests

A particularly interesting subcategory of integration tests are our client property tests. Most of the client property tests live in [`tests/qgen.rs`](tests/qgen.rs), but there are other files which use `crate::fixtures::querygen` to generate tests as well.

## Environment Variables

The tests require a `DATABASE_URL` environment variable to be set. The easiest way to do this is to create a `.env` file with the following contents:

```env
DATABASE_URL=postgres://USER_NAME@localhost:PORT/pg_search
```

USER_NAME should be replaced with your system user name. (eg: output of `whoami`)

PORT should be replaced with 28800 + your postgres version. (eg: 28818 for Postgres 18)

## Running Tests with pgrx-managed PostgreSQL

If you are using pgrx’s bundled PostgreSQL, follow these steps from the root of the repository:

```shell
#! /bin/sh

set -x
export DATABASE_URL=postgresql://localhost:28818/pg_search
export RUST_BACKTRACE=1
cargo pgrx stop --package pg_search
cargo pgrx install --package pg_search --pg-config ~/.pgrx/18.1/pgrx-install/bin/pg_config
cargo pgrx start --package pg_search

cargo test --package tests
```

## Running Tests with a Self-Hosted PostgreSQL

If you are using a self-hosted PostgreSQL installation, install the `pg_search` extension on your system's PostgreSQL instead of pgrx’s.

```shell
#! /bin/sh

set -x
export DATABASE_URL=postgresql://localhost:28818/pg_search
export RUST_BACKTRACE=1
cargo pgrx stop --package pg_search
cargo pgrx install --package pg_search --pg-config /opt/homebrew/opt/postgresql@18/bin/pg_config
cargo pgrx start --package pg_search

cargo test --package tests
```

To run a single test, you can use the following command(replace `<testname>` with the test file name without the `.rs` extension):

```shell
cargo test --package tests --test <testname>
```

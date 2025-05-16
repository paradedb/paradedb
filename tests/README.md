# Test suite for `pg_search`

This is the test suite for the `pg_search` extension.

## Running Tests with pgrx-managed PostgreSQL

If you are using pgrx’s bundled PostgreSQL, follow these steps from the root of the repository:

```shell
#! /bin/sh

set -x
export DATABASE_URL=postgresql://localhost:28817/pg_search
export RUST_BACKTRACE=1
cargo pgrx stop --package pg_search
cargo pgrx install --package pg_search --features=icu --pg-config ~/.pgrx/17.0/pgrx-install/bin/pg_config
cargo pgrx start --package pg_search

cargo test --package tests --features=icu
```

## Running Tests with a Self-Hosted PostgreSQL

If you are using a self-hosted PostgreSQL installation, install the `pg_search` extension on your system's PostgreSQL instead of pgrx’s.

```shell
#! /bin/sh

set -x
export DATABASE_URL=postgresql://localhost:28817/pg_search
export RUST_BACKTRACE=1
cargo pgrx stop --package pg_search
cargo pgrx install --package pg_search --features=icu --pg-config /opt/homebrew/opt/postgresql@17/bin/pg_config
cargo pgrx start --package pg_search

cargo test --package tests --features=icu
```

To run a single test, you can use the following command(replace `<testname>` with the test file name without the `.rs` extension):

```shell
cargo test --package tests --features=icu --test <testname>
```

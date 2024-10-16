# Test suite for `pg_search`

This is the test suite for the `pg_search` extension.

An example of doing all that's necessary to run the tests is, from the root of the repo is:

```shell
#! /bin/sh

set -x
export DATABASE_URL=postgresql://localhost:28816/pg_search
export RUST_BACKTRACE=1 
cargo pgrx stop --package pg_search
cargo pgrx install --package pg_search --features=icu --pg-config ~/.pgrx/16.4/pgrx-install/bin/pg_config

cargo test --package pg_search-tests --features=icu
```


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

## SQL Regression Testing Framework

We have a SQL regression testing framework that enables writing SQL tests and automatically verifying their output against expected results.

### Adding a New SQL Test

1. Create a new SQL file in the `tests/sql` directory:

   ```sql
   -- tests/sql/my_feature.sql

   -- Test for feature X
   DROP TABLE IF EXISTS test_table;
   CREATE TABLE test_table (id INT, value TEXT);
   INSERT INTO test_table VALUES (1, 'test');

   -- This is the SQL we want to test
   SELECT * FROM test_table;

   DROP TABLE test_table;
   ```

2. Generate the test function by running:

   ```bash
   cargo run --bin generate_regression_tests
   ```

   This will automatically scan the `tests/sql` directory and add a test function for your new SQL file to `tests/tests/regression_tests.rs`.

3. Run the test with the `REGENERATE_EXPECTED` flag the first time:

   ```bash
   REGENERATE_EXPECTED=1 ./scripts/pg_search_test.sh --test regression_tests test_my_feature
   ```

   This will generate an expected output file in `tests/sql/expected/my_feature.out`.

4. Verify the expected output is correct.

5. Commit both the SQL file and the generated output file.

### Running SQL Regression Tests

- **Run all regression tests:**

  ```bash
  ./scripts/pg_search_test.sh --test regression_tests
  ```

- **Run a specific test:**

  ```bash
  ./scripts/pg_search_test.sh --test regression_tests test_my_feature
  ```

- **Regenerate expected output for a test:**

  ```bash
  REGENERATE_EXPECTED=1 ./scripts/pg_search_test.sh --test regression_tests test_my_feature
  ```

- **Regenerate all expected outputs:**

  ```bash
  REGENERATE_EXPECTED=1 ./scripts/pg_search_test.sh --test regression_tests
  ```

### How the Framework Works

When you run a test:

1. The SQL file is executed via psql
2. The output is normalized to remove variable content
3. The normalized output is compared with the expected output
4. If they match, the test passes; if not, a detailed diff is shown

The system handles normalization of PostgreSQL output, so you don't need to worry about session-specific messages or timing information causing test failures.

### Why Not Use PostgreSQL's pg_regress?

While PostgreSQL's `pg_regress` offers similar functionality, our custom framework:

1. **Integrates with Cargo** - Runs within the Rust test infrastructure
2. **Simplifies workflow** - Add tests without PostgreSQL build knowledge
3. **Provides tailored normalization** - Handles ParadeDB-specific output patterns
4. **Works with pgrx** - Seamlessly integrates with our development environment

This approach offers a more straightforward experience for ParadeDB contributors while maintaining the core functionality of regression testing similat to `pg_regress`.

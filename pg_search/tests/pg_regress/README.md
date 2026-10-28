# PostgreSQL Regression Tests for ParadeDB

This directory contains the **pg regress tests** for ParadeDB's `pg_search` extension. These run with `cargo pgrx regress` and do not need the extension to be manually installed: it is handled automatically.

For a complete overview of ParadeDB's testing infrastructure (including unit tests, integration tests, and client property tests), please see the [Testing section in `CONTRIBUTING.md`](../../../CONTRIBUTING.md#testing).

## Directory Structure

- `sql/`: Contains SQL script files that are executed during testing
- `expected/`: Contains expected output files for each test
- `results/` (ignored under git): Stores actual output files generated during test runs
- `common/`: Contains common setup and cleanup scripts used by multiple tests

## Adding New Tests

### Step 1: Create the SQL Test

1. Name the file after the feature, behavior, or issue it covers, following nearby tests.
2. If the test belongs to a group with shared setup, include the corresponding script from `common/`, for example:

   ```sql
   \i common/PREFIX_setup.sql
   ```

3. Add a short comment describing the behavior under test:

   ```sql
   -- Tests my new feature
   ```

4. Make output deterministic, including an `ORDER BY` wherever row order matters.
5. If the test uses shared setup, include the matching cleanup script at the end:

   ```sql
   \i common/PREFIX_cleanup.sql
   ```

### Step 2: Generate the Expected Output

Bootstrap the test and promote its output to `expected/`:

```bash
cd pg_search
cargo pgrx regress --add PREFIX_your_test
```

Pass a PostgreSQL version before `--add` to use a version other than the default, for example `cargo pgrx regress pg17 --add PREFIX_your_test`.

### Step 3: Verify the Expected Output

1. Check the generated output file in `expected/PREFIX_your_test.out`
2. Verify that results and errors demonstrate the intended behavior.
3. Verify that any `EXPLAIN` plans use stable options and the expected execution path.

## Running Tests

### Run All Tests

```bash
cd pg_search
cargo pgrx regress
```

### Run Specific Tests

```bash
cd pg_search
cargo pgrx regress pg18 PREFIX_your_test
```

Use `--auto` only when you intentionally want to replace the expected output of a failing test with its current output.

## Common Pitfalls

1. **Non-deterministic Results**: Use fixed dates and ORDER BY clauses to ensure consistent results
2. **Missing Data**: Verify that all test queries return at least one row of data
3. **Timing Variations**: Use `COSTS OFF, TIMING OFF` in EXPLAIN to avoid timing-dependent output

## Contributing New Tests

When contributing new tests:

1. Follow the naming convention of existing tests
2. Use the appropriate common setup/cleanup scripts
3. Check that your test produces deterministic results
4. Include metadata comments at the top of the file explaining what's being tested

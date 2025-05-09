# PostgreSQL Regression Tests for ParadeDB

This directory contains regression tests for ParadeDB's `pg_search` extension.

## Directory Structure

- `sql/`: Contains SQL script files that are executed during testing
- `expected/`: Contains expected output files for each test
- `results/` (ignored under git): Stores actual output files generated during test runs
- `common/`: Contains common setup and cleanup scripts used by multiple tests

## Test Organization

Tests are organized into logical groups with a common prefix:

- `mixedff_`: Mixed Fast Fields tests
  - `mixedff_basic_*`: Basic mixed fast fields functionality tests
  - `mixedff_edgecases_*`: Edge cases and boundary condition tests
  - `mixedff_queries_*`: Tests for complex query features
  - `mixedff_advanced_*`: Tests for advanced features and optimizations

Each group uses its own setup and cleanup scripts from the `common/` directory.

## Adding New Tests

### Step 1: Create SQL Test File

1. Name your test file using the appropriate prefix and sequence number (e.g., `PREFIX_your_test.sql`)
2. Start with the common setup script for your test group, e.g.,:

   ```sql
   \i common/PREFIX_setup.sql
   ```

3. Add a descriptive header and echo statement:

   ```sql
   -- Tests my new feature
   ```

4. Use deterministic data and ORDER BY clauses to ensure consistent results
5. End with the appropriate cleanup script, e.g.,:

   ```sql
   \i common/PREFIX_cleanup.sql
   ```

### Step 2: Generate Expected Output

Run your test to generate the expected output file:

```bash
cd pg_search
cargo pgrx regress --auto
```

### Step 3: Verify the Expected Output

1. Check the generated output file in `expected/PREFIX_your_test.out`
2. Ensure all queries return at least one row of data
3. Verify that any EXPLAIN plans look reasonable

## Running Tests

### Run All Tests

```bash
cd pg_search
cargo pgrx regress
```

### Run Specific Tests

```bash
cd pg_search
cargo pgrx regress pg17 PREFIX_your_test
```

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

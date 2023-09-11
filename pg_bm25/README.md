<h1 align="center">
  <img src="../docs/logo/pg_bm25.svg" alt="pg_bm25" width="800px"></a>
<br>

[![Testing](https://github.com/paradedb/paradedb/actions/workflows/test-pg_bm25.yml/badge.svg)](https://github.com/paradedb/paradedb/actions/workflows/test-pg_bm25.yml)
[![codecov](https://codecov.io/gh/getretake/paradedb/graph/badge.svg?token=PHV8CAMHNQ)](https://codecov.io/gh/getretake/paradedb)

## Overview

`pg_bm25` is a PostgreSQL extension that enables full-text search
using the Okapi BM25 algorithm, the state-of-the-art ranking function
for full text search. It is built in Rust using `pgrx` and supported on PostgreSQL 11+.

## Development

### Prerequisites

Before developing the extension, ensure that you have Rust installed
(version >1.70), ideally via `rustup`.

### Running the Extension

1. Install and initialize pgrx:

```bash
cargo install --locked cargo-pgrx
cargo pgrx init
```

2. Start pgrx:

```bash
cargo pgrx run
```

This will launch an interactive connection to Postgres. Inside Postgres, create
the extension by running:

```sql
CREATE EXTENSION pg_bm25;
```

You can verify that the extension functions are installed by using:

```sql
\df
```

Now, you have access to all the extension functions.

### Indexing a Table

By default, the `pg_bm25` extension creates a table called `paradedb.mock_items` that you can
use for quick experimentation.

To index a table, use the following SQL command:

```sql
CREATE TABLE mock_items AS SELECT * FROM paradedb.mock_items;
CREATE INDEX idx_mock_items ON mock_items USING bm25 ((mock_items.*));
```

Once the indexing is complete, you can run various search functions on it.

### Performing Searches

Execute a search query on your indexed table:

```sql
SELECT *
FROM mock_items
WHERE mock_items @@@ 'description:keyboard OR category:electronics OR rating>2';
```

### Modifying the Extension

If you make changes to the extension code, follow these steps to update it:

1. Recompile the extension:

```bash
cargo pgrx run
```

2. Recreate the extension to load the latest changes:

```sql
DROP EXTENSION pg_bm25;
CREATE EXTENSION pg_bm25;
```

## Testing

To run the unit test suite, use the following command:

```bash
cargo pgrx test
```

This will run all unit tests defined in `/src`. To add a new unit test, simply add
tests inline in the relevant files, using the `#[cfg(test)]` attribute.

To run the integration test suite, use the following command:

```bash
./test/runtests.sh
```

This will create a temporary database, initialize it with the SQL commands defined
in `fixtures.sql`, and run the tests in `/test/sql` against it. To add a new test,
simply add a new `.sql` file to `/test/sql` and a corresponding `.out` file to
`/test/expected` for the expected output, and it will automatically get picked up
by the test suite.

## Packaging

The extension gets packaged into our Docker image as part of the build process.
If you want to package the extension locally, you can do so by running the
following command:

```bash
cargo pgrx package [--test]
```

<h1 align="center">
  <img src="../docs/logo/pg_search.svg" alt="pg_search" width="400px"></a>
<br>
</h1>

[![Testing](https://github.com/paradedb/paradedb/actions/workflows/test-pg_search.yml/badge.svg)](https://github.com/paradedb/paradedb/actions/workflows/test-pg_search.yml)
[![codecov](https://codecov.io/gh/getretake/paradedb/graph/badge.svg?token=PHV8CAMHNQ)](https://codecov.io/gh/getretake/paradedb)

`pg_search` enables hybrid search in Postgres. Hybrid search is a search technique that combines BM25-based full text search with vector-based similarity search. This extension is built in Rust using `pgrx` and supported on PostgreSQL 11+.

## Development

### Prerequisites

Before developing the extension, ensure that you have Rust installed
(version >1.70), ideally via `rustup` (we've observed issues with installing Rust
via Homebrew on macOS).

Then, install and initialize pgrx:

```bash
cargo install --locked cargo-pgrx
cargo pgrx init
```

### Running the Extension

`pg_search` is built on top of two extensions: `pg_bm25` and `pgvector`. To install these two
extensions, run the configure script (this must be done _after_ initializing pgrx):

```bash
./configure.sh
```

Note that you need to run this script every time you'd like to update these dependencies.

Then, start pgrx``:

```bash
cargo pgrx run
```

This will launch an interactive connection to Postgres. Inside Postgres, create
the extension by running:

```sql
CREATE EXTENSION pg_search CASCADE;
```

Now, you have access to all the extension functions. `pg_search` comes with a table
called `paradedb.mock_items`, which contains some pre-populated data:

```sql
CREATE TABLE mock_items AS SELECT * FROM paradedb.mock_items;
SELECT * FROM mock_items LIMIT 5;
```

### Indexing a Table

To perform a hybrid search, you'll first need to create a BM25 and HNSW index on your table.

To create a BM25 index:

```sql
CREATE INDEX idx_mock_items ON mock_items USING bm25 ((mock_items.*));
```

To create a HNSW index:

```sql
CREATE INDEX ON mock_items USING hnsw (embedding vector_l2_ops);
```

### Performing Searches

The following query executes a hybrid search on `mock_items`:

```sql
SELECT
    description,
    category,
    rating,
    paradedb.weighted_mean(
        paradedb.score_bm25(ctid, 'idx_mock_items', 'description:keyboard'),
        '[1,2,3]' <-> embedding,
        ARRAY[0.8, 0.2]
    ) AS score_hybrid
FROM mock_items
ORDER BY score_hybrid DESC
LIMIT 3;
```

See the [documentation](https://docs.paradedb.com/search/hybrid) for more details.

### Modifying the Extension

If you make changes to the extension code, follow these steps to update it:

1. Recompile the extension:

```bash
cargo pgrx run
```

2. Recreate the extension to load the latest changes:

```sql
DROP EXTENSION pg_search;
CREATE EXTENSION pg_search;
```

## Testing

To run the unit test suite, use the following command:

```bash
cargo pgrx test
```

This will run all unit tests defined in `/src`. To add a new unit test, simply add
tests inline in the relevant files, using the `#[cfg(test)]` attribute.

To run the integration test suite, first install the extension on your system with:

```bash
cargo pgrx install
```

You'll need to run this command every time you make changes to the extension that
you want reflected in the integration test suite. Then, run the tests with:

```bash
./test/runtests.sh
```

This will create a temporary database, initialize it with the SQL commands defined
in `fixtures.sql`, and run the tests in `/test/sql` against it. To add a new test,
simply add a new `.sql` file to `/test/sql` and a corresponding `.out` file to
`/test/expected` for the expected output, and it will automatically get picked up
by the test suite.

## Packaging

The extension gets packaged into our Docker image(s) as part of the build process.
If you want to package the extension locally, you can do so by running the
following command:

```bash
cargo pgrx package [--test]
```

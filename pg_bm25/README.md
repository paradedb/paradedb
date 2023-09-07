# pg_bm25

[![codecov](https://codecov.io/gh/getretake/paradedb/graph/badge.svg?token=PHV8CAMHNQ)](https://codecov.io/gh/getretake/paradedb)

The pg_bm25 extension is a PostgreSQL extension that enables full-text search
using the Okapi BM25 algorithm, which is the state-of-the-art ranking function
for full-text search information retrieval. It is built in Rust using `pgrx` and
supported on PostgreSQL 11+.

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

To index a table, use the following SQL command:

```sql
SELECT index_bm25('table_name', 'index_name', '{col1, col2}');
```

Once the indexing is complete, you can run various search functions on it.

### Performing Searches

Execute a search query on your indexed table:

```sql
SELECT search_bm25('query', 'table_name', 'index_name', 10, 0);
```

Here, `10` represents the maximum number of results to return, and `0` is the
offset. You can specify columns in your search query by using the following
format: `column_name:query`.

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

To run the test suite, use the following command:

```bash
cargo pgrx test
```

This will run all unit tests defined in `/src` and all integration tests defined
in `/test/sql` and `/test/expected`. To add a new integration test, simply add a
new `.sql` file to `/test/sql` and a corresponding `.out` file to
`/test/expected` for the expected output.

## Packaging

The extension gets packaged into our Docker image as part of the build process.
If you want to package the extension locally, you can do so by running the
following command:

```bash
cargo pgrx package [--test]
```

<h1 align="center">
  <img src="../docs/logo/pg_bm25.svg" alt="pg_bm25" width="500px"></a>
<br>
</h1>

[![Testing](https://github.com/paradedb/paradedb/actions/workflows/test-pg_bm25.yml/badge.svg)](https://github.com/paradedb/paradedb/actions/workflows/test-pg_bm25.yml)
[![codecov](https://codecov.io/gh/getretake/paradedb/graph/badge.svg?token=PHV8CAMHNQ)](https://codecov.io/gh/getretake/paradedb)

## Overview

`pg_bm25` is a PostgreSQL extension that enables full text search over SQL tables
using the BM25 algorithm, the state-of-the-art ranking function
for full text search. It is built in Rust using `pgrx` and supported on PostgreSQL
11+.

## Development

### Prerequisites

Before developing the extension, ensure that you have Rust installed
(version >1.70), ideally via `rustup` (we've observed issues with installing Rust
via Homebrew on macOS).

Then, install and initialize pgrx:

```bash
cargo install cargo-pgrx --version 0.9.8
cargo pgrx init
```

### Running the Extension

First, start pgrx:

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

By default, the `pg_bm25` extension creates a table called `paradedb.mock_items`
that you can use for quick experimentation.

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
WHERE mock_items @@@ 'description:keyboard OR category:electronics';
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

To run the integration test suite, simply run:

```bash
./test/runtests.sh
```

This will create a temporary database, initialize it with the SQL commands defined
in `fixtures.sql`, and run the tests in `/test/sql` against it. To add a new test,
simply add a new `.sql` file to `/test/sql` and a corresponding `.out` file to
`/test/expected` for the expected output, and it will automatically get picked up
by the test suite.

## Installation

If you'd like to install the extension on a local machine, for instance if you
are self-hosting Postgres and would like to use the extension within your existing
Postgres database, follow these steps:

1. Install Rust and cargo-pgrx:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install cargo-pgrx --version 0.9.8
```

2. Then, run:

```bash
# Clone the repo (optionally pick a specific version)
git clone https://github.com/paradedb/paradedb.git --tag <VERSION>

# Install pg_bm25
cd pg_bm25/
cargo pgrx init --pg<YOUR-POSTGRES-MAJOR_VERSION>=`which pg_config`
cargo pgrx install
```

You can then create the extension in your database by running:

```sql
CREATE EXTENSION pg_bm25;
```

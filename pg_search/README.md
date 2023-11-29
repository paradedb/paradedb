<h1 align="center">
  <img src="../docs/logo/pg_search.svg" alt="pg_search" width="600px"></a>
<br>
</h1>

[![Test pg_search](https://github.com/paradedb/paradedb/actions/workflows/test-pg_search.yml/badge.svg)](https://github.com/paradedb/paradedb/actions/workflows/test-pg_search.yml)

## Overview

`pg_search` is a PostgreSQL extension that enables hybrid search in Postgres. Hybrid search is a search technique that combines BM25-based full text search with vector-based similarity search. It is built on top of `pg_bm25`, the leading full text search extension for Postgres, and `pgvector`, the leading vector similarity search extension for Postgres, using `pgrx`.

`pg_search` is supported on all versions supported by the PostgreSQL Global Development Group, which includes PostgreSQL 12+.

## Installation

### From ParadeDB

The easiest way to use the extension is to run the ParadeDB Dockerfile:

```bash
docker run \
  -e POSTGRES_USER=<user> \
  -e POSTGRES_PASSWORD=<password> \
  -e POSTGRES_DB=<dbname> \
  -p 5432:5432 \
  -d \
  paradedb/paradedb:latest
```

This will spin up a Postgres instance with `pg_search` and its dependencies preinstalled.

### From Self-Hosted PostgreSQL

If you are self-hosting Postgres and would like to use the extension within your existing Postgres, follow these steps:

#### Debian/Ubuntu

We provide pre-built binaries for Debian-based Linux for PostgreSQL 15 (more versions coming soon). You can download the latest version for your architecture from the [releases page](https://github.com/paradedb/paradedb/releases). Note that to install `pg_search`, you need to also download and install `pg_bm25`, and `pgvector`. You can install `pgvector` via:

```bash
sudo apt-get install postgres-15-pgvector
```

ParadeDB collects anonymous telemetry to help us understand how many people are using the project. You can opt-out of telemetry by setting `export TELEMETRY=false` (or unsetting the variable) in your shell or in your `~/.bashrc` file before running the extension.

#### macOS and Windows

We don't suggest running production workloads on macOS or Windows. As a result, we don't provide prebuilt binaries for these platforms. If you are running Postgres on macOS or Windows and want to install `pg_search`, please follow the [development](#development) instructions, but do `cargo pgrx install --release` instead of `cargo pgrx run`. This will build the extension from source and install it in your Postgres instance.

You can then create the extension in your database by running:

```sql
CREATE EXTENSION pg_search CASCADE;
```

Note: If you are using a managed Postgres service like Amazon RDS, you will not be able to install `pg_bm25` until the Postgres service explicitly supports it.

## Usage

### Indexing

`pg_search` comes with a helper function that creates a test table that you can use for quick experimentation.

```sql
CALL paradedb.create_search_test_table();
CREATE TABLE mock_items AS SELECT * FROM paradedb.search_test_table;
```

To perform a hybrid search, you'll first need to create a BM25 and a HNSW index on your table. To index a table, use the following SQL command:

```sql
-- BM25 index
CREATE INDEX idx_mock_items
ON mock_items
USING bm25 ((mock_items.*))
WITH (text_fields='{"description": {}, "category": {}}');

-- HNSW index
CREATE INDEX ON mock_items USING hnsw (embedding vector_l2_ops);
```

Once the indexing is complete, you can run various search functions on it.

### Basic Search

Execute a search query on your indexed table:

```sql
SELECT
    description,
    category,
    rating,
    paradedb.weighted_mean(
        paradedb.minmax_bm25(ctid, 'idx_mock_items', 'description:keyboard'),
        1 - paradedb.minmax_norm(
          '[1,2,3]' <-> embedding,
          MIN('[1,2,3]' <-> embedding) OVER (),
          MAX('[1,2,3]' <-> embedding) OVER ()
        ),
        ARRAY[0.8,0.2]
    ) as score_hybrid
FROM mock_items
ORDER BY score_hybrid DESC;
```

Please refer to the [documentation](https://docs.paradedb.com/search/hybrid) for a more thorough overview of `pg_search`'s query support.

## Development

### Prerequisites

Before developing the extension, ensure that you have Rust installed
(version >1.70), ideally via `rustup` (we've observed issues with installing Rust via Homebrew on macOS).

If you are on macOS and using Postgres.app, you'll first need to add the `pg_config` binary to your path:

```bash
export PATH="$PATH:/Applications/Postgres.app/Contents/Versions/latest/bin"
```

Then, install and initialize pgrx:

```bash
# Note: Replace --pg15 with your version of Postgres, if different (i.e. --pg16, --pg14, etc.)
cargo install --locked cargo-pgrx --version 0.11.1
cargo pgrx init --pg15=`which pg_config`
```

### Running the Extension

`pg_search` is built on top of two extensions: `pg_bm25` and `pgvector`. To install these two extensions, run the configure script. This must be done _after_ initializing pgrx:

```bash
./configure.sh
```

Note that you need to run this script every time you'd like to update these dependencies.

Then, start pgrx:

```bash
cargo pgrx run
```

This will launch an interactive connection to Postgres. Inside Postgres, create the extension by running:

```sql
CREATE EXTENSION pg_search CASCADE;
```

Now, you have access to all the extension functions.

### Modifying the Extension

If you make changes to the extension code, follow these steps to update it:

1. Recompile the extension:

```bash
cargo pgrx run
```

2. Recreate the extension to load the latest changes:

```sql
DROP EXTENSION pg_search CASCADE;
CREATE EXTENSION pg_search CASCADE;
```

## Testing

To run the unit test suite, use the following command:

```bash
cargo pgrx test
```

This will run all unit tests defined in `/src`. To add a new unit test, simply add tests inline in the relevant files, using the `#[cfg(test)]` attribute.

To run the integration test suite, simply run:

```bash
./test/runtests.sh -p threaded
```

This will create a temporary database, initialize it with the SQL commands defined in `fixtures.sql`, and run the tests in `/test/sql` against it. To add a new test, simply add a new `.sql` file to `/test/sql` and a corresponding `.out` file to `/test/expected` for the expected output, and it will automatically get picked up by the test suite.

Note: the bash script takes arguments and allows you to run tests either sequentially or in parallel. For more info run `./test/runtests.sh -h`

## License

`pg_search` is licensed under the [GNU Affero General Public License v3.0](../LICENSE).

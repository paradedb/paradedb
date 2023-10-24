<h1 align="center">
  <img src="../docs/logo/pg_search.svg" alt="pg_search" width="600px"></a>
<br>
</h1>

[![Testing](https://github.com/paradedb/paradedb/actions/workflows/test-pg_search.yml/badge.svg)](https://github.com/paradedb/paradedb/actions/workflows/test-pg_search.yml)

## Overview

`pg_search` is a PostgreSQL extension that enables hybrid search in Postgres. Hybrid search is a search technique that combines BM25-based full text search with vector-based similarity search. It is built on top of `pg_bm25`, the leading full text search extension for Postgres, and `pgvector`, the leading vector similarity search extension for Postgres, using `pgrx`.

`pg_search` is supported on PostgreSQL 11+.

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

We provide prebuilt binaries for Debian-based Linux, currently only for PostgreSQL 15 (more versions coming soon). To install `pg_search`, follow these steps:

```bash
# Install pg_bm25
wget "$(curl -s "https://api.github.com/repos/paradedb/paradedb/releases/latest" | grep "browser_download_url.*pg_bm25.*.deb" | cut -d : -f 2,3 | tr -d \")" -O pg_bm25.deb && sudo apt-get install pg_bm25.deb

# Install pgvector
sudo apt-get install postgresql-15-pgvector

# Install pg_search
wget "$(curl -s "https://api.github.com/repos/paradedb/paradedb/releases/latest" | grep "browser_download_url.*pg_search.*.deb" | cut -d : -f 2,3 | tr -d \")" -O pg_bm25.deb && sudo apt-get install pg_search.deb
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

By default, the `pg_search` extension creates a table called `paradedb.mock_items` that you can use for quick experimentation.

To perform a hybrid search, you'll first need to create a BM25 and a HNSW index on your table. To index a table, use the following SQL command:

```sql
CREATE TABLE mock_items AS SELECT * FROM paradedb.mock_items;

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
        paradedb.minmax_bm25(ctid, 'idx_mock_items', 'keyboard'),
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

Then, install and initialize pgrx:

```bash
cargo install cargo-pgrx --version 0.11.0
cargo pgrx init
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

The `pg_search` is licensed under the [GNU Affero General Public License v3.0](../LICENSE).

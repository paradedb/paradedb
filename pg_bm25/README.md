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

### Roadmap

- [x] BM25 scoring
- [x] Highlighting
- [x] Boosted queries
- [x] Filtering
- [] Faceting/aggregations
- [] Autocomplete
- [] Fuzzy queries
- [] Custom tokenizers

## Usage

### Running the Extension

The easiest way to test the extension is to run the ParadeDB Dockerfile:

```bash
docker run \
  -e POSTGRES_USER=<user> \
  -e POSTGRES_PASSWORD=<password> \
  -e POSTGRES_DB=<dbname> \
  -p 5432:5432 \
  paradedb/paradedb:latest
```

This will spin up a Postgres instance with `pg_bm25` preinstalled.

### Indexing

By default, the `pg_bm25` extension creates a table called `paradedb.mock_items`
that you can use for quick experimentation.

To index a table, use the following SQL command:

```sql
CREATE TABLE mock_items AS SELECT * FROM paradedb.mock_items;
CREATE INDEX idx_mock_items ON mock_items USING bm25 ((mock_items.*));
```

Once the indexing is complete, you can run various search functions on it.

### Basic Search

Execute a search query on your indexed table:

```sql
SELECT *
FROM mock_items
WHERE mock_items @@@ 'description:keyboard OR category:electronics';
```

This will return:

```csv
         description         | rating |  category
-----------------------------+--------+-------------
 Plastic Keyboard            |      4 | Electronics
 Ergonomic metal keyboard    |      4 | Electronics
 Innovative wireless earbuds |      5 | Electronics
 Fast charging power bank    |      4 | Electronics
 Bluetooth-enabled speaker   |      3 | Electronics
(5 rows)
```

Scoring and highlighting are supported:

```sql
SELECT *, paradedb.rank_bm25(ctid), paradedb.highlight_bm25(ctid, 'description')
FROM mock_items
WHERE mock_items @@@ 'description:keyboard OR category:electronics';
```

This will return:

```csv
 id |         description         | rating |  category   | rating2 | rank_bm25 |         highlight_bm25
----+-----------------------------+--------+-------------+---------+-----------+---------------------------------
  1 | Ergonomic metal keyboard    |      4 | Electronics |       5 | 4.9403534 | Ergonomic metal <b>keyboard</b>
  2 | Very plasticy keyboard      |      4 | Electronics |       4 | 4.9403534 | Very plasticy <b>keyboard</b>
 12 | Innovative wireless earbuds |      5 | Electronics |       1 | 2.1096356 |
 22 | Fast charging power bank    |      4 | Electronics |       1 | 2.1096356 |
 32 | Bluetooth-enabled speaker   |      3 | Electronics |       1 | 2.1096356 |
(5 rows)
```

Scores can be tuned via boosted queries:

```sql
SELECT *
FROM mock_items
WHERE mock_items @@@ 'description:keyboard^2 OR category:electronics';
```

Please refer to the [documentation](https://docs.paradedb.com/search/bm25) for a more thorough overview
of `pg_bm25`'s query support.

## Benchmarks

On a table with 1 million rows, `pg_bm25` indexes 50 seconds faster than `tsvector` and searches + ranks
results 20x faster. Indexing and search times are nearly identical to those of a dedicated ElasticSearch
instance.

<img src="../docs/images/bm25_index_benchmark.png" alt="" width="500px">

<img src="../docs/images/bm25_search_benchmark.png" alt="" width="500px">

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

### Testing

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

If you are self-hosting Postgres and would like to use the extension within your existing
Postgres, follow these steps:

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

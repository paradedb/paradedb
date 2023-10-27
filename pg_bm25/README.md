<h1 align="center">
  <img src="../docs/logo/pg_bm25.svg" alt="pg_bm25" width="500px"></a>
<br>
</h1>

[![Testing](https://github.com/paradedb/paradedb/actions/workflows/test-pg_bm25.yml/badge.svg)](https://github.com/paradedb/paradedb/actions/workflows/test-pg_bm25.yml)

## Overview

`pg_bm25` is a PostgreSQL extension that enables full text search over SQL tables using the BM25 algorithm, the state-of-the-art ranking function
for full text search. It is built on top of Tantivy, the Rust-based alternative to Apache Lucene, using `pgrx`.

`pg_bm25` is supported on PostgreSQL 11+.

Check out the `pg_bm25` benchmarks [here](../benchmarks/README.md).

### Roadmap

- [x] BM25 scoring
- [x] Highlighting
- [x] Boosted queries
- [x] Filtering
- [x] Bucket and metrics aggregations
- [x] Autocomplete
- [x] Fuzzy search
- [x] Custom tokenizers
- [x] JSON field search
- [ ] Datetime aggregations
- [ ] Facet fields

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

This will spin up a Postgres instance with `pg_bm25` preinstalled.

### From Self-Hosted PostgreSQL

If you are self-hosting Postgres and would like to use the extension within your existing Postgres, follow these steps:

#### Debian/Ubuntu

We provide prebuilt binaries for Debian-based Linux, currently only for PostgreSQL 15 (more versions coming soon). To install `pg_bm25`, follow these steps:

```bash
# Download the .deb file
wget "$(curl -s "https://api.github.com/repos/paradedb/paradedb/releases/latest" | grep "browser_download_url.*pg_bm25.*.deb" | cut -d : -f 2,3 | tr -d \")" -O pg_bm25.deb

# Install the .deb file
sudo apt-get install pg_bm25.deb
```

ParadeDB collects anonymous telemetry to help us understand how many people are using the project. You can opt-out of telemetry by setting `export TELEMETRY=false` (or unsetting the variable) in your shell or in your `~/.bashrc` file before running the extension.

#### macOS and Windows

We don't suggest running production workloads on macOS or Windows. As a result, we don't provide prebuilt binaries for these platforms. If you are running Postgres on macOS or Windows and want to install `pg_bm25`, please follow the [development](#development) instructions, but do `cargo pgrx install --release` instead of `cargo pgrx run`. This will build the extension from source and install it in your Postgres instance.

You can then create the extension in your database by running:

```sql
CREATE EXTENSION pg_bm25;
```

Note: If you are using a managed Postgres service like Amazon RDS, you will not be able to install `pg_bm25` until the Postgres service explicitly supports it.

## Usage

### Indexing

By default, the `pg_bm25` extension creates a table called `paradedb.mock_items` that you can use for quick experimentation.

To index a table, use the following SQL command:

```sql
CREATE TABLE mock_items AS SELECT * FROM paradedb.mock_items;

CREATE INDEX idx_mock_items
ON mock_items
USING bm25 ((mock_items.*))
WITH (text_fields='{"description": {}, "category": {}}');
```

Once the indexing is complete, you can run various search functions on it.

### Basic Search

Execute a search query on your indexed table:

```sql
SELECT description, rating, category
FROM mock_items
WHERE mock_items @@@ 'description:keyboard OR category:electronics'
LIMIT 5;
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
SELECT description, rating, category, paradedb.rank_bm25(ctid), paradedb.highlight_bm25(ctid, 'idx_mock_items', 'description')
FROM mock_items
WHERE mock_items @@@ 'description:keyboard OR category:electronics'
LIMIT 5;
```

This will return:

```csv
 id |         description         | rating |  category   | rank_bm25 |         highlight_bm25
----+-----------------------------+--------+-------------+-----------+---------------------------------
  1 | Ergonomic metal keyboard    |      4 | Electronics | 4.9403534 | Ergonomic metal <b>keyboard</b>
  2 | Very plasticy keyboard      |      4 | Electronics | 4.9403534 | Very plasticy <b>keyboard</b>
 12 | Innovative wireless earbuds |      5 | Electronics | 2.1096356 |
 22 | Fast charging power bank    |      4 | Electronics | 2.1096356 |
 32 | Bluetooth-enabled speaker   |      3 | Electronics | 2.1096356 |
(5 rows)
```

Scores can be tuned via boosted queries:

```sql
SELECT description, rating, category
FROM mock_items
WHERE mock_items @@@ 'description:keyboard^2 OR category:electronics';
```

New data that arrives or rows that are changed are automatically reindexed and searchable. For instance, let's create and search for a new row in our table:

```sql
INSERT INTO mock_items (description, rating, category) VALUES ('New keyboard', 5, 'Electronics');

SELECT description, rating, category
FROM mock_items
WHERE mock_items @@@ 'description:keyboard OR category:electronics'
LIMIT 5;
```

This will return:

```csv
         description         | rating |  category
-----------------------------+--------+-------------
 New keyboard                |      5 | Electronics
 Plastic Keyboard            |      4 | Electronics
 Ergonomic metal keyboard    |      4 | Electronics
 Innovative wireless earbuds |      5 | Electronics
 Fast charging power bank    |      4 | Electronics
(5 rows)
```

Please refer to the [documentation](https://docs.paradedb.com/search/bm25) for a more thorough overview of `pg_bm25`'s query support.

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
cargo install --locked cargo-pgrx --version 0.11.0
cargo pgrx init --pg15=`which pg_config`
```

### Running the Extension

First, start pgrx:

```bash
cargo pgrx run
```

This will launch an interactive connection to Postgres. Inside Postgres, create the extension by running:

```sql
CREATE EXTENSION pg_bm25;
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

This will run all unit tests defined in `/src`. To add a new unit test, simply add tests inline in the relevant files, using the `#[cfg(test)]` attribute.

To run the integration test suite, simply run:

```bash
./test/runtests.sh -p threaded
```

This will create a temporary database, initialize it with the SQL commands defined in `fixtures.sql`, and run the tests in `/test/sql` against it. To add a new test, simply add a new `.sql` file to `/test/sql` and a corresponding `.out` file to `/test/expected` for the expected output, and it will automatically get picked up by the test suite.

Note: the bash script takes arguments and allows you to run tests either sequentially or in parallel. For more info run `./test/runtests.sh -h`

## License

The `pg_bm25` is licensed under the [GNU Affero General Public License v3.0](../LICENSE).

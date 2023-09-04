<p align="center">
  <img src="media/assets/paradedb.svg" alt="ParadeDB" width="500px"></a>
</p>

<p align="center">
    <b>PostgreSQL for Search</b> <br />
</p>

<h3 align="center">
  <a href="https://paradedb.com">Website</a>
</h3>

[![codecov](https://codecov.io/gh/getretake/paradedb/graph/badge.svg?token=PHV8CAMHNQ)](https://codecov.io/gh/getretake/paradedb)

ParadeDB is an ElasticSearch alternative built on Postgres.

To get started, run our Docker Compose file:

```bash
git clone git@github.com:getretake/paradedb.git
cd paradedb/docker
docker compose up
```

By default, this will start the ParadeDB database at `http://localhost:5432`.

Note that ParadeDB is still under active development and is not yet ready to use in production. We're aiming to be
ready by the end of September 2023.

- [ ] Search
  - [x] Full-text search with BM25
  - [ ] Faceted search
  - [x] Similarity search
  - [ ] Hybrid search
  - [ ] Distributed search
  - [ ] Real-time search
  - [ ] Generative search
  - [ ] Multimodal search
- [ ] Cloud Database
  - [ ] Managed cloud
  - [ ] Self-serve cloud
- [ ] Web-based SQL Editor

## Development

### Prerequisites

Before developing the extension, ensure you have Rust installed (version >
1.70).

### Installation

1. Install and initialize pgrx:

```bash
cargo install --locked cargo-pgrx
cargo pgrx init
```

2. Start pgrx:

```bash
cargo pgrx run
```

This will launch an interactive connection to Postgres.

Inside Postgres, create the extension by running:

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

Here, `10` represents the maximum number of results to return, and `0` is the offset. You can specify specific columns
in your search query using the following format: `column_name:query`.

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

## Contributing

For more information on how to contribute, please see our
[Contributing Guide](CONTRIBUTING.md).

## License

ParadeDB is licensed under the [Elastic License 2.0](LICENSE). Our goal with
choosing ELv2 is to maintain an open-source spirit and be as permissive as
possible, while protecting against abuse. Our users can continue to use and
contribute to ParadeDB freely, and we can safely create a sustainable business
and continue to invest in our community, project and product.

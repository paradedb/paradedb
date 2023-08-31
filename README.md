<p align="center">
  <img src="assets/paradedb.svg" alt="Retake" width="500px"></a>
</p>

<p align="center">
    <b>PostgreSQL for Search</b> <br />
</p>

<h3 align="center">
  <a href="https://docs.getretake.com">Documentation</a> &bull;
  <a href="https://getretake.com">Website</a>
</h3>

<p align="center">
<a href="https://github.com/getretake/retake/stargazers/" target="_blank">
    <img src="https://img.shields.io/github/stars/getretake/retake?style=social&label=Star&maxAge=60" alt="Stars">
</a>
<a href="https://github.com/getretake/retake/releases" target="_blank">
    <img src="https://img.shields.io/github/v/release/getretake/retake?color=white" alt="Release">
</a>
<a href="https://github.com/getretake/retake/tree/main/LICENSE" target="_blank">
    <img src="https://img.shields.io/static/v1?label=license&message=Apache-2.0&color=white" alt="License">
</a>
</p>

ParadeDB is an ElasticSearch alternative built on Postgres.

To get started, run our Docker Compose file:

```bash
git clone git@github.com:getretake/paradedb.git
cd paradedb/docker
docker compose up
```

By default, this will start the ParadeDB database at `http://localhost:5432`.

## Usage

TODO

## Key Features

TODO

## How ParadeDB Works

TODO

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
SELECT search_bm25('query', 'table_name', 'index_name', k);
```

Note: You can specify specific columns in your search query using the following
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

## Contributing

For more information on how to contribute, please see our
[Contributing Guide](CONTRIBUTING.md).

## License

ParadeDB is licensed under the [Elastic License 2.0](LICENSE). Our goal with
choosing ELv2 is to maintain an open-source spirit and be as permissive as
possible, while protecting against abuse. Our users can continue to use and
contribute to ParadeDB freely, and we can safely create a sustainable business
and continue to invest in our community, project and product.

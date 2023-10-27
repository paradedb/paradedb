# pg_sparse

## Overview

`pg_sparse` is a PostgreSQL extension that enables similarity search over sparse vectors
in Postgres using HNSW.

## Installation

### From Self-Hosted Postgres

If you are self-hosting Postgres and would like to use the extension within your existing
Postgres, follow these steps:

1. Install Rust and cargo-pgrx:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install cargo-pgrx --version 0.11.0
```

2. Then, run:

```bash
# Clone the repo (optionally pick a specific version)
git clone https://github.com/paradedb/paradedb.git --tag <VERSION>

# Install pg_sparse
cd pg_sparse/
cargo pgrx init --pg<YOUR-POSTGRES-MAJOR_VERSION>=`which pg_config`
cargo pgrx install
```

You can then create the extension in your database by running:

```sql
CREATE EXTENSION pg_sparse;
```

If you are using a managed Postgres service like Amazon RDS, you will not be able to install `pg_sparse` until the Postgres service explicitly supports it.

## Usage

### Indexing

By default, the `pg_sparse` extension creates a table called `paradedb.mock_items`
that you can use for quick experimentation.

To index a table, use the following SQL command:

```sql
CREATE INDEX idx_mock_items
ON paradedb.mock_items
USING sparse_hnsw(sparse_embedding)
```

TODO: Implement index creation, insertion, and scans.

## Development

### Prerequisites

Before developing the extension, ensure that you have Rust installed
(version >1.70), ideally via `rustup` (we've observed issues with installing Rust
via Homebrew on macOS).

Then, install and initialize pgrx:

```bash
cargo install cargo-pgrx --version 0.11.0
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
CREATE EXTENSION pg_sparse;
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
DROP EXTENSION pg_sparse;
CREATE EXTENSION pg_sparse;
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

## License

The `pg_sparse` is licensed under the [GNU Affero General Public License v3.0](../LICENSE).

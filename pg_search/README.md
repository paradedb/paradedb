<h1 align="center">
  <img src="../docs/logo/pg_search.svg" alt="pg_search"></a>
<br>
</h1>

## Overview

`pg_search` is a Postgres extension that enables full text search over heap tables using the BM25 algorithm. It is built on top of Tantivy, the Rust-based alternative to Apache Lucene, using `pgrx`. Please refer to the [ParadeDB documentation](https://docs.paradedb.com/documentation/getting-started/install) to get started.

`pg_search` is supported on official PostgreSQL Global Development Group Postgres versions, starting at PostgreSQL 15.

Benchmarks can be found in the [`/benchmarks` directory](../benchmarks/README.md).

## Installation

### From ParadeDB

The easiest way to use the extension is to run the ParadeDB Dockerfile:

```bash
docker run \
  --name paradedb \
  -e POSTGRES_USER=<user> \
  -e POSTGRES_PASSWORD=<password> \
  -e POSTGRES_DB=<dbname> \
  -v paradedb_data:/var/lib/postgresql/ \
  -p 5432:5432 \
  -d \
  paradedb/paradedb:latest
```

This will spin up a Postgres instance with `pg_search` preinstalled.

### From Self-Hosted PostgreSQL

If you are self-hosting Postgres and would like to use the extension within your existing Postgres, follow the steps below.

It's **very important** to make the following change to your `postgresql.conf` configuration file. `pg_search` must be in the list of `shared_preload_libraries` if your Postgres version is less than 17:

```c
shared_preload_libraries = 'pg_search'
```

This enables the extension to spawn a background worker process that performs writes to the index. If this background process is not started because of an incorrect `postgresql.conf` configuration, your database connection will crash or hang when you attempt to create a `pg_search` index.

#### Debian/Ubuntu

We provide prebuilt binaries for Debian-based Linux for Postgres 15+. You can download the latest version for your architecture from the [releases page](https://github.com/paradedb/paradedb/releases).

#### RHEL/Rocky

We provide prebuilt binaries for Red Hat-based Linux for Postgres 15+. You can download the latest version for your architecture from the [releases page](https://github.com/paradedb/paradedb/releases).

You can also install `pg_search` via [Pigsty](https://pigsty.io/ext/repo/). To install `pig` for `yum` / `dnf` compatible systems [follow these instructions](https://pigsty.io/ext/repo/yum/). Then:

```bash
dnf install pig
pig install pg_search
```

#### macOS

We provide prebuilt binaries for macOS for Postgres 15+. You can download the latest version for your architecture from the [releases page](https://github.com/paradedb/paradedb/releases).

#### Windows

Windows is not supported. This restriction is [inherited from pgrx not supporting Windows](https://github.com/pgcentralfoundation/pgrx?tab=readme-ov-file#caveats--known-issues).

## Development

### Prerequisites

To develop the extension, first install stable Rust using `rustup`:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install stable
```

Note: While it is possible to install Rust via your package manager, we recommend using `rustup` as we've observed inconsistencies with Homebrew's Rust installation on macOS.

Then, install the PostgreSQL version of your choice using your system package manager. Here we provide the commands for the default PostgreSQL version used by this project:

```bash
# macOS
brew install postgresql@18

# Ubuntu
wget --quiet -O - https://www.postgresql.org/media/keys/ACCC4CF8.asc | sudo apt-key add -
sudo sh -c 'echo "deb http://apt.postgresql.org/pub/repos/apt/ $(lsb_release -cs)-pgdg main" > /etc/apt/sources.list.d/pgdg.list'
sudo apt-get update && sudo apt-get install -y postgresql-18 postgresql-server-dev-18

# Arch Linux
sudo pacman -S extra/postgresql
```

If you are using Postgres.app to manage your macOS PostgreSQL, you'll need to add the `pg_config` binary to your path before continuing:

```bash
export PATH="$PATH:/Applications/Postgres.app/Contents/Versions/latest/bin"
```

Then, install and initialize `pgrx`:

```bash
# Note: Replace --pg18 with your version of Postgres, if different (i.e. --pg17, etc.)
cargo install --locked cargo-pgrx --version 0.17.0

# macOS arm64
cargo pgrx init --pg18=/opt/homebrew/opt/postgresql@18/bin/pg_config

# macOS amd64
cargo pgrx init --pg18=/usr/local/opt/postgresql@18/bin/pg_config

# Ubuntu
cargo pgrx init --pg18=/usr/lib/postgresql/18/bin/pg_config

# Arch Linux
cargo pgrx init --pg18=/usr/bin/pg_config
```

If you prefer to use a different version of Postgres, update the `--pg` flag accordingly.

Note: While it is possible to develop using pgrx's own Postgres installation(s), via `cargo pgrx init` without specifying a `pg_config` path, we recommend using your system package manager's Postgres as we've observed inconsistent behaviours when using pgrx's.

`pgrx` requires `libclang`, which does not come by default on Linux. To install it:

```bash
# Ubuntu
sudo apt install libclang-dev

# Arch Linux
sudo pacman -S extra/clang
```

#### pgvector

`pgvector` is needed for hybrid search integration tests.

```bash
# Note: Replace 18 with your version of Postgres
git clone --branch v0.8.1 https://github.com/pgvector/pgvector.git
cd pgvector/

# macOS arm64
PG_CONFIG=/opt/homebrew/opt/postgresql@18/bin/pg_config make
sudo PG_CONFIG=/opt/homebrew/opt/postgresql@18/bin/pg_config make install # may need sudo

# macOS amd64
PG_CONFIG=/usr/local/opt/postgresql@18/bin/pg_config make
sudo PG_CONFIG=/usr/local/opt/postgresql@18/bin/pg_config make install # may need sudo

# Ubuntu
PG_CONFIG=/usr/lib/postgresql/18/bin/pg_config make
sudo PG_CONFIG=/usr/lib/postgresql/18/bin/pg_config make install # may need sudo

# Arch Linux
PG_CONFIG=/usr/bin/pg_config make
sudo PG_CONFIG=/usr/bin/pg_config make install # may need sudo
```

### Running the Extension

First, start pgrx:

```bash
cargo pgrx run
```

This will launch an interactive connection to Postgres. Inside Postgres, create the extension by running:

```sql
CREATE EXTENSION pg_search;
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
DROP EXTENSION pg_search;
CREATE EXTENSION pg_search;
```

### Testing

This section covers the **unit tests** located in the `pg_search/src` directory. For a complete overview of ParadeDB's testing infrastructure (which includes integration tests, client property tests, and pg regress tests), please see the [Testing section in `CONTRIBUTING.md`](../CONTRIBUTING.md#testing).

Unit tests can be run using `cargo test -p pg_search -- a_specific_method_to_run`. These tests sometimes (if marked `#[pg_test]`) run inside the Postgres process, which allows them to use all of Postgres APIs via `pgrx`. There is no need to pre-install the extension for these: the `#[pg_test]` annotation on the test will re-install the extension automatically.

_Note: For **pg regress tests**, please refer to [pg_search/tests/pg_regress/README.md](tests/pg_regress/README.md). For **integration tests** and **client property tests**, please refer to [tests/README.md](../tests/README.md)._

## License

`pg_search` is licensed under the [GNU Affero General Public License v3.0](../LICENSE) and as commercial software. For commercial licensing, please contact us at [sales@paradedb.com](mailto:sales@paradedb.com).

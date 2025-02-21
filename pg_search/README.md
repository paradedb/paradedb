<h1 align="center">
  <img src="../docs/logo/pg_search.svg" alt="pg_search"></a>
<br>
</h1>

[![Test pg_search](https://github.com/paradedb/paradedb/actions/workflows/test-pg_search.yml/badge.svg)](https://github.com/paradedb/paradedb/actions/workflows/test-pg_search.yml)
[![Benchmark pg_search](https://github.com/paradedb/paradedb/actions/workflows/benchmark-pg_search.yml/badge.svg)](https://github.com/paradedb/paradedb/actions/workflows/benchmark-pg_search.yml)

## Overview

`pg_search` is a Postgres extension that enables full text search over heap tables using the BM25 algorithm. It is built on top of Tantivy, the Rust-based alternative to Apache Lucene, using `pgrx`. Please refer to the [ParadeDB documentation](https://docs.paradedb.com/documentation/getting-started/quickstart) to get started.

`pg_search` is supported on all versions supported by the PostgreSQL Global Development Group, which includes PostgreSQL 13+.

Check out the `pg_search` benchmarks [here](../benchmarks/README.md).

`pg_search` uses Tantivy version 0.23.

## Installation

### From ParadeDB

The easiest way to use the extension is to run the ParadeDB Dockerfile:

```bash
docker run \
  --name paradedb \
  -e POSTGRES_USER=<user> \
  -e POSTGRES_PASSWORD=<password> \
  -e POSTGRES_DB=<dbname> \
  -v paradedb_data:/var/lib/postgresql/data/ \
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

We provide prebuilt binaries for Debian-based Linux for Postgres 14, 15, 16, and 17. You can download the latest version for your architecture from the [releases page](https://github.com/paradedb/paradedb/releases).

Our prebuilt binaries come with the ICU tokenizer enabled, which requires the `libicu` library. If you don't have it installed, you can do so with:

```bash
# Ubuntu 20.04 or 22.04
sudo apt-get install -y libicu70

# Ubuntu 24.04
sudo apt-get install -y libicu74
```

Or, you can compile the extension from source without `--features icu` to build without the ICU tokenizer.

ParadeDB collects anonymous telemetry to help us understand how many people are using the project. You can opt out of telemetry by setting `export PARADEDB_TELEMETRY=false` (or unsetting the variable) in your shell or in your `~/.bashrc` file before running the extension.

#### macOS

We don't suggest running production workloads on macOS. As a result, we don't provide prebuilt binaries for macOS. If you are running Postgres on macOS and want to install `pg_search`, please follow the [development](#development) instructions, but do `cargo pgrx install --release` instead of `cargo pgrx run`. This will build the extension from source and install it in your Postgres instance.

You can then create the extension in your database by running:

```sql
CREATE EXTENSION pg_search;
```

Note: If you are using a managed Postgres service like Amazon RDS, you will not be able to install `pg_search` until the Postgres service explicitly supports it.

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
brew install postgresql@17

# Ubuntu
wget --quiet -O - https://www.postgresql.org/media/keys/ACCC4CF8.asc | sudo apt-key add -
sudo sh -c 'echo "deb http://apt.postgresql.org/pub/repos/apt/ $(lsb_release -cs)-pgdg main" > /etc/apt/sources.list.d/pgdg.list'
sudo apt-get update && sudo apt-get install -y postgresql-17 postgresql-server-dev-17

# Arch Linux
sudo pacman -S extra/postgresql
```

If you are using Postgres.app to manage your macOS PostgreSQL, you'll need to add the `pg_config` binary to your path before continuing:

```bash
export PATH="$PATH:/Applications/Postgres.app/Contents/Versions/latest/bin"
```

Then, install and initialize `pgrx`:

```bash
# Note: Replace --pg17 with your version of Postgres, if different (i.e. --pg16, etc.)
cargo install --locked cargo-pgrx --version 0.12.7

# macOS arm64
cargo pgrx init --pg17=/opt/homebrew/opt/postgresql@17/bin/pg_config

# macOS amd64
cargo pgrx init --pg17=/usr/local/opt/postgresql@17/bin/pg_config

# Ubuntu
cargo pgrx init --pg17=/usr/lib/postgresql/17/bin/pg_config

# Arch Linux
cargo pgrx init --pg17=/usr/bin/pg_config
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

`pgvector` needed for hybrid search unit tests.

```bash
# Note: Replace 17 with your version of Postgres
git clone --branch v0.8.0 https://github.com/pgvector/pgvector.git
cd pgvector/

# macOS arm64
PG_CONFIG=/opt/homebrew/opt/postgresql@17/bin/pg_config make
sudo PG_CONFIG=/opt/homebrew/opt/postgresql@17/bin/pg_config make install # may need sudo

# macOS amd64
PG_CONFIG=/usr/local/opt/postgresql@17/bin/pg_config make
sudo PG_CONFIG=/usr/local/opt/postgresql@17/bin/pg_config make install # may need sudo

# Ubuntu
PG_CONFIG=/usr/lib/postgresql/17/bin/pg_config make
sudo PG_CONFIG=/usr/lib/postgresql/17/bin/pg_config make install # may need sudo

# Arch Linux
PG_CONFIG=/usr/bin/pg_config make
sudo PG_CONFIG=/usr/bin/pg_config make install # may need sudo
```

#### ICU Tokenizer

`pg_search` comes with multiple tokenizers for different languages. The ICU tokenizer is not enabled by default in development due to the additional dependencies it requires. To develop with the ICU tokenizer enabled, first:

Ensure that the `libicu` library is installed. It should come preinstalled on most distros, but you can install it with your system package manager if it isn't:

```bash
# macOS
brew install icu4c

# Ubuntu 20.04 or 22.04
sudo apt-get install -y libicu70

# Ubuntu 24.04
sudo apt-get install -y libicu74

# Arch Linux
sudo pacman -S core/icu

# Debian 12 (Bookworm)
# Note: Debian Bookworm ships libicu72, which is no longer supported in the rust_icu crate
# These instructions compile and libicu76 from source
curl -L -o icu4c-76_1-src.tgz https://github.com/unicode-org/icu/releases/download/release-76-1/icu4c-76_1-src.tgz && tar -xzf icu4c-76_1-src.tgz

cd /icu/source/
./runConfigureICU Linux --prefix=/opt/icu76
make "-j$(nproc)" && make install
```

On Debian, you'll need to compile it from source

Additionally, on macOS and Debian you'll need to add the `icu-config` binary to your path before continuing:

```bash
# macOS arm64
export PKG_CONFIG_PATH="/opt/homebrew/opt/icu4c/lib/pkgconfig"

# macOS amd64
export PKG_CONFIG_PATH="/usr/local/opt/icu4c/lib/pkgconfig"

# Debian 12 (Bookworm)
export ICU_CONFIG=/opt/icu76/bin/icu-config
```

Finally, to enable the ICU tokenizer in development, pass `--features icu` to the `cargo pgrx run` and `cargo pgrx test` commands.

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

We use `cargo test` as our runner for `pg_search` tests.

The tests require a `DATABASE_URL` envirobnment variable to be set. The easiest way to do this is to create a `.env` file with the following contents.

```env
DATABASE_URL=postgres://USER_NAME@localhost:PORT/pg_search
```

USER_NAME should be replaced with your system user name. (eg: output of `whoami`)

PORT should be replaced with 28800 + your postgres version. (eg: 28817 for Postgres 17)

## License

`pg_search` is licensed under the [GNU Affero General Public License v3.0](../LICENSE) and as commercial software. For commercial licensing, please contact us at [sales@paradedb.com](mailto:sales@paradedb.com).

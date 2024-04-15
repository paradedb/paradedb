# ParadeDB Cargo Dev Tool

## Installation

The first time you install `cargo-paradedb`, you should navigate to the `cargo-paradedb` crate and run:

```sh
cargo run install
```

After this first-time installation, you can run `cargo paradedb install` from anywhere and Cargo will globally re-install `cargo-paradedb` with the latest code changes from your source folder.

If you don't want to install globally, you can always just `cargo run` from the `cargo-paradedb` crate folder.

### Installing From Git Url

In containers or cloud instances, it's useful to be able to install globally with a single command:

```sh
cargo install --git https://github.com/paradedb/paradedb.git cargo-paradedb
```

This will install the tool for use as `cargo paradedb` without having to clone the repository first. You can also specify a branch:

```sh
cargo install \
    --git https://github.com/paradedb/paradedb.git \
    --branch new-feature-branch \
    cargo-paradedb
```

## Benchmarks

To run benchmarks, you must be able to connect to a running ParadeDB instance. `cargo-paradedb` accepts a Postgres connection url in one of the following ways:
1. A `.env` file with a `DATABASE_URL='postgresql://...'` entry, located in a parent folder of `cargo-paradedb`.
2. Setting the `DATABASE_URL` environment variable when running `cargo paradedb`.
3. Passing a `--url` argument directly to `cargo paradedb bench` commands.

Benchmark tools are run under the `cargo paradedb bench` subcommand, which are organized in `NOUN -> VERB` convention as `DATASET -> ACTION`. The first argument to `cargo paradedb bench` should be which dataset (or "corpus") you would like to benchmark. `eslogs` is the generated corpus for Elasticsearch's benchmarks, and the main corpus that we use for our benchmarks. A example command would be:

```sh
cargo paradedb bench eslogs query-search-index
```

### Generating Benchmark Data

Our benchmarks use the same generated data as the [elasticsearch-opensearch-benchmark](https://github.com/elastic/elasticsearch-opensearch-benchmark) project. To run the data generation tool, you must have [Go](https://go.dev/doc/install) installed. Run the generator tool with:

```sh
cargo paradedb bench eslogs generate
```

In the command above, `generate` can accept arguments to specify a random seed, number of events to generate, table name, and more. Pass `--help` to the `generate` command to see the available options.

The `generate` tool is idempotent. It will produce a table in your Postgres database with the number of events that you asked it to generate. As it generates data, it will periodically commit the `INSERT` transaction to Postgres. If you kill the process, it will pick up where it left off the next time you run it.

### Running Benchmarks

All commands below operate on default tables, visible with `--help`. Defaults can be overidden with options passed to each command.

Benchmarks that build a table or index are only run once, as these operations usually take a long time. Benchmarks that peform fast operations, like queries, are sampled many times with the [Criterion](https://github.com/bheisler/criterion.rs) library.

Build a `pg_search` index:
```sh
cargo paradedb bench eslogs build-search-index
```

Query a `pg_search` index (index must already exist):
```sh
cargo paradedb bench eslogs query-search-index
```

Build a `pg_analytics` table using `parquet`:
```sh
cargo paradedb bench eslogs build-parquet-table
```

Count rows in a `pg_analytics` table using `parquet`:
```sh
cargo paradedb bench eslogs count-parquet-table
```

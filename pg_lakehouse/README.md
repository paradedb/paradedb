<h1 align="center">
  <img src="../docs/logo/pg_lakehouse.svg" alt="pg_lakehouse" width="500px">
<br>
</h1>

## Overview

`pg_lakehouse` is an extension that transforms Postgres into a big data query engine over object stores like S3 and table formats like Apache Iceberg. Queries are pushed down to [Apache DataFusion](https://github.com/apache/datafusion), which delivers excellent analytical performance. Combinations of the following object stores, table formats, and file formats are supported.

### Object Stores

- [x] Amazon S3
- [x] Local file system
- [ ] Azure Blob Storage (coming soon)
- [ ] Google Cloud Storage (coming soon)
- [ ] HDFS (coming soon)

### File Formats

- [x] Parquet
- [x] CSV
- [x] JSON
- [x] Avro
- [ ] ORC (coming soon)

### Table Formats

- [x] Deltalake
- [ ] Apache Iceberg (coming soon)

## Motivation

Today, a vast amount of non-operational data — events, metrics, historical snapshots, vendor data, etc. — is ingested into data lakes like S3. Moving this data into a cloud data warehouse or even Postgres is expensive and time consuming. By allowing companies to query data where it already lives, `pg_lakehouse` eliminates the need for expensive new infrastructure, data movement, and loss of data freshness.

`pg_lakehouse` uses the foreign data wrapper (FDW) API to connect to any object store or table format and the executor hook API to push queries to DataFusion. While other FDWs over object stores like S3 have existed in the Postgres extension ecosystem, these FDWs lack support for most object stores and table formats and are very slow for large analytical workloads. `pg_lakehouse` differentiates itself by supporting a wide breadth of stores and formats and by being very fast.

## Getting Started

The following example uses `pg_lakehouse` to query an example dataset of 3 million NYC taxi trips from January 2024, hosted in a public S3 bucket provided by ParadeDB.

```sql
CREATE EXTENSION pg_lakehouse;
CREATE FOREIGN DATA WRAPPER s3_wrapper HANDLER s3_fdw_handler VALIDATOR s3_fdw_validator;

-- Provide S3 credentials
CREATE SERVER s3_server FOREIGN DATA WRAPPER s3_wrapper
OPTIONS (region 'us-east-1', url 's3://paradedb-benchmarks', skip_signature 'true');

-- Create foreign table
CREATE FOREIGN TABLE trips (
    "VendorID"              INT,
    "tpep_pickup_datetime"  TIMESTAMP,
    "tpep_dropoff_datetime" TIMESTAMP,
    "passenger_count"       INT,
    "trip_distance"         REAL,
    "RatecodeID"            REAL,
    "store_and_fwd_flag"    INT,
    "PULocationID"          REAL,
    "DOLocationID"          REAL,
    "payment_type"          REAL,
    "fare_amount"           REAL,
    "extra"                 REAL,
    "mta_tax"               REAL,
    "tip_amount"            REAL,
    "tolls_amount"          REAL,
    "improvement_surcharge" REAL,
    "total_amount"          REAL
)
SERVER s3_server
OPTIONS (path 's3://paradedb-benchmarks/yellow_tripdata_2024-01.parquet', extension 'parquet');

-- Success! Now you can query the remote Parquet file like a regular Postgres table
SELECT COUNT(*) FROM trips;
  count
---------
 2964624
(1 row)
```

Note that column names must be wrapped in double quotes to preserve uppercase letters. This is because DataFusion is case-sensitive and Postgres' foreign table column names must match the foreign table's column names exactly.

## Query Acceleration

On its own, `pg_lakehouse` is only able to push down column projections, sorts, and limits to DataFusion. This means that queries containing
aggregates, joins, etc. are not fully accelerated.

This can be solved by installing [`pg_analytics`](https://github.com/paradedb/paradedb/tree/dev/pg_analytics#overview), which is able to push down the entirety of most queries over `pg_lakehouse` foreign tables to DataFusion.

```sql
CREATE EXTENSION pg_analytics;
```

Note: In the future, we will move the relevant parts of `pg_analytics` into `pg_lakehouse` so this is no longer necessary.

## Amazon S3

This code block demonstrates how to create a foreign table over S3.

```sql
CREATE FOREIGN DATA WRAPPER s3_wrapper HANDLER s3_fdw_handler VALIDATOR s3_fdw_validator;
CREATE SERVER s3_server FOREIGN DATA WRAPPER s3_wrapper
OPTIONS (
    region 'us-east-1',
    url 's3://path/to/bucket'
    skip_signature 'true'
);

-- Replace the dummy schema with the actual schema of your file data
CREATE FOREIGN TABLE local_file_table ("x" INT)
SERVER s3_server
OPTIONS (path 's3://path/to/file.parquet', extension 'parquet');
```

### S3 Server Options

- `region` (required): AWS region, e.g. `us-east-1`
- `url` (required): Path to the S3 bucket, starting with `s3://`
- `endpoint`: The endpoint for communicating with the S3 instance. Defaults to the [region endpoint](https://docs.aws.amazon.com/general/latest/gr/s3.html). For example, can be set to `http://localhost:4566` if testing against a Localstack instance.
- `allow_http`: If set to `true`, allows both HTTP and HTTPS endpoints. Defaults to `false`.
- `skip_signature`: If set to `true`, will not sign requests. This is useful for connecting to public S3 buckets. Defaults to `false`.

### S3 Table Options

- `path` (required): Must start with `s3://` and point to the location of your file. The path should end in a `/` if it points to a directory of partitioned Parquet files.
- `extension` (required): One of `avro`, `csv`, `json`, and `parquet`.
- `format`: One of `delta` or `iceberg`. If omitted, `pg_lakehouse` assumes that no table format is used.

### S3 Credentials

`CREATE USER MAPPING` is used to pass in credentials for private buckets.

```sql
-- Get the name of the current user
SELECT current_user;
 current_user
--------------
 myuser

-- Run this before CREATE FOREIGN TABLE
CREATE USER MAPPING FOR myuser
SERVER s3_server
OPTIONS (
  access_key_id 'XXXXXX',
  secret_access_key 'XXXXXX'
);

-- Now, run CREATE FOREIGN TABLE
```

Note: To make credentials available to all users, you can set the user to `public`. Valid user mapping options are:

- `access_key_id`: AWS access key ID
- `secret_access_key`: AWS secret access key
- `session_token`: Sets the AWS session token

## Local File System

To be queryable, files must exist on the same machine as your Postgres instance.

```sql
CREATE FOREIGN DATA WRAPPER local_file_wrapper HANDLER local_file_fdw_handler VALIDATOR local_file_fdw_validator;
CREATE SERVER local_file_server FOREIGN DATA WRAPPER local_file_wrapper;

-- Replace the dummy schema with the actual schema of your file data
CREATE FOREIGN TABLE local_file_table ("x" INT)
SERVER local_file_server
OPTIONS (path 'file:///path/to/file.parquet', extension 'parquet');
```

### Local File Table Options

- `path` (required): Must start with `file:///` and point to the location of your file. The path should end in a `/` if it points to a directory of partitioned Parquet files.
- `extension` (required): One of `avro`, `csv`, `json`, and `parquet`.
- `format`: Only `delta` is accepted for the Deltalake format. If omitted, no table format is assumed.

## Data Movement

### To Postgres

Moving data from a data lake into Postgres can be done in a single query.

```sql
-- Assumes you have created the trips table from the quickstart
CREATE TABLE trips_copy AS SELECT * FROM trips;
```

### To Data Lake

Writes to data lakes are not yet supported but are coming soon.

## Development

### Install Rust

To develop the extension, first install Rust v1.77.2 using `rustup`. We will soon make the extension compatible with newer versions of Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install 1.77.2

# We recommend setting the default version to 1.77.2  for consistency across your system
rustup default 1.77.2
```

Note: While it is possible to install Rust via your package manager, we recommend using `rustup` as we've observed inconcistencies with Homebrew's Rust installation on macOS.

Then, install the PostgreSQL version of your choice using your system package manager. Here we provide the commands for the default PostgreSQL version used by this project:

### Install Postgres

```bash
# macOS
brew install postgresql@16

# Ubuntu
wget --quiet -O - https://www.postgresql.org/media/keys/ACCC4CF8.asc | sudo apt-key add -
sudo sh -c 'echo "deb http://apt.postgresql.org/pub/repos/apt/ $(lsb_release -cs)-pgdg main" > /etc/apt/sources.list.d/pgdg.list'
sudo apt-get update && sudo apt-get install -y postgresql-16 postgresql-server-dev-16
```

If you are using Postgres.app to manage your macOS PostgreSQL, you'll need to add the `pg_config` binary to your path before continuing:

```bash
export PATH="$PATH:/Applications/Postgres.app/Contents/Versions/latest/bin"
```

### Install pgrx

Then, install and initialize `pgrx`:

```bash
# Note: Replace --pg16 with your version of Postgres, if different (i.e. --pg15, --pg14, etc.)
cargo install --locked cargo-pgrx --version 0.11.3

# macOS arm64
cargo pgrx init --pg16=/opt/homebrew/opt/postgresql@16/bin/pg_config

# macOS amd64
cargo pgrx init --pg16=/usr/local/opt/postgresql@16/bin/pg_config

# Ubuntu
cargo pgrx init --pg16=/usr/lib/postgresql/16/bin/pg_config
```

If you prefer to use a different version of Postgres, update the `--pg` flag accordingly.

Note: While it is possible to develop using pgrx's own Postgres installation(s), via `cargo pgrx init` without specifying a `pg_config` path, we recommend using your system package manager's Postgres as we've observed inconsistent behaviours when using pgrx's.

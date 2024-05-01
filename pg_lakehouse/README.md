<h1 align="center">
  <img src="../docs/logo/pg_lakehouse.svg" alt="pg_lakehouse" width="500px">
<br>
</h1>

## Overview

`pg_lakehouse` is an extension that transforms Postgres into an analytical query engine over data lakes like S3. Queries are pushed down to [Apache DataFusion](https://github.com/apache/datafusion), which significantly improves performance. Any combination of the following object stores, table formats, and file formats is supported.

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

Today, an enormous amount of time and money is spent running and moving data into cloud data warehouses. At the same time, much of this data already lives in S3. `pg_lakehouse` is allows companies to build a data warehouse on top of their existing Postgres and S3 with zero new infrastructure, data movement, loss of data freshness, or vendor lock-in.

`pg_lakehouse` uses the foreign data wrapper (FDW) API to connect to data lakes and the executor hook API to push queries to DataFusion. While FDWs over S3 and Parquet exist in the Postgres extension ecosystem, these FDWs lack support for many file and table formats and are very slow for large analytical workloads. `pg_lakehouse` differentiates itself by supporting these formats and by being very fast.

## Getting Started

The following example uses `pg_lakehouse` to query an example dataset of NYC taxi trips from January 2024, hosted in a public S3 bucket provided by ParadeDB.

```sql
CREATE EXTENSION pg_lakehouse;
CREATE FOREIGN DATA WRAPPER s3_wrapper HANDLER s3_fdw_handler VALIDATOR s3_fdw_validator;

-- Provide S3 credentials
CREATE SERVER s3_server FOREIGN DATA WRAPPER s3_wrapper
OPTIONS (region 'us-east-1', url 's3://paradedb-benchmarks', skip_signature 'true');

-- Create foreign table
CREATE FOREIGN TABLE trips (
    trip_id             INT,
    pickup_datetime     TIMESTAMP,
    dropoff_datetime    TIMESTAMP,
    pickup_longitude    DOUBLE PRECISION,
    pickup_latitude     DOUBLE PRECISION,
    dropoff_longitude   DOUBLE PRECISION,
    dropoff_latitude    DOUBLE PRECISION,
    passenger_count     SMALLINT,
    trip_distance       REAL,
    fare_amount         REAL,
    extra               REAL,
    tip_amount          REAL,
    tolls_amount        REAL,
    total_amount        REAL,
    payment_type        INT,
    pickup_ntaname      REAL,
    dropoff_ntaname     REAL
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

## Query Acceleration

On its own, `pg_lakehouse` is only able to push down column projections, sorts, and limits to DataFusion. This means that queries containing
aggregates, joins, etc. are not fully accelerated.

This can be solved by installing [`pg_analytics`](https://github.com/paradedb/paradedb/tree/dev/pg_analytics#overview), which is able to push down the entirety of most queries over `pg_lakehouse` foreign tables to DataFusion.

```sql
CREATE EXTENSION pg_analytics;
```

## Amazon S3

This code block demonstrates how to create a foreign table over S3.

```sql
CREATE FOREIGN DATA WRAPPER s3_wrapper HANDLER s3_fdw_handler VALIDATOR s3_fdw_validator;
CREATE SERVER s3_server FOREIGN DATA WRAPPER s3_wrapper
OPTIONS (
    region 'us-east-1',
    url 's3://path/to/bucket'
    access_key_id 'XXXXXXX'
    secret_access_key 'XXXXXXX'
);

-- Replace the dummy schema with the actual schema of your file data
CREATE FOREIGN TABLE local_file_table (x INT)
SERVER s3_server
OPTIONS (path 's3://path/to/file.parquet', extension 'parquet');
```

### S3 Server Options

- `region` (required): AWS region, e.g. `us-east-1`
- `url` (required): Path to the S3 bucket, starting with `s3://`
- `access_key_id`: AWS access key ID
- `secret_access_key`: AWS secret access key
- `endpoint`: The endpoint for communicating with the S3 instance. Defaults to the [region endpoint](https://docs.aws.amazon.com/general/latest/gr/s3.html). For example, can be set to `http://localhost:4566` if testing against a Localstack instance.
- `session_token`: Sets the AWS session token
- `allow_http`: If set to `true`, allows both HTTP and HTTPS endpoints. Defaults to `false`.
- `skip_signature`: If set to `true`, will not sign requests. This is useful for connecting to public S3 buckets. Defaults to `false`.

User permissions should be set accordingly, as Postgres stores these credentials in the `pg_catalog.pg_foreign_server` table and can be seen by anyone with access to this table. Additional credential security measures will be introduced in future updates.

### S3 Table Options

- `path` (required): Must start with `s3://` and point to the location of your file. The path should end in a `/` if it points to a directory of partitioned Parquet files.
- `extension` (required): One of `avro`, `csv`, `json`, and `parquet`.
- `format`: One of `delta` or `iceberg`. If omitted, no table format is assumed.

## Local File System

To be queryable, files must exist on the same machine as your Postgres instance.

```sql
CREATE FOREIGN DATA WRAPPER local_file_wrapper HANDLER local_file_fdw_handler VALIDATOR local_file_fdw_validator;
CREATE SERVER local_file_server FOREIGN DATA WRAPPER local_file_wrapper;

-- Replace the dummy schema with the actual schema of your file data
CREATE FOREIGN TABLE local_file_table (x INT)
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

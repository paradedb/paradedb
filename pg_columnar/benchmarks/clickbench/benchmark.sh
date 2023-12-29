#!/bin/bash

# This script benchmarks the performance of pg_columnar using the ClickBench benchmkark
# suite. It is supported on both Ubuntu and macOS, for local development via `cargo` as
# well as in CI testing via Docker.
#
# The local development version runs a smaller subset of the hits dataset, hits_100k_rows.csv,
# which is a randomly sampled 100,000 rows from the full ClickBench dataset, hits.csv. It is roughly
# 0.1% (~0.075GB) of the full dataset of 100M rows (~75GB). Other dataset sizes available inckude:
# - hits_500k_rows.csv.gz
# - hits_1m_rows.csv.gz
# - hits_2m_rows.csv.gz
# - hits_5m_rows.csv.gz
# The local development version is intended for quick iteration and is
# designed to be run via `cargo clickbench`, instead of running this script directly.
#
# The CI version runs the full ClickBench dataset, hits.csv, which is roughly 100M rows
# (~75GB). The CI version is intended for use in CI and official benchmarking, and is
# designed to be run directly via `./benchmark.sh`.

# Exit on subcommand errors
set -Eeuo pipefail

# Handle params
usage() {
  echo "Usage: $0 [OPTIONS]"
  echo "Options:"
  echo " -h (optional),  Display this help message"
  echo " -t (optional),  Version tag to benchmark:"
  echo "                  - 'x.y.z'  Runs the full ClickBench benchmark against a specific ParadeDB Docker image (e.g. 0.3.1)"
  echo "                  - 'latest' Runs the full ClickBench benchmark the latest ParadeDB Docker image"
  echo "                  - 'local'  Runs the full ClickBench benchmark the current commit inside a local ParadeDB Docker build"
  echo "                  - 'pgrx'   Runs a minified ClickBench benchmark against the current commit inside the pg_columnar pgrx PostgreSQL instance"
  echo " -s (optional),  Type of storage layout to benchmark:"
  echo "                  - 'hot'                 Runs with in-memory storage using PostgreSQL's CREATE TEMP TABLE"
  echo "                  - 'cold'                Runs with on-disk storage using PostgreSQL's CREATE TABLE"
  echo "                  - 'parquet-single'      Runs with on-disk storage using PostgreSQL's CREATE EXTERNAL TABLE using a single Parquet file"
  echo "                  - 'parquet-partitioned' Runs with on-disk storage using PostgreSQL's CREATE EXTERNAL TABLE using partitioned Parquet files"
  exit 1
}

# Instantiate vars
FLAG_TAG="pgrx"
FLAG_STORAGE="hot"
TRIES=3

# Assign flags to vars and check
while getopts "ht:s:" flag
do
  case $flag in
    h)
      usage
      ;;
    t)
      FLAG_TAG=$OPTARG
      ;;
    s)
      FLAG_STORAGE=$OPTARG
      ;;
    *)
      usage
      ;;
  esac
done

# Determine the base directory of the script
BASEDIR=$(dirname "$0")
cd "$BASEDIR/"

# Cleanup function to reset the environment
cleanup() {
  echo ""
  echo "Cleaning up benchmark environment..."
  if [ "$FLAG_TAG" == "pgrx" ]; then
    # Check if PostgreSQL is in recovery mode. This can happen if one of the quer caused a crash. If
    # so, we need to wait for recovery to finish before we can drop the extension.
    for attempt in {1..5}; do
      psql -h localhost -p 28815 -d pg_columnar -t -c 'DROP EXTENSION IF EXISTS pg_columnar CASCADE;' &> /dev/null && break
      echo "PostgreSQL is in recovery mode (likely due to a crash), waiting for recovery to finish..."
      sleep 5
    done
    if [ $attempt -eq 5 ]; then
      echo "Failed to drop pg_columnar extension after several attempts. PostgreSQL is likely still in recovery mode."
    fi
    cargo pgrx stop
  else
    if docker ps -q --filter "name=paradedb" | grep -q .; then
      docker kill paradedb > /dev/null 2>&1
    fi
    docker rm paradedb > /dev/null 2>&1
  fi
  echo "Done, goodbye!"
}

# Register the cleanup function to run when the script exits
trap cleanup EXIT

echo ""
echo "*********************************************************************************"
echo "* Benchmarking pg_columnar version '$FLAG_TAG' against ClickBench on '$FLAG_STORAGE' storage..."
echo "*********************************************************************************"
echo ""

if [ "$FLAG_TAG" == "pgrx" ]; then
  # For local benchmarking via pgrx, we download hits_100k_rows.csv, which is ~5M rows (~3.75GB)
  if [ ! -e hits_100k_rows.csv ]; then
    echo "Downloading hits_100k_rows.csv dataset..."
    wget --no-verbose --continue https://paradedb-benchmarks.s3.amazonaws.com/hits_100k_rows.csv.gz
    gzip -d hits_100k_rows.csv.gz
    chmod 666 hits_100k_rows.csv
  else
    echo "Dataset already exists, skipping download..."
  fi

  # Build pg_columnar and start its pgrx PostgreSQL instance
  echo ""
  echo "Building pg_columnar..."
  cargo pgrx stop
  cargo pgrx install
  cargo pgrx start

  # Run the benchmarking
  if [ "$FLAG_STORAGE" = "hot" ]; then
    # For hot storage, we create a temporary table, load all the data in it, and run the queries
    # within the same session. Queries are run directly from the `copy_hot.sql` file
    psql -h localhost -p 28815 -d pg_columnar -t < copy_hot.sql
  elif [ "$FLAG_STORAGE" = "cold" ]; then
    # For cold storage, we create a permanent table, load all the data in it, and run the queries
    # once, printing the output of each query to the terminal
    echo "Creating pg_columnar..."
    psql -h localhost -p 28815 -d pg_columnar -t < create_cold.sql
    echo "Loading data..."
    psql -h localhost -p 28815 -d pg_columnar -t < copy_cold.sql
    echo "Running queries..."
    psql -h localhost -p 28815 -d pg_columnar -t < queries.sql
  elif [ "$FLAG_STORAGE" = "parquet-single" ]; then
    echo "TODO: Implement pgrx + Parquet single storage benchmarking"
  elif [ "$FLAG_STORAGE" = "parquet-partitioned" ]; then
    echo "TODO: Implement pgrx + Parquet partitioned storage benchmarking"
  else
    echo "Invalid storage type: $FLAG_STORAGE"
    usage
  fi
  # For local benchmarking via pgrx, we don't print the disk usage or parse the results into
  # the format expected by the ClickBench dashboard
else
  # For CI/official benchmarking via Docker, we download the full hits.csv dataset, which is ~100M rows (~75GB)
  if [ ! -e hits.csv ]; then
    echo "Downloading hits.csv dataset..."
    wget --no-verbose --continue 'https://datasets.clickhouse.com/hits_compatible/hits.csv.gz'
    gzip -d hits.csv.gz
    chmod 666 hits.csv
  else
    echo "Dataset already exists, skipping download..."
  fi

  # If the version tag is "local", we build the ParadeDB Docker image from source to test the current commit
  if [ "$FLAG_TAG" == "local" ]; then
    echo "Building ParadeDB Docker image from source..."
    docker build -t paradedb/paradedb:"$FLAG_TAG" \
      -f "../../../docker/Dockerfile" \
      --build-arg PG_VERSION_MAJOR="15" \
      --build-arg PG_BM25_VERSION="0.0.0" \
      --build-arg PG_SPARSE_VERSION="0.0.0" \
      --build-arg PG_COLUMNAR_VERSION="0.0.0" \
      --build-arg PGVECTOR_VERSION="0.5.1" \
      "../../../"
    echo ""
  fi

  # Install and run Docker container for ParadeDB in detached mode
  echo "Spinning up ParadeDB $FLAG_TAG Docker image..."
  docker run \
    -d \
    --name paradedb \
    -e POSTGRES_USER=myuser \
    -e POSTGRES_PASSWORD=mypassword \
    -e POSTGRES_DB=mydatabase \
    -p $PORT:5432 \
    paradedb/paradedb:"$FLAG_TAG"

  # Wait for Docker container to spin up
  echo ""
  echo "Waiting for ParadeDB Docker image to spin up..."
  sleep 5
  echo "Done!"

  # Run the benchmarking
  if [ "$FLAG_STORAGE" = "hot" ]; then
    echo "TODO: Implement Docker + hot storage benchmarking"
  elif [ "$FLAG_STORAGE" = "cold" ]; then
    echo "TODO: Implement Docker + cold storage benchmarking"
  elif [ "$FLAG_STORAGE" = "parquet-single" ]; then
    echo "TODO: Implement Docker + Parquet single storage benchmarking"
  elif [ "$FLAG_STORAGE" = "parquet-partitioned" ]; then
    echo "TODO: Implement Docker + Parquet partitioned storage benchmarking"
  else
    echo "Invalid storage type: $FLAG_STORAGE"
    usage
  fi
fi

echo "Benchmark complete!"

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
    if [ "$attempt" -eq 5 ]; then
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

# Download function to retrieve the dataset and verify its checksum
download_and_verify() {
  local url=$1
  local checksum=$2
  local filename=$3

  # Check if the file already exists and verify its checksum
  if [ -e "$filename" ]; then
    if echo "$checksum  $filename" | md5sum -c --status; then
      echo "Dataset '$filename' already exists and is verified, skipping download..."
      return
    else
      echo "Checksum mismatch. Re-downloading '$filename'..."
    fi
  fi

  # Downloading the file
  echo "Downloading $filename dataset..."
  wget --no-verbose --continue -O "$filename.gz" "$url"
  gzip -d "$filename.gz"
  chmod 666 "$filename"
}

echo ""
echo "*********************************************************************************"
echo "* Benchmarking pg_columnar version '$FLAG_TAG' against ClickBench on '$FLAG_STORAGE' storage..."
echo "*********************************************************************************"
echo ""

if [ "$FLAG_TAG" == "pgrx" ]; then
  # For local benchmarking via pgrx, we download hits_100k_rows.csv, which is ~5M rows (~3.75GB)
  download_and_verify "https://paradedb-benchmarks.s3.amazonaws.com/hits_100k_rows.csv.gz" "06b18e929bc94ea93706b782d8b1120e" "hits_100k_rows.csv"
  echo ""

  # Rust nightly is required for SIMD support, which is mandatory for benchmarking as
  # it is a major performance boost
  CURRENT_RUST_TOOLCHAIN=$(rustup show active-toolchain)
  if [[ $CURRENT_RUST_TOOLCHAIN != *"nightly"* ]]; then
    echo "Switching to Rust nightly toolchain for maximum performance via SIMD..."
    rustup override unset
    rustup update nightly
    rustup default nightly

    echo "Reinstalling cargo-pgrx on Rust nightly toolchain..."
    cargo install --locked cargo-pgrx --version 0.11.1 --force
  else
    echo "Already on Rust nightly toolchain, skipping toolchain switch..."
  fi

  # Build pg_columnar and start its pgrx PostgreSQL instance
  echo ""
  echo "Building pg_columnar in release mode with SIMD support..."
  cargo pgrx stop
  cargo pgrx install --features simd --release
  cargo pgrx start
  echo ""

  # Run the benchmarking
  if [ "$FLAG_STORAGE" = "hot" ]; then
    # Table creation, data loading, and query execution are all done from `benchmark_hot.sql`
    psql -h localhost -p 28815 -d pg_columnar -t < benchmark_hot.sql
  elif [ "$FLAG_STORAGE" = "cold" ]; then
    # Table creation, data loading, and query execution are all done from `benchmark_cold.sql`
    psql -h localhost -p 28815 -d pg_columnar -t < benchmark_cold.sql
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
  download_and_verify "https://datasets.clickhouse.com/hits_compatible/hits.csv.gz" "TODO" "hits.tsv"

  # If the version tag is "local", we build the ParadeDB Docker image from source to test the current commit
  if [ "$FLAG_TAG" == "local" ]; then
    echo "Building ParadeDB Docker image from source..."
    docker build -t paradedb/paradedb:"$FLAG_TAG" \
      -f "../../../docker/Dockerfile" \
      --build-arg PG_VERSION_MAJOR="15" \
      --build-arg PG_BM25_VERSION="0.0.0" \
      --build-arg PG_COLUMNAR_VERSION="0.0.0" \
      --build-arg PG_SPARSE_VERSION="0.0.0" \
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
    -p "$PORT":5432 \
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

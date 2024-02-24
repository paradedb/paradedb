#!/bin/bash

# This script benchmarks the performance of pg_analytics using the ClickBench benchmkark
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
  echo "                  - 'pgrx'   Runs a minified ClickBench benchmark against the current commit inside the pg_analytics pgrx PostgreSQL instance"
  exit 1
}

# Instantiate vars
FLAG_TAG="pgrx"
DOCKER_PORT=5432

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
      psql -h localhost -p 28816 -d pg_analytics -t -c 'DROP EXTENSION IF EXISTS pg_analytics CASCADE;' &> /dev/null && break
      echo "PostgreSQL is in recovery mode (likely due to a crash), waiting for recovery to finish..."
      sleep 5
    done
    if [ "$attempt" -eq 5 ]; then
      echo "Failed to drop pg_analytics extension after several attempts. PostgreSQL is likely still in recovery mode."
    fi
    cargo pgrx stop
  else
    if docker ps -q --filter "name=paradedb" | grep -q .; then
      docker kill paradedb > /dev/null 2>&1
    fi
    docker rm paradedb > /dev/null 2>&1
  fi

  # Delete the log.txt file, if it exists
  if [ -f "log.txt" ]; then
    rm -rf "log.txt"
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
echo "* Benchmarking pg_analytics version '$FLAG_TAG' against ClickBench"
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
    echo "Switching to Rust nightly toolchain for SIMD..."
    rustup override unset
    rustup update nightly
    rustup default nightly

    echo "Reinstalling cargo-pgrx on Rust nightly toolchain..."
    cargo install --locked cargo-pgrx --version 0.11.2 --force
  else
    echo "Already on Rust nightly toolchain, skipping toolchain switch..."
  fi

  # Retrieve pg_config path
  OS_NAME=$(uname)
  case "$OS_NAME" in
    Darwin)
      # Check arch to set proper pg_config path
      if [ "$(uname -m)" = "arm64" ]; then
        PG_CONFIG_PATH="/opt/homebrew/opt/postgresql@16/bin/pg_config"
      elif [ "$(uname -m)" = "x86_64" ]; then
        PG_CONFIG_PATH="/usr/local/opt/postgresql@16/bin/pg_config"
      else
        echo "Unknown arch, exiting..."
        exit 1
      fi
      ;;
    Linux)
      PG_CONFIG_PATH="/usr/lib/postgresql/16/bin/pg_config"
      ;;
  esac

  # Build pg_analytics and start its pgrx PostgreSQL instance
  echo ""
  echo "Building pg_analytics in release mode with SIMD..."
  cargo pgrx stop
  cargo pgrx init "--pg16=$PG_CONFIG_PATH"
  cargo pgrx install "--pg-config=$PG_CONFIG_PATH"
  cargo pgrx start
  echo ""

  # Run the benchmarking
  psql -h localhost -p 28816 -d pg_analytics -t < benchmark.sql
  # For local benchmarking via pgrx, we don't print the disk usage or parse the results into
  # the format expected by the ClickBench dashboard
else
  # For CI benchmarking via Docker, we have a few dataset options:
  # - hits_5m.tsv.gz: 5M rows (~3.75GB)
  download_and_verify "https://paradedb-benchmarks.s3.amazonaws.com/hits_5m_rows.tsv.gz" "0dd087f3b6c8262fb962bd262163d402" "hits.tsv"
  # - hits.tsv.gz: 100M rows (~75GB) (full dataset)
  # download_and_verify "https://datasets.clickhouse.com/hits_compatible/hits.tsv.gz" "5ef60063da951e18ae3fa929c9f3aad4" "hits.tsv"

  # If the version tag is "local", we build the ParadeDB Docker image from source to test the current commit
  if [ "$FLAG_TAG" == "local" ]; then
    echo "Building ParadeDB Docker image from source..."
    docker build \
      --tag paradedb/paradedb:"$FLAG_TAG" \
      --build-arg POSTGRESQL_USERNAME=myuser \
      --build-arg POSTGRESQL_PASSWORD=mypassword \
      --build-arg POSTGRESQL_DATABASE=mydatabase \
      --build-arg POSTGRESQL_POSTGRES_PASSWORD=postgres \
      --file "../../../docker/Dockerfile" \
      "../../../"
    echo ""
  fi

  # Install and run Docker container for ParadeDB in detached mode
  echo "Spinning up ParadeDB $FLAG_TAG Docker image..."
  docker run \
    --name paradedb \
    -e POSTGRESQL_USERNAME=myuser \
    -e POSTGRESQL_PASSWORD=mypassword \
    -e POSTGRESQL_DATABASE=mydatabase \
    -e POSTGRESQL_POSTGRES_PASSWORD=postgres \
    -p $DOCKER_PORT:5432 \
    -d \
    paradedb/paradedb:"$FLAG_TAG"

  # Wait for Docker container to spin up
  echo ""
  echo "Waiting for ParadeDB Docker image to spin up..."
  sleep 10
  echo "Done!"

  echo ""
  echo "Loading dataset..."
  psql -h localhost -U myuser -d mydatabase -p 5432 -t < create.sql
  psql -h localhost -U myuser -d mydatabase -p 5432 -t -c '\timing' -c "\\copy hits FROM 'hits.tsv'"

  echo ""
  echo "Running queries..."
  ./run.sh 2>&1 | tee log.txt

  echo ""
  echo "Printing disk usage..."
  sudo docker exec paradedb du -bcs /bitnami/postgresql/data

  echo ""
  echo "Printing results..."
  grep -oP 'Time: \d+\.\d+ ms' log.txt | sed -r -e 's/Time: ([0-9]+\.[0-9]+) ms/\1/' |
  awk '{ if (i % 3 == 0) { printf "[" }; printf $1 / 1000; if (i % 3 != 2) { printf "," } else { print "]," }; ++i; }'
fi

echo ""
echo "Benchmark complete!"

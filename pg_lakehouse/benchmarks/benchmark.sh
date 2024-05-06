#!/bin/bash

# This script benchmarks the performance of pg_lakehouse using the ClickBench benchmkark
# suite. It is supported on both Ubuntu and macOS.

# Exit on subcommand errors
set -Eeuo pipefail

# Handle params
usage() {
  echo "Usage: $0 [OPTIONS]"
  echo "Options:"
  echo " -h (optional),  Display this help message"
  echo " -t (optional),  Version tag to benchmark (default: local):"
  echo "                  - 'x.y.z'  Runs the full ClickBench benchmark against a specific ParadeDB Docker image (e.g. 0.3.1)"
  echo "                  - 'latest' Runs the full ClickBench benchmark the latest ParadeDB Docker image"
  echo "                  - 'local'  Runs the full ClickBench benchmark the current commit inside a local ParadeDB Docker build"
  echo " -w (optional),  Workload to benchmark (default: single):"
  echo "                  - 'single' Runs the full ClickBench benchmark against a single Parquet file"
  echo "                  - 'partitioned' Runs the full ClickBench benchmark against one hundred partitioned Parquet files"
  exit 1
}

# Instantiate vars
FLAG_TAG="local"
WORKLOAD="single"
DOCKER_PORT=5432
OS=$(uname)

# Assign flags to vars and check
while getopts "ht:w:s:" flag
do
  case $flag in
    h)
      usage
      ;;
    t)
      FLAG_TAG=$OPTARG
      ;;
    w)
      WORKLOAD=$OPTARG
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
  # If the container successfully started, print the logs. This is
  # helpful to debug scenarios where the container starts but the
  # Postgres server crashes.
  echo ""
  echo "Printing Docker logs..."
  docker logs paradedb
  echo ""
  echo "Cleaning up benchmark environment..."
  if docker ps -q --filter "name=paradedb" | grep -q .; then
    docker kill paradedb > /dev/null 2>&1
  fi
  docker rm paradedb > /dev/null 2>&1

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
    echo "Dataset '$filename' already exists, verifying checksum..."
    if echo "$checksum $filename" | md5sum -c --status; then
      echo "Dataset '$filename' already exists and is verified, skipping download..."
      return
    else
      echo "Checksum mismatch. Re-downloading '$filename'..."
    fi
  fi

  # Downloading the file
  echo "Downloading $filename dataset..."
  wget --no-verbose --continue -O "$filename" "$url"
}

echo ""
echo "*********************************************************************************"
echo "* Benchmarking pg_lakehouse version '$FLAG_TAG' against ClickBench"
echo "*********************************************************************************"
echo ""

# For CI benchmarking via Docker, use the full dataset (hits.parquet: 100M rows ~15GBs)
if [ "$WORKLOAD" == "single" ]; then
  echo "Using ClickBench's single Parquet file for benchmarking..."
  download_and_verify "https://datasets.clickhouse.com/hits_compatible/hits.parquet" "e903fd8cc8462a454df107390326844a" "hits.parquet"
elif [ "$WORKLOAD" == "partitioned" ]; then
  # TODO: Clean this up
  echo "Using ClickBench's one hundred partitioned Parquet files for benchmarking..."
fi

# If the version tag is "local", we build the ParadeDB Docker image from source to test the current commit
if [ "$FLAG_TAG" == "local" ]; then
  echo "Building ParadeDB Docker image from source..."
  docker build \
    --tag paradedb/paradedb:"$FLAG_TAG" \
    --build-arg POSTGRESQL_USERNAME=myuser \
    --build-arg POSTGRESQL_PASSWORD=mypassword \
    --build-arg POSTGRESQL_DATABASE=mydatabase \
    --build-arg POSTGRESQL_POSTGRES_PASSWORD=postgres \
    --file "../../docker/Dockerfile" \
    "../../"
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

# We use the postgres superuser as it is required to create a foreign-data wrapper
echo ""
echo "Loading dataset..."
export PGPASSWORD='postgres'
if [ "$WORKLOAD" == "single" ]; then
  docker cp "hits.parquet" paradedb:/tmp/hits.parquet
elif [ "$WORKLOAD" == "partitioned" ]; then
  for file in partitioned/*.parquet; do
    docker cp "$file" paradedb:/tmp/
  done
else
  echo "Invalid workload: $WORKLOAD"
  exit 1
fi
psql -h localhost -U postgres -d mydatabase -p 5432 -t < create.sql

echo ""
echo "Running queries..."
./run.sh 2>&1 | tee log.txt

echo ""
echo "Printing disk usage..."
if [ "$OS" == "Linux" ]; then
  sudo docker exec paradedb du -bcs /bitnami/postgresql/data
else
  docker exec paradedb du -bcs /bitnami/postgresql/data
fi

echo ""
echo "Printing results..."
if [ "$OS" == "Linux" ]; then
  grep -oP 'Time: \d+\.\d+ ms' log.txt | sed -r -e 's/Time: ([0-9]+\.[0-9]+) ms/\1/' |
  awk '{ if (i % 3 == 0) { printf "[" }; printf $1 / 1000; if (i % 3 != 2) { printf "," } else { print "]," }; ++i; }'
else
  grep -E -o 'Time: [0-9]+\.[0-9]+ ms' log.txt | sed -E -e 's/Time: ([0-9]+\.[0-9]+) ms/\1/' |
  awk '{ if (i % 3 == 0) { printf "[" }; printf $1 / 1000; if (i % 3 != 2) { printf "," } else { print "]," }; ++i; }'
fi

echo ""
echo "Benchmark complete!"

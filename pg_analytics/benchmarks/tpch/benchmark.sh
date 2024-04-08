#!/bin/bash


# This script benchmarks the performance of pg_analytics using the TPC-H benchmark
# suite. It is supported on both Ubuntu and macOS, for local development via `cargo` as
# well as in CI testing via Docker.
#
# The local development versions runs the smaller dataset
# TODO: Write description

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
      docker kill paradedb
    fi
    docker rm paradedb
  fi
  echo "Done, goodbye!"
}

# Register the cleanup function to run when the script exits
trap cleanup EXIT

# Download function to retrieve the dataset
download_dataset() {
  local url=$1
  local filename=$2

  # Check if the file already exists
  if [ -e "$filename" ]; then
    echo "Dataset '$filename' already exists, skipping download..."
    return
  fi

  # Downloading the TPC-H folder
  echo "Downloading $filename dataset..."
  wget --no-verbose --continue -O "$filename" "$url"
  unzip "$filename"
}

echo ""
echo "*********************************************************************************"
echo "* Benchmarking pg_analytics version '$FLAG_TAG' against TPC-H"
echo "*********************************************************************************"
echo ""

if [ "$FLAG_TAG" == "pgrx" ]; then
  # TODO
  echo "TODO"
else
  # TODO: Make the various dataset sizes compatible here
  # Actually, I'm using pre-generated datasets because dbgen doesn't work on macOS. Need to update here
  download_dataset "https://paradedb-benchmarks.s3.amazonaws.com/TPC-H_V3.0.1.zip" "TPC-H_V3.0.1.zip"

  # If the version tag is "local", we build the ParadeDB Docker image from source to test the current commit
  if [ "$FLAG_TAG" == "local" ]; then
    echo "Building ParadeDB Docker image from source..."
    docker build -t paradedb/paradedb:"$FLAG_TAG" \
      -f "../../../docker/Dockerfile" \
      "../../../"
    echo ""
  fi

  # Install and run Docker container for ParadeDB in detached mode
  echo ""
  echo "Spinning up ParadeDB $FLAG_TAG Docker image..."
  docker run \
    -d \
    --name paradedb \
    -e POSTGRES_USER=myuser \
    -e POSTGRES_PASSWORD=mypassword \
    -e POSTGRES_DB=mydatabase \
    -p "$DOCKER_PORT":5432 \
    paradedb/paradedb:"$FLAG_TAG"

  # Wait for Docker container to spin up
  echo ""
  echo "Waiting for ParadeDB Docker image to spin up..."
  sleep 10
  echo "Done!"

  echo ""
  echo "Loading dataset..."
  export PGPASSWORD='mypassword'
  psql -h localhost -U myuser -d mydatabase -p 5432 -t < create.sql

  # TODO: Handle the data generation + loading, this here is broken
  psql -h localhost -U myuser -d mydatabase -p 5432 -t -c "CALL paradedb.init();" -c '\timing' -c "\\COPY nation FROM 'TPC-H_V3.0.1/ref_data/1/nation.tbl.1' WITH (FORMAT CSV, DELIMITER '|')"
  psql -h localhost -U myuser -d mydatabase -p 5432 -t -c "CALL paradedb.init();" -c '\timing' -c "\\COPY customer FROM 'TPC-H_V3.0.1/ref_data/1/customer.tbl.1' WITH (FORMAT CSV, DELIMITER '|')"
  psql -h localhost -U myuser -d mydatabase -p 5432 -t -c "CALL paradedb.init();" -c '\timing' -c "\\COPY supplier FROM 'TPC-H_V3.0.1/ref_data/1/supplier.tbl.1' WITH (FORMAT CSV, DELIMITER '|')"
  psql -h localhost -U myuser -d mydatabase -p 5432 -t -c "CALL paradedb.init();" -c '\timing' -c "\\COPY part FROM 'TPC-H_V3.0.1/ref_data/1/part.tbl.1' WITH (FORMAT CSV, DELIMITER '|')"
  psql -h localhost -U myuser -d mydatabase -p 5432 -t -c "CALL paradedb.init();" -c '\timing' -c "\\COPY partsupp FROM 'TPC-H_V3.0.1/ref_data/1/partsupp.tbl.1' WITH (FORMAT CSV, DELIMITER '|')"
  psql -h localhost -U myuser -d mydatabase -p 5432 -t -c "CALL paradedb.init();" -c '\timing' -c "\\COPY orders FROM 'TPC-H_V3.0.1/ref_data/1/orders.tbl.1' WITH (FORMAT CSV, DELIMITER '|')"
  psql -h localhost -U myuser -d mydatabase -p 5432 -t -c "CALL paradedb.init();" -c '\timing' -c "\\COPY lineitem FROM 'TPC-H_V3.0.1/ref_data/1/lineitem.tbl.1' WITH (FORMAT CSV, DELIMITER '|')"

  echo ""
  echo "Running queries..."
  ./run.sh 2>&1 | tee log.txt

  echo ""
  echo "Printing disk usage..."
  sudo docker exec paradedb du -bcs /var/lib/postgresql/data

  echo ""
  echo "Printing results..."
  grep -oP 'Time: \d+\.\d+ ms' log.txt | sed -r -e 's/Time: ([0-9]+\.[0-9]+) ms/\1/' |
  awk '{ if (i % 3 == 0) { printf "[" }; printf $1 / 1000; if (i % 3 != 2) { printf "," } else { print "]," }; ++i; }'
fi

echo ""
echo "Benchmark complete!"

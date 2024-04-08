#!/bin/bash

# This script benchmarks the performance of pg_analytics using the TPC-H benchmkark
# suite. It is supported on both Ubuntu and macOS.

# Exit on subcommand errors
set -Eeuo pipefail

# Handle params
usage() {
  echo "Usage: $0 [OPTIONS]"
  echo "Options:"
  echo " -h (optional),  Display this help message"
  echo " -t (optional),  Version tag to benchmark:"
  echo "                  - 'x.y.z'  Runs the full TPC-H benchmark against a specific ParadeDB Docker image (e.g. 0.3.1)"
  echo "                  - 'latest' Runs the full TPC-H benchmark the latest ParadeDB Docker image"
  echo "                  - 'local'  Runs the full TPC-H benchmark the current commit inside a local ParadeDB Docker build"
  echo " -w (optional),  Workload to benchmark:"
  echo "                  - 'olap' Runs the TPC-H benchmark against all pg_analytics 'parquet' tables"
  echo "                  - 'htap' Runs the TPC-H benchmark against a combination of pg_analytics 'parquet' and Postgres 'heap' tables"
  exit 1
}

# Instantiate vars
FLAG_TAG="local"
WORKLOAD="olap"
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
    if echo "$checksum $filename" | md5sum -c --status; then
      echo "Dataset '$filename' already exists and is verified, skipping download..."
      return
    else
      echo "Checksum mismatch. Re-downloading '$filename'..."
    fi
  fi

  # Downloading the file (TPC-H data generation file)
  echo "Downloading $filename dataset..."
  wget --no-verbose --continue -O "$filename" "$url"
  unzip "$filename"
  rm -rf "$filename" # Remove the zip file
}

# Generate the TPC-H dataset
generate_dataset() {
  echo ""
  echo "Generating TPC-H dataset..."
  cd TPC-H_V3.0.1/dbgen
  make
  ./dbgen -s 1 -f
  cd ../..
}

echo ""
echo "*********************************************************************************"
echo "* Benchmarking pg_analytics version '$FLAG_TAG' against TPC-H"
echo "*********************************************************************************"
echo ""

# Download the data generation tool and generate the dataset
download_and_verify "https://paradedb-benchmarks.s3.amazonaws.com/TPC-H_V3.0.1.zip" "bc82f852c6b6f31002a4c2dffa3efbb3" "TPC-H_V3.0.1.zip"
generate_dataset

# If the version tag is "local", we build the ParadeDB Docker image from source to test the current commit
if [ "$FLAG_TAG" == "local" ]; then
  echo ""
  echo "Building ParadeDB Docker image from source..."
  docker build \
    --tag paradedb/paradedb:"$FLAG_TAG" \
    --build-arg POSTGRESQL_USERNAME=myuser \
    --build-arg POSTGRESQL_PASSWORD=mypassword \
    --build-arg POSTGRESQL_DATABASE=mydatabase \
    --build-arg POSTGRESQL_POSTGRES_PASSWORD=postgres \
    --file "../../../docker/Dockerfile" \
    "../../../"
fi

# Install and run Docker container for ParadeDB in detached mode
echo ""
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
export PGPASSWORD='mypassword'
if [ "$WORKLOAD" == "olap" ]; then
  echo "Loading OLAP tables..."
  psql -h localhost -U myuser -d mydatabase -p 5432 -t < create_olap.sql
elif [ "$WORKLOAD" == "htap" ]; then
  echo "Loading HTAP tables..."
  psql -h localhost -U myuser -d mydatabase -p 5432 -t < create_htap.sql
fi
psql -h localhost -U myuser -d mydatabase -p 5432 -t -c '\timing' -c "\\COPY nation FROM 'TPC-H_V3.0.1/dbgen/nation.tbl' DELIMITER '|' CSV"
psql -h localhost -U myuser -d mydatabase -p 5432 -t -c '\timing' -c "\\COPY region FROM 'TPC-H_V3.0.1/dbgen/region.tbl' DELIMITER '|' CSV"
psql -h localhost -U myuser -d mydatabase -p 5432 -t -c '\timing' -c "\\COPY customer FROM 'TPC-H_V3.0.1/dbgen/customer.tbl' DELIMITER '|' CSV"
psql -h localhost -U myuser -d mydatabase -p 5432 -t -c '\timing' -c "\\COPY supplier FROM 'TPC-H_V3.0.1/dbgen/supplier.tbl' DELIMITER '|' CSV"
psql -h localhost -U myuser -d mydatabase -p 5432 -t -c '\timing' -c "\\COPY part FROM 'TPC-H_V3.0.1/dbgen/part.tbl' DELIMITER '|' CSV"
psql -h localhost -U myuser -d mydatabase -p 5432 -t -c '\timing' -c "\\COPY partsupp FROM 'TPC-H_V3.0.1/dbgen/partsupp.tbl' DELIMITER '|' CSV"
psql -h localhost -U myuser -d mydatabase -p 5432 -t -c '\timing' -c "\\COPY orders FROM 'TPC-H_V3.0.1/dbgen/orders.tbl' DELIMITER '|' CSV"
psql -h localhost -U myuser -d mydatabase -p 5432 -t -c '\timing' -c "\\COPY lineitem FROM 'TPC-H_V3.0.1/dbgen/lineitem.tbl' DELIMITER '|' CSV"

echo ""
echo "Running queries..."
./run.sh 2>&1 | tee log.txt

echo ""
echo "Printing disk usage..."
OS=$(uname)
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

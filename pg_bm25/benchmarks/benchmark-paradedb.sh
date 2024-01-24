#!/bin/bash

# This script benchmarks the performance of ParadeDB, specifically pg_bm25, for index
# and search time.

# Exit on subcommand errors
set -Eeuo pipefail

# Handle params
usage() {
  echo "Usage: $0 [OPTIONS]"
  echo "Options:"
  echo " -h (optional),   Display this help message"
  echo " -t (optional),   Docker tag to use for paradedb/paradedb:tag. Use 'local' to build from source. Use 'pgrx' to run against pgrx instead of Docker. Default: 'latest'"
  exit 1
}

# Instantiate vars
FLAG_TAG="latest"

# Assign flags to vars and check
while getopts "ht:" flag
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

# Vars
PORT=5431
OUTPUT_CSV=out/benchmark_paradedb.csv

# Ensure the "out" directory exists
mkdir -p out/

# Cleanup function to stop and remove the Docker container
cleanup() {
  if [ -s query_error.log ]; then
    echo "!!! Benchmark cleanup triggered !!!"
    cat query_error.log
    rm -rf query_error.log
  fi
  echo ""
  echo "Cleaning up benchmark environment..."
  if [ "$FLAG_TAG" == "pgrx" ]; then
    db_query "DROP EXTENSION pg_bm25 CASCADE;"
    cd ../pg_bm25/
    cargo pgrx stop
    cd ../benchmarks/
  else
    if docker ps -q --filter "name=paradedb" | grep -q .; then
      docker kill paradedb > /dev/null 2>&1
    fi
    docker rm paradedb > /dev/null 2>&1
  fi
  rm -rf query_output.log
  echo "Done, goodbye!"
}

# Register the cleanup function to run when the script exits
trap cleanup EXIT

echo ""
echo "*******************************************************"
echo "* Benchmarking ParadeDB version: $FLAG_TAG"
echo "*******************************************************"
echo ""

# If the tag is "pgrx", run the benchmark against pgrx instead of Docker. If the tag
# is "local", build ParadeDB from source to test the current commit
if [ "$FLAG_TAG" == "pgrx" ]; then
  echo "Building pg_bm25 Extension from Source..."
  cd ../pg_bm25/
  cargo build
  cargo pgrx start
  cd ../benchmarks/
elif [ "$FLAG_TAG" == "local" ]; then
  echo "Building ParadeDB Dockerfile from Source..."
  docker build -t paradedb/paradedb:"$FLAG_TAG" \
    -f "../../docker/Dockerfile" \
    "../../"
  echo ""
else
  echo "Pulling ParadeDB $FLAG_TAG from Docker Hub..."
fi

# If the tag is "pgrx", define the right parameters to run the benchmarks
# against pgrx instead of Docker. Otherwise, spin up a Docker container in detached mode
if [ "$FLAG_TAG" == "pgrx" ]; then
  export PG_HOST="localhost"
  export PG_PORT="28815"
  export PG_DATABASE="pg_bm25"
  export PG_USER=""
  export PG_PASSWORD=""
  export USING_PGRX=true
else
  echo "Spinning up ParadeDB $FLAG_TAG Docker container..."
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
  echo "Waiting for Docker container to spin up..."
  sleep 5
  echo "Done!"
fi

# Source the script to load data into the database
# shellcheck disable=SC1091
source "helpers/get_data.sh"

# If we're using pgrx, we need to manually create the pg_bm25 extension
if [ "$FLAG_TAG" == "pgrx" ]; then
  echo ""
  echo "Creating pg_bm25 extension..."
  db_query "CREATE EXTENSION pg_bm25;"
fi

# Load data into database
echo ""
echo "Loading data into database..."
load_data
echo "Done!"

# Output file for recording times
echo "Table Size,Index Time,Search Time" > "$OUTPUT_CSV"

# If the tag is "pgrx", we only run a single test for 1M rows. Otherwise, we run the full list
# of tests between 10,000 and 5M rows
if [ "$FLAG_TAG" == "pgrx" ]; then
  TABLE_SIZES=(100000)
else
  # Table sizes to be processed (in number of rows). The maximum is 5M rows with the Wikipedia dataset
  TABLE_SIZES=(10000 50000 100000 200000 300000 400000 500000 600000 700000 800000 900000 1000000 1500000 2000000 2500000 3000000 3500000 4000000 4500000 5000000)
fi

for SIZE in "${TABLE_SIZES[@]}"; do
  echo ""
  echo "Running benchmarking suite on table with $SIZE rows..."
  TABLE_NAME="wikipedia_articles_$SIZE"
  INDEX_NAME="search_index_$SIZE"

  # Create temporary table with limit
  echo "-- Creating temporary table with $SIZE rows..."
  db_query "CREATE TABLE IF NOT EXISTS $TABLE_NAME AS SELECT * FROM wikipedia_articles LIMIT $SIZE;"
  db_query "ALTER TABLE $TABLE_NAME ADD COLUMN IF NOT EXISTS id SERIAL"

  # Time indexing
  echo "-- Timing indexing..."
  start_time=$( { time db_query "CALL paradedb.create_bm25(index_name => '$INDEX_NAME', table_name => '$TABLE_NAME', key_field => 'id', text_fields => '{\"url\": {}, \"title\": {}, \"body\": {}}');" > query_output.log 2> query_error.log ; } 2>&1 )
  index_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  # Time search
  echo "-- Timing search..."
  start_time=$( { time db_query "SELECT * FROM $INDEX_NAME.search('body:Canada', limit_rows => 10);" > query_output.log 2> query_error.log ; } 2>&1 )
  search_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  # Record times to CSV
  echo "$SIZE,$index_time,$search_time" >> $OUTPUT_CSV

  # Print query plan
  echo "-- Printing query plan..."
  db_query "EXPLAIN SELECT * FROM $INDEX_NAME.search('body:Canada') LIMIT 10;"

  # Cleanup: drop temporary table and index
  echo "-- Cleaning up..."
  db_query "CALL paradedb.drop_bm25('$INDEX_NAME');"
  db_query "DROP TABLE $TABLE_NAME;"
  echo "Done!"
done

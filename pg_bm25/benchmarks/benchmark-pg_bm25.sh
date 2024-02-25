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
  echo " -t (optional),   Docker tag to use for paradedb/paradedb:tag. Use 'local' to build from source. Default: 'latest'"
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

PORT=5431
OUTPUT_CSV=out/benchmark_pg_bm25.csv

# Ensure the "out" directory exists
mkdir -p out/

# shellcheck disable=SC1091
source "helpers/get_data.sh"

# Cleanup function to stop and remove the Docker container
cleanup() {
  if [ -s query_error.log ]; then
    echo "!!! Benchmark cleanup triggered !!!"
    cat query_error.log
    rm -rf query_error.log
  fi
  echo ""
  echo "Cleaning up benchmark environment..."
  if docker ps -q --filter "name=paradedb" | grep -q .; then
    docker kill paradedb > /dev/null 2>&1
  fi
  docker rm paradedb > /dev/null 2>&1
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

# If the tag is "local", build ParadeDB from source to test the current commit
if [ "$FLAG_TAG" == "local" ]; then
  echo "Building ParadeDB From Source..."
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
echo "Spinning up ParadeDB $FLAG_TAG server..."
docker run \
  --name paradedb \
  -e POSTGRESQL_USERNAME=myuser \
  -e POSTGRESQL_PASSWORD=mypassword \
  -e POSTGRESQL_DATABASE=mydatabase \
  -e POSTGRESQL_POSTGRES_PASSWORD=postgres \
  -p $PORT:5432 \
  -d \
  paradedb/paradedb:"$FLAG_TAG"

# Wait for Docker container to spin up
echo ""
echo "Waiting for server to spin up..."
sleep 10
echo "Done!"

# Load data into database
echo ""
echo "Loading data into database..."
load_data
echo "Done!"

# Output file for recording times
echo "Table Size,Index Time,Search Time" > "$OUTPUT_CSV"

# Table sizes to be processed (in number of rows). The maximum is 5M rows with the Wikipedia dataset
TABLE_SIZES=(10000 50000 100000 200000 300000 400000 500000 600000 700000 800000 900000 1000000 1500000 2000000 2500000 3000000 3500000 4000000 4500000 5000000)

for SIZE in "${TABLE_SIZES[@]}"; do
  echo ""
  echo "Running benchmarking suite on table with $SIZE rows..."
  TABLE_NAME="wikipedia_articles_$SIZE"
  INDEX_NAME="search_index_$SIZE"

  # Create temporary table with limit
  echo "-- Creating temporary table with $SIZE rows..."
  db_query "CREATE TABLE $TABLE_NAME AS SELECT * FROM wikipedia_articles LIMIT $SIZE;"
  db_query "ALTER TABLE $TABLE_NAME ADD COLUMN id SERIAL"

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

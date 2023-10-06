#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Ensure the "out" directory exists
mkdir -p out

# shellcheck disable=SC1091
source "helpers/get_data.sh"

PORT=5431
PARADEDB_VERSION=latest
OUTPUT_CSV=out/benchmark_paradedb.csv

# Cleanup function to stop and remove the Docker container
cleanup() {
  echo ""
  echo "Cleaning up benchmark environment..."
  if docker ps -q --filter "name=paradedb" | grep -q .; then
    docker kill paradedb
  fi
  docker rm paradedb
  echo "Done!"
}

# Register the cleanup function to run when the script exits
trap cleanup EXIT

echo ""
echo "*******************************************************"
echo "Benchmarking ParadeDB version: $PARADEDB_VERSION"
echo "*******************************************************"
echo ""

# 1. Install and run docker container for paradedb in detached mode
echo "Spinning up ParadeDB $PARADEDB_VERSION server..."
docker run \
  -d \
  --name paradedb \
  -e POSTGRES_USER=myuser \
  -e POSTGRES_PASSWORD=mypassword \
  -e POSTGRES_DB=mydatabase \
  -p $PORT:5432 \
  paradedb/paradedb:$PARADEDB_VERSION

# Wait for Docker container to spin up
echo ""
echo "Waiting for server to spin up..."
sleep 5
echo "Done!"

# 2. Load data into database
echo ""
echo "Loading data into database..."
load_data
echo "Done!"

# Output file for recording times
echo "Table Size,Index Time,Search Time" > $OUTPUT_CSV

# Table sizes to be processed (in number of rows). You can modify this to go up to 5 million rows with the Wikipedia dataset.
#TABLE_SIZES=(10000 50000 100000 200000 300000 400000 500000 600000 700000 800000 900000 1000000 2000000 3000000)
TABLE_SIZES=(10000 5000000)

for SIZE in "${TABLE_SIZES[@]}"; do
  echo ""
  echo "Running benchmarking suite on table with $SIZE rows..."
  TABLE_NAME="wikipedia_articles_$SIZE"
  INDEX_NAME="search_index_$SIZE"

  # Create temporary table with limit
  echo "-- Creating temporary table with $SIZE rows..."
  db_query localhost "$PORT" mydatabase myuser mypassword "CREATE TABLE $TABLE_NAME AS SELECT * FROM wikipedia_articles LIMIT $SIZE;"

  # Time indexing
  echo "-- Timing indexing..."
  start_time=$( (time db_query localhost "$PORT" mydatabase myuser mypassword "CREATE INDEX $INDEX_NAME ON $TABLE_NAME USING bm25 (($TABLE_NAME.*)) WITH (text_fields='{\"url\": {}, \"title\": {}, \"body\": {}}');" > /dev/null) 2>&1 )
  index_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  # Time search
  echo "-- Timing search..."
  start_time=$( (time db_query localhost "$PORT" mydatabase myuser mypassword "SELECT * FROM $TABLE_NAME WHERE $TABLE_NAME @@@ 'Canada' LIMIT 10" > /dev/null) 2>&1 )
  search_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  # Record times to CSV
  echo "$SIZE,$index_time,$search_time" >> $OUTPUT_CSV

  # Cleanup: drop temporary table and index
  echo "-- Cleaning up..."
  db_query localhost "$PORT" mydatabase myuser mypassword "DROP TABLE $TABLE_NAME;"
  db_query localhost "$PORT" mydatabase myuser mypassword "DROP INDEX IF EXISTS $INDEX_NAME;"
  echo "Done!"
done

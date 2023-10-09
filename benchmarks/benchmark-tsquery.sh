#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Ensure the "out" directory exists
mkdir -p out

# shellcheck disable=SC1091
source "helpers/get_data.sh"

PORT=5431
PG_VERSION=15.4
OUTPUT_CSV=out/benchmark_tsquery.csv

# Cleanup function to stop and remove the Docker container
cleanup() {
  echo ""
  echo "Cleaning up benchmark environment..."
  if docker ps -q --filter "name=postgres" | grep -q .; then
    docker kill postgres
  fi
  docker rm postgres
  echo "Done!"
}

# Register the cleanup function to run when the script exits
trap cleanup EXIT

echo ""
echo "*******************************************************"
echo "* Benchmarking tsquery for PostgreSQL version: $PG_VERSION"
echo "*******************************************************"
echo ""

# Install and run Docker container for PostgreSQL in detached mode
echo "Spinning up official PostgreSQL $PG_VERSION server..."
docker run \
  -d \
  --name postgres \
  -e POSTGRES_USER=myuser \
  -e POSTGRES_PASSWORD=mypassword \
  -e POSTGRES_DB=mydatabase \
  -p $PORT:5432 \
  postgres:$PG_VERSION

# Wait for Docker container to spin up
echo ""
echo "Waiting for server to spin up..."
sleep 5
echo "Done!"

# Load data into database
echo ""
echo "Loading data into database..."
load_data
echo "Done!"

# Output file for recording times
echo "Table Size,Index Time,Search Time" > $OUTPUT_CSV

# Table sizes to be processed (in number of rows). The maximum is 5M rows with the Wikipedia dataset
TABLE_SIZES=(10000 50000 100000 200000 300000 400000 500000 600000 700000 800000 900000 1000000 2000000 3000000 4000000 5000000)

for SIZE in "${TABLE_SIZES[@]}"; do
  echo ""
  echo "Running benchmarking suite on table with $SIZE rows..."
  TABLE_NAME="wikipedia_articles_$SIZE"
  INDEX_NAME="search_index_$SIZE"

  # Create temporary table with limit
  echo "-- Creating temporary table with $SIZE rows..."
  db_query "CREATE TABLE $TABLE_NAME AS SELECT * FROM wikipedia_articles LIMIT $SIZE;"
  db_query "ALTER TABLE $TABLE_NAME ADD COLUMN search_vector tsvector;"

  # Time indexing -- we include vector creation with the indexing metric because it is a required setup for search via tsvector
  echo "-- Timing indexing..."
  start_time=$( (time db_query "UPDATE $TABLE_NAME SET search_vector = to_tsvector('english', title) || to_tsvector('english', body); CREATE INDEX $INDEX_NAME ON $TABLE_NAME USING gin(search_vector);" > /dev/null) 2>&1 )
  index_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  # Time search
  echo "-- Timing search..."
  start_time=$( (time db_query "SELECT title, body, ts_rank_cd(search_vector, query) as rank FROM $TABLE_NAME, to_tsquery('Canada') query WHERE query @@ search_vector ORDER BY rank DESC LIMIT 10;" > /dev/null) 2>&1 )
  search_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  # Record times to CSV
  echo "$SIZE,$index_time,$search_time" >> $OUTPUT_CSV

  # Cleanup: drop temporary table
  echo "-- Cleaning up..."
  db_query "DROP TABLE $TABLE_NAME;"
  echo "Done!"
done

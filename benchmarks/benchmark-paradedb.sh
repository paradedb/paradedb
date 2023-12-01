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
OUTPUT_CSV=out/benchmark_paradedb.csv

# Ensure the "out" directory exists
mkdir -p out/

# shellcheck disable=SC1091
source "helpers/get_data.sh"

# Cleanup function to stop and remove the Docker container
cleanup() {
  if [ -s query_error.log ]; then
    echo "!!! Benchmark cleanup triggered !!!"
    cat query_error.log
  fi
  echo ""
  echo "Cleaning up benchmark environment..."
  if docker ps -q --filter "name=paradedb" | grep -q .; then
    docker kill paradedb > /dev/null 2>&1
  fi
  docker rm paradedb > /dev/null 2>&1
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
  docker build -t paradedb/paradedb:"$FLAG_TAG" \
    --no-cache \
    -f "../docker/Dockerfile" \
    --build-arg PG_VERSION_MAJOR="15" \
    --build-arg PG_BM25_VERSION="0.0.0" \
    --build-arg PG_SEARCH_VERSION="0.0.0" \
    --build-arg PG_SPARSE_VERSION="0.0.0" \
    --build-arg PG_GRAPHQL_VERSION="1.3.0" \
    --build-arg PG_JSONSCHEMA_VERSION="0.1.4" \
    --build-arg PGSQL_HTTP_VERSION="1.6.0" \
    --build-arg PG_NET_VERSION="0.7.2" \
    --build-arg PGVECTOR_VERSION="0.5.1" \
    --build-arg PG_CRON_VERSION="1.6.2" \
    --build-arg PG_IVM_VERSION="1.7" \
    --build-arg PG_HASHIDS_VERSION="1.2.1" \
    --build-arg PG_REPACK_VERSION="1.5.0" \
    --build-arg PG_STAT_MONITOR_VERSION="2.0.3" \
    --build-arg PG_HINT_PLAN_VERSION="1.5.1" \
    --build-arg PG_ROARINGBITMAP_VERSION="0.5.4" \
    --build-arg PGFACETING_VERSION="0.1.0" \
    --build-arg PGTAP_VERSION="1.3.1" \
    --build-arg PGAUDIT_VERSION="1.7.0" \
    --build-arg POSTGIS_VERSION="3.4.1" \
    --build-arg PGROUTING_VERSION="3.6.1" \
    --build-arg HYPOPG_VERSION="1.4.0" \
    --build-arg RUM_VERSION="1.3.13" \
    --build-arg AGE_VERSION="1.4.0" \
    "../"
  echo ""
fi

# Install and run Docker container for ParadeDB in detached mode
echo "Spinning up ParadeDB $FLAG_TAG server..."
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
echo "Waiting for server to spin up..."
sleep 5
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
  start_time=$( { time db_query "CREATE INDEX $INDEX_NAME ON $TABLE_NAME USING bm25 (($TABLE_NAME.*)) WITH (key_field='id', text_fields='{\"url\": {}, \"title\": {}, \"body\": {}}');" > query_output.log 2> query_error.log ; } 2>&1 )
  index_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  # Time search
  echo "-- Timing search..."
  start_time=$( { time db_query "SELECT * FROM $TABLE_NAME WHERE $TABLE_NAME @@@ 'body:Canada' LIMIT 10" > query_output.log 2> query_error.log ; } 2>&1 )
  search_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  # Record times to CSV
  echo "$SIZE,$index_time,$search_time" >> $OUTPUT_CSV

  # Print query plan
  echo "-- Printing query plan..."
  db_query "EXPLAIN SELECT * FROM $TABLE_NAME WHERE $TABLE_NAME @@@ 'body:Canada' LIMIT 10"

  # Cleanup: drop temporary table and index
  echo "-- Cleaning up..."
  db_query "DROP INDEX IF EXISTS $INDEX_NAME;"
  db_query "DROP TABLE $TABLE_NAME;"
  echo "Done!"
done

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
echo "* Benchmarking ParadeDB version: $FLAG_TAG"
echo "*******************************************************"
echo ""

# If the tag is "local", build ParadeDB from source to test the current commit
if [ "$FLAG_TAG" == "local" ]; then
  echo "Building ParadeDB From Source..."
  docker build -t paradedb/paradedb:"$FLAG_TAG" \
    -f "../docker/Dockerfile" \
    --build-arg PG_VERSION_MAJOR=15 \
    --build-arg PG_BM25_VERSION=0.0.0 \
    --build-arg PG_SEARCH_VERSION=0.0.0 \
    --build-arg PG_SPARSE_VERSION=0.0.0 \
    --build-arg PGVECTOR_VERSION="$(jq -r '.extensions.pgvector.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg PG_CRON_VERSION="$(jq -r '.extensions.pg_cron.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg PG_NET_VERSION="$(jq -r '.extensions.pg_net.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg PG_IVM_VERSION="$(jq -r '.extensions.pg_ivm.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg PG_GRAPHQL_VERSION="$(jq -r '.extensions.pg_graphql.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg PG_HASHIDS_VERSION="$(jq -r '.extensions.pg_hashids.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg PG_JSONSCHEMA_VERSION="$(jq -r '.extensions.pg_jsonschema.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg PG_REPACK_VERSION="$(jq -r '.extensions.pg_repack.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg PG_STAT_MONITOR_VERSION="$(jq -r '.extensions.pg_stat_monitor.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg PG_HINT_PLAN_VERSION="$(jq -r '.extensions.pg_hint_plan.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg PG_ROARINGBITMAP_VERSION="$(jq -r '.extensions.pg_roaringbitmap.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg PGFACETING_VERSION="$(jq -r '.extensions.pgfaceting.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg PGTAP_VERSION="$(jq -r '.extensions.pgtap.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg PGAUDIT_VERSION="$(jq -r '.extensions.pgaudit.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg POSTGIS_VERSION="$(jq -r '.extensions.postgis.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg PGROUTING_VERSION="$(jq -r '.extensions.pgrouting.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg PGSQL_HTTP_VERSION="$(jq -r '.extensions.pgsql_http.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg HYPOPG_VERSION="$(jq -r '.extensions.hypopg.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg RUM_VERSION="$(jq -r '.extensions.rum.version' '../conf/third_party_pg_extensions.json')" \
    --build-arg AGE_VERSION="$(jq -r '.extensions.age.version' '../conf/third_party_pg_extensions.json')" \
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

  # Time indexing
  echo "-- Timing indexing..."
  start_time=$( (time db_query "CREATE INDEX $INDEX_NAME ON $TABLE_NAME USING bm25 (($TABLE_NAME.*)) WITH (text_fields='{\"url\": {}, \"title\": {}, \"body\": {}}');" > /dev/null) 2>&1 )
  index_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  # Time search
  echo "-- Timing search..."
  start_time=$( (time db_query "SELECT * FROM $TABLE_NAME WHERE $TABLE_NAME @@@ 'Canada' LIMIT 10" > /dev/null) 2>&1 )
  search_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  # Record times to CSV
  echo "$SIZE,$index_time,$search_time" >> $OUTPUT_CSV

  # Print query plan
  echo "-- Printing query plan..."
  db_query "EXPLAIN SELECT * FROM $TABLE_NAME WHERE $TABLE_NAME @@@ 'Canada' LIMIT 10"

  # Cleanup: drop temporary table and index
  echo "-- Cleaning up..."
  db_query "DROP TABLE $TABLE_NAME;"
  db_query "DROP INDEX IF EXISTS $INDEX_NAME;"
  echo "Done!"
done

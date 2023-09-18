#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Ensure the "out" directory exists
mkdir -p out

# shellcheck disable=SC1091
source "get_data.sh"

PORT=5431

# 1. Start a postgres docker container
echo "Spinning up postgres server..."
docker pull postgres:15.4
docker run \
  -d \
  --name postgres \
  -e POSTGRES_USER=myuser \
  -e POSTGRES_PASSWORD=mypassword \
  -e POSTGRES_DB=mydatabase \
  -p $PORT:5432 \
  postgres:15.4

# Wait for docker container to spin up
echo "Waiting for server to spin up..."
sleep 5

# 2. Load data into database
echo "Loading data into database..."
WIKI_ARTICLES_FILE=wiki-articles.json
load_data localhost "$PORT" mydatabase myuser mypassword "$WIKI_ARTICLES_FILE"

# Output file for recording times
OUTPUT_CSV=out/benchmark_tsquery.csv
echo "Table Size,Index Time,Search Time" > $OUTPUT_CSV

# Table sizes to be processed
TABLE_SIZES=(10000 50000 100000 200000 300000 400000 500000 600000 700000 800000 900000 1000000)

for SIZE in "${TABLE_SIZES[@]}"; do
  TABLE_NAME="wikipedia_articles_$SIZE"

  # Create temporary table with limit
  db_query localhost "$PORT" mydatabase myuser mypassword "CREATE TABLE $TABLE_NAME AS SELECT * FROM wikipedia_articles LIMIT $SIZE;"

  # Add the search_vector column to the temporary table
  db_query localhost "$PORT" mydatabase myuser mypassword "ALTER TABLE $TABLE_NAME ADD COLUMN search_vector tsvector;"

  # Time indexing
  start_time=$( (time db_query localhost "$PORT" mydatabase myuser mypassword "UPDATE $TABLE_NAME SET search_vector = to_tsvector('english', title) || to_tsvector('english', body);" > /dev/null) 2>&1 )
  index_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  # Time search
  start_time=$( (time db_query localhost "$PORT" mydatabase myuser mypassword "SELECT title, body, ts_rank_cd(search_vector, query) as rank FROM $TABLE_NAME, to_tsquery('canada') query WHERE query @@ search_vector ORDER BY rank DESC LIMIT 10;" > /dev/null) 2>&1 )
  search_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  # Record times to CSV
  echo "$SIZE,$index_time,$search_time" >> $OUTPUT_CSV

  # Cleanup: drop temporary table
  db_query localhost "$PORT" mydatabase myuser mypassword "DROP TABLE $TABLE_NAME;"
done

# 5. Destroy
echo "Destroying container..."
docker kill postgres
docker rm postgres

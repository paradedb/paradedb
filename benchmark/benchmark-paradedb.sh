#!/bin/bash

source "get_data.sh"

PORT=5431

# 1. Install and run docker container for paradedb in detached mode
echo "Spinning up paradedb server..."
docker run \
    -d \
    --name paradedb \
    -e POSTGRES_USER=myuser \
    -e POSTGRES_PASSWORD=mypassword \
    -e POSTGRES_DB=mydatabase \
    -p $PORT:5432 \
    docker-paradedb

# Wait for docker container to spin up
echo "Waiting for server to spin up..."
sleep 5

# 2. Load data into database mydatabase via load_data.sql
echo "Loading data into database..."
WIKI_ARTICLES_FILE=wiki-articles.json
load_data localhost $PORT mydatabase myuser mypassword $WIKI_ARTICLES_FILE

# Output file for recording times
OUTPUT_CSV=out/benchmark_paradedb.csv
echo "Table Size,Index Time,Search Time" > $OUTPUT_CSV

# Table sizes to be processed
TABLE_SIZES=(10000 50000 100000 200000 300000 400000 500000 600000 700000 800000 900000 1000000)

for SIZE in "${TABLE_SIZES[@]}"; do
    TABLE_NAME="wikipedia_articles_$SIZE"
    INDEX_NAME="search_index_$SIZE"

    # Create temporary table with limit
    db_query localhost $PORT mydatabase myuser mypassword "CREATE TABLE $TABLE_NAME AS SELECT * FROM wikipedia_articles LIMIT $SIZE;"

    # Time indexing
    echo "Time indexing for table size $SIZE..."
    start_time=$( (time db_query localhost $PORT mydatabase myuser mypassword "CREATE INDEX $INDEX_NAME ON $TABLE_NAME USING bm25 (($TABLE_NAME.*));" > /dev/null) 2>&1 )
    index_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

    # Time search
    echo "Time search query for table size $SIZE..."
    start_time=$( (time db_query localhost $PORT mydatabase myuser mypassword "SELECT * FROM $TABLE_NAME WHERE $TABLE_NAME @@@ 'Canada' LIMIT 10" > /dev/null) 2>&1 )
    search_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

    # Record times to CSV
    echo "$SIZE,$index_time,$search_time" >> $OUTPUT_CSV

    # Cleanup: drop temporary table and index
    db_query localhost $PORT mydatabase myuser mypassword "DROP TABLE $TABLE_NAME;"
    db_query localhost $PORT mydatabase myuser mypassword "DROP INDEX IF EXISTS $INDEX_NAME;"
done

# 5. Destroy db
echo "Destroying container..."
# docker kill paradedb
# docker rm paradedb

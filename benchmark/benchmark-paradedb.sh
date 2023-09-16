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
 # paradedb/paradedb:latest 

# Wait for docker container to spin up
echo "Waiting for server to spin up..."
sleep 5

# 2. Load data into database mydatabase via load_data.sql
echo "Loading data into database..."
WIKI_ARTICLES_FILE=wiki-articles.json
load_data localhost $PORT mydatabase myuser mypassword $WIKI_ARTICLES_FILE

TABLE_NAME=wikipedia_articles
INDEX_NAME=search_index

# 3. Run and time indexing
# CREATE INDEX search_index ON wikipedia_articles USING bm25 ((wikipedia_articles.*));
echo "Time indexing..."
db_query localhost $PORT mydatabase myuser mypassword "DROP INDEX IF EXISTS $INDEX_NAME;"
time db_query localhost $PORT mydatabase myuser mypassword "CREATE INDEX $INDEX_NAME ON $TABLE_NAME USING bm25 (($TABLE_NAME.*));"

# 4. Run and time search - TODO: rank
# SELECT * FROM wikipedia_articles WHERE wikipedia_articles @@@ 'america' LIMIT 10
echo "Time search query..."
time db_query localhost $PORT mydatabase myuser mypassword "SELECT * FROM $TABLE_NAME WHERE $TABLE_NAME @@@ 'america' LIMIT 10" >> search_output_paradedb.txt;

# 5. Destroy db
echo "Destroying container..."
# docker kill paradedb
# docker rm paradedb
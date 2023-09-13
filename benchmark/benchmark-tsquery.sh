#!/bin/bash

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
WIKI_ARTICLES_FILE=wiki-articles-1000.json
load_data localhost $PORT mydatabase myuser mypassword $WIKI_ARTICLES_FILE

TABLE_NAME=wikipedia_articles

# 3. Run and time indexing
# UPDATE wikipedia_articles 
# SET search_vector = to_tsvector('english', title) || to_tsvector('english', body);
db_query localhost $PORT mydatabase myuser mypassword "ALTER TABLE $TABLE_NAME ADD COLUMN search_vector tsvector;"
time db_query localhost $PORT mydatabase myuser mypassword "UPDATE $TABLE_NAME SET search_vector = to_tsvector('english', title) || to_tsvector('english', body);"

# 4. Run and time search
echo "Time search query..."
# SELECT title, body, ts_rank_cd(search_vector, query) as rank
# FROM wikipedia_articles, to_tsquery('america') query
# WHERE query @@ textsearch
# ORDER BY rank DESC
# LIMIT 10;
time db_query localhost $PORT mydatabase myuser mypassword "SELECT title, body, ts_rank_cd(search_vector, query) as rank FROM $TABLE_NAME, to_tsquery('america') query WHERE query @@ search_vector ORDER BY rank DESC LIMIT 10;" >> search_output_tsquery.txt

# 5. Destroy
echo "Destroying container..."
docker kill postgres
docker rm postgres
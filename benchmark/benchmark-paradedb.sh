#!/bin/bash

# 1. Install and run docker container for paradedb:
docker run \
 -e POSTGRES_USER=suriya \
 -e POSTGRES_PASSWORD=password \
 -e POSTGRES_DB=mydatabase \
 -p 5432:5432 \
 paradedb/paradedb:latest

# 2. Load data into database mydatabase via load_data.sql

# 3. Run and time indexing
# SELECT index_bm25('wikipedia_articles', 'bench_index', '{title, body}');

# 4. Run and time search
# SELECT search_bm25('america', 'bench_index', 10);

# 5. Destroy db?
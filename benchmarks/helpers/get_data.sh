#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

HOST=localhost
PORT=5431
DATABASE=mydatabase
USER=myuser
PASSWORD=mypassword
WIKI_ARTICLES_FILE=wiki-articles.json

# Helper function to run a query on the benchmarking database
db_query () {
  local QUERY=$1
  PGPASSWORD="$PASSWORD" psql -h "$HOST" -p "$PORT" -d "$DATABASE" -U "$USER" -c "$QUERY"
}

# Helper function to download the benchmarking dataset
download_data () {
  if [ ! -f "$WIKI_ARTICLES_FILE" ]; then
    if wget https://www.dropbox.com/s/wwnfnu441w1ec9p/$WIKI_ARTICLES_FILE.bz2 -O $WIKI_ARTICLES_FILE.bz2; then
      echo "-- Unzipping $WIKI_ARTICLES_FILE..."
      bunzip2 $WIKI_ARTICLES_FILE.bz2
    else
      echo "-- Failed to download benchmarking dataset, exiting."
      exit 1
    fi
    echo "-- Done!"
  else
    echo "-- Dataset $WIKI_ARTICLES_FILE found, skipping download."
  fi
}

# This function loads the benchmarking dataset into the benchmarking database, for SQL-based benchmarks
load_data () {
  # First, download the dataset
  download_data
  
  # In order to pull entries from your local files, you have to use the combo of cat and COPY FROM STDIN with the -c option
  echo "-- Creating table for JSON entries and loading entries from file into table (this may take a few minutes)..."
  db_query "DROP TABLE IF EXISTS temp_json;"
  db_query "CREATE TABLE temp_json ( j JSONB );"
  db_query "COPY temp_json FROM STDIN CSV QUOTE E'\x01' DELIMITER E'\x02';" < "$WIKI_ARTICLES_FILE"

  echo "-- Loading JSON data into the wikipedia_articles table..."
  PGPASSWORD=$PASSWORD psql -h "$HOST" -p "$PORT" -d "$DATABASE" -U "$USER" -f helpers/load_data.sql
}

export -f load_data
export -f db_query

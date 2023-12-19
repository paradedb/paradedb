#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Set default values
DEFAULT_HOST=localhost
DEFAULT_PORT=5431
DEFAULT_DATABASE=mydatabase
DEFAULT_USER=myuser
DEFAULT_PASSWORD=mypassword

# Use environment variables if they are set, otherwise use defaults
HOST=${HOST:-$DEFAULT_HOST}
PORT=${PORT:-$DEFAULT_PORT}
DATABASE=${DATABASE:-$DEFAULT_DATABASE}
USER=${USER:-$DEFAULT_USER}
PASSWORD=${PASSWORD:-$DEFAULT_PASSWORD}
WIKI_ARTICLES_FILE=wiki-articles.json

# Helper function to run a query on the benchmarking database
db_query () {
  local QUERY=$1
  if $USING_PGRX; then
    psql -h "$HOST" -p "$PORT" -d "$DATABASE" -c "$QUERY"
  else
    PGPASSWORD="$PASSWORD" psql -h "$HOST" -p "$PORT" -d "$DATABASE" -U "$USER" -c "$QUERY"
  fi
}

# Helper function to download the benchmarking dataset
download_data () {
  if [ ! -f "$WIKI_ARTICLES_FILE" ]; then
    # We maintain our own copy of the dataset on S3 to avoid Dropbox rate limits
    if wget -nv https://paradedb-benchmarks.s3.amazonaws.com/wiki-articles.json.bz2 -O $WIKI_ARTICLES_FILE.bz2; then
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
  # db_query "COPY temp_json FROM STDIN CSV QUOTE E'\x01' DELIMITER E'\x02';" < "$WIKI_ARTICLES_FILE"


  head -n 100000 "$WIKI_ARTICLES_FILE" | db_query "COPY temp_json FROM STDIN CSV QUOTE E'\x01' DELIMITER E'\x02';"


  echo "-- Loading JSON data into the wikipedia_articles table..."
  if $USING_PGRX; then
    # When using pgrx, we only load 100,000 rows
    # sed "s/{{LIMIT}}/100000/g" load_data.sql > load_data.sql
    psql -h "$HOST" -p "$PORT" -d "$DATABASE" -f helpers/load_data.sql
  else
    # When using Docker, we load the full dataset of 5M rows
    # sed "s/{{LIMIT}}/5000000/g" load_data.sql > load_data.sql
    PGPASSWORD="$PASSWORD" psql -h "$HOST" -p "$PORT" -d "$DATABASE" -U "$USER" -f helpers/load_data.sql
  fi
}

export -f load_data
export -f db_query

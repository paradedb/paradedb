#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Set default values
DEFAULT_PG_HOST=localhost
DEFAULT_PG_PORT=5431
DEFAULT_PG_DATABASE=mydatabase
DEFAULT_PG_USER=myuser
DEFAULT_PG_PASSWORD=mypassword
DEFAULT_USING_PGRX=false

# Use environment variables if they are set, otherwise use defaults
PG_HOST=${PG_HOST:-$DEFAULT_PG_HOST}
PG_PORT=${PG_PORT:-$DEFAULT_PG_PORT}
PG_DATABASE=${PG_DATABASE:-$DEFAULT_PG_DATABASE}
PG_USER=${PG_USER:-$DEFAULT_PG_USER}
PG_PASSWORD=${PG_PASSWORD:-$DEFAULT_PG_PASSWORD}
USING_PGRX=${USING_PGRX:-$DEFAULT_USING_PGRX}
WIKI_ARTICLES_FILE=wiki-articles.json

# Helper function to run a query on the benchmarking database
db_query () {
  local QUERY=$1
  if $USING_PGRX; then
    echo "using pgrx"
    psql -h "$PG_HOST" -p "$PG_PORT" -d "$PG_DATABASE" -c "$QUERY"
  else
    echo "using docker"
    echo "$QUERY"
    echo "$PG_HOST"
    echo "$PG_PORT"
    echo "$PG_DATABASE"
    echo "$PG_USER"
    echo "$PG_PASSWORD"
    PGPASSWORD="$PG_PASSWORD" psql -h "$PG_HOST" -p "$PG_PORT" -d "$PG_DATABASE" -U "$PG_USER" -c "$QUERY"
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
    psql -h "$PG_HOST" -p "$PG_PORT" -d "$PG_DATABASE" -f helpers/load_data.sql
  else
    # When using Docker, we load the full dataset of 5M rows
    # sed "s/{{LIMIT}}/5000000/g" load_data.sql > load_data.sql
    PGPASSWORD="$PG_PASSWORD" psql -h "$PG_HOST" -p "$PG_PORT" -d "$PG_DATABASE" -U "$PG_USER" -f helpers/load_data.sql
  fi
}

export -f load_data
export -f db_query

#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# TODO: simplify these functions down further (they're still repetitive)
db_query () {
  HOST=$1
  PORT=$2
  DATABASE=$3
  USER=$4
  PASSWORD=$5
  QUERY=$6
  PGPASSWORD="$PASSWORD" psql -h "$HOST" -p "$PORT" -d "$DATABASE" -U "$USER" -c "$QUERY"
}

db_file () {
  HOST=$1
  PORT=$2
  DATABASE=$3
  USER=$4
  PASSWORD=$5
  FILE=$6
  PGPASSWORD=$PASSWORD psql -h "$HOST" -p "$PORT" -d "$DATABASE" -U "$USER" -f "$FILE"
}

load_data () {
  HOST=localhost
  PORT=5431
  DATABASE=mydatabase
  USER=myuser
  PASSWORD=mypassword
  WIKI_ARTICLES_FILE=wiki-articles.json

  if [ ! -f "$WIKI_ARTICLES_FILE" ]; then
    echo "-- Downloading wiki-articles.json..."
    wget https://www.dropbox.com/s/wwnfnu441w1ec9p/$WIKI_ARTICLES_FILE.bz2 -O $WIKI_ARTICLES_FILE.bz2

    if [ $? -eq 0 ]; then
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

  # In order to pull entries from your local files, you have to use the combo of cat and COPY FROM STDIN with the -c option
  echo "-- Creating table for JSON entries and loading entries from file into table (this may take a few minutes)..."
  db_query "$HOST" "$PORT" "$DATABASE" "$USER" "$PASSWORD" "DROP TABLE IF EXISTS temp_json;"
  db_query "$HOST" "$PORT" "$DATABASE" "$USER" "$PASSWORD" "CREATE TABLE temp_json ( j JSONB );"
  db_query "$HOST" "$PORT" "$DATABASE" "$USER" "$PASSWORD" "COPY temp_json FROM STDIN CSV QUOTE E'\x01' DELIMITER E'\x02';" < "$WIKI_ARTICLES_FILE"

  echo "-- Loading JSON data into the wikipedia_articles table..."
  db_file "$HOST" "$PORT" "$DATABASE" "$USER" "$PASSWORD" helpers/load_data.sql
}

export -f load_data
export -f db_query

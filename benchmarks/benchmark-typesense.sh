#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Ensure the "out" directory exists
mkdir -p out

# shellcheck disable=SC1091
source "helpers/get_data.sh"

PORT=8108
TS_VERSION=0.25.1
WIKI_ARTICLES_FILE=wiki-articles.json
TYPESENSE_API_KEY=xyz
TYPESENSE_DATA=$(pwd)/typesense-data
TYPESENSE_BULK_OUTPUT=out/typesense_bulk_output
OUTPUT_CSV=out/benchmark_typesense.csv

# Cleanup function to stop and remove the Docker container
cleanup() {
  echo ""
  echo "Cleaning up benchmark environment..."
  if docker ps -q --filter "name=typesense" | grep -q .; then
    docker kill typesense
  fi
  docker rm typesense
  echo "Done!"
}

# Register the cleanup function to run when the script exits
trap cleanup EXIT

echo ""
echo "*******************************************************"
echo "* Benchmarking Typesense version: $TS_VERSION"
echo "*******************************************************"
echo ""

# Download and run docker container for Typesense
echo "Creating Typesense $TS_VERSION node..."
docker run \
  -d \
  --name typesense \
  -p $PORT:8108 \
  -v"$TYPESENSE_DATA:/data" "typesense/typesense:$TS_VERSION" \
  --data-dir /data \
  --api-key=$TYPESENSE_API_KEY \
  --enable-cors

# Wait for Docker container to spin up
echo ""
echo "Waiting for server to spin up..."
sleep 30
echo "Done!"

# Retrieve the benchmarking dataset
echo ""
echo "Retrieving dataset..."
download_data
echo "Done!"

# Output file for recording times
echo "Table Size,Index Time,Search Time" > $OUTPUT_CSV

# Table sizes to be processed (in number of rows). The maximum is 5M rows with the Wikipedia dataset
TABLE_SIZES=(10000 50000 100000 200000 300000 400000 500000 600000 700000 800000 900000 1000000 1500000 2000000 2500000 3000000 3500000 4000000 45000000 5000000)

for SIZE in "${TABLE_SIZES[@]}"; do
  echo ""
  echo "Running benchmarking suite on index with $SIZE documents..."

  # Create Typesense collection (only if it doesn't exist)
  COLLECTION_EXISTS=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:$PORT/collections/wikipedia_articles")
  if [ "$COLLECTION_EXISTS" -ne "200" ]; then
    echo "-- Creating Typesense collection..."
    curl "http://localhost:$PORT/collections" \
      -X POST \
      -H "Content-Type: application/json" \
      -H "X-TYPESENSE-API-KEY: ${TYPESENSE_API_KEY}" -d '{
        "name": "wikipedia_articles",
        "fields": [
          {"name": "title", "type": "string"},
          {"name": "body", "type": "string"},
          {"name": "url", "type": "string"}
        ]
    }'
  fi

  # Prepare data to be indexed by Typesense
  echo ""
  echo "-- Preparing data to be consumed by Typesense..."
  CURL_BULK_SIZE=250000
  mkdir "$TYPESENSE_BULK_OUTPUT"
  TOTAL_BULK_FILENAME="$TYPESENSE_BULK_OUTPUT/${SIZE}_ts.json"
  BULK_UPLOAD_PREFIX="upload_"
  head -n "$SIZE" "$WIKI_ARTICLES_FILE" > "$TOTAL_BULK_FILENAME"
  split -l "$CURL_BULK_SIZE" "$TOTAL_BULK_FILENAME" "$TYPESENSE_BULK_OUTPUT/$BULK_UPLOAD_PREFIX"

  # Time indexing using bulk import
  echo "-- Loading data of size $SIZE into wikipedia_articles index..."
  echo "-- Timing indexing..."
  start_time=$( (time find "$TYPESENSE_BULK_OUTPUT" -type f -name "$BULK_UPLOAD_PREFIX*" | while IFS= read -r data_filename; do
        curl "http://localhost:$PORT/collections/wikipedia_articles/documents/import?batch_size=500" -X POST -H "X-TYPESENSE-API-KEY: ${TYPESENSE_API_KEY}" --data-binary @"$data_filename"
  done) 2>&1 )
  index_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  # Time search
  echo "-- Timing search..."
  start_time=$( (time curl -H "X-TYPESENSE-API-KEY: ${TYPESENSE_API_KEY}" "http://localhost:$PORT/collections/wikipedia_articles/documents/search?q=Canada&query_by=title,body" > /dev/null) 2>&1 )
  search_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  # Confirm document count
  doc_count=$(curl --silent -H "X-TYPESENSE-API-KEY: ${TYPESENSE_API_KEY}" -X GET "http://localhost:$PORT/collections/wikipedia_articles" | jq '.num_documents')
  echo "-- Number of documents in wikipedia_articles index for size $SIZE: $doc_count"

  # Record times to CSV
  echo "$SIZE,$index_time,$search_time" >> $OUTPUT_CSV

  # Cleanup: delete the temporary data files
  echo "-- Cleaning up..."
  curl --silent -H "X-TYPESENSE-API-KEY: ${TYPESENSE_API_KEY}" -X DELETE "http://localhost:$PORT/collections/wikipedia_articles"
  rm -rf "$TYPESENSE_BULK_OUTPUT"
  echo ""
  echo "Done!"
done

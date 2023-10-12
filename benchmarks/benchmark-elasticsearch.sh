#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Ensure the "out" directory exists
mkdir -p out

# shellcheck disable=SC1091
source "helpers/get_data.sh"

PORT=9200
ES_VERSION=8.9.2
WIKI_ARTICLES_FILE=wiki-articles.json
ELASTIC_BULK_FOLDER=out/elastic_bulk_output
OUTPUT_CSV=out/benchmark_elasticsearch.csv

# Cleanup function to stop and remove the Docker container
cleanup() {
  echo ""
  echo "Cleaning up benchmark environment..."
  if docker ps -q --filter "name=es01" | grep -q .; then
    docker kill es01
  fi
  docker rm es01
  docker network rm elastic
  echo "Done!"
}

# Register the cleanup function to run when the script exits
trap cleanup EXIT

echo ""
echo "*******************************************************"
echo "* Benchmarking ElasticSearch version: $ES_VERSION"
echo "*******************************************************"
echo ""

# Download and run docker container for ElasticSearch
echo "Creating ElasticSearch $ES_VERSION node..."
docker network create elastic
docker run \
  -d \
  --name es01 \
  --net elastic \
  -p $PORT:9200 \
  -it \
  docker.elastic.co/elasticsearch/elasticsearch:$ES_VERSION

# Wait for Docker container to spin up
echo ""
echo "Waiting for server to spin up..."
sleep 40
echo "Done!"

# Retrieve the benchmarking dataset
echo ""
echo "Retrieving dataset..."
download_data
echo "Done!"

# Produce and save password
echo ""
echo "Producing and saving new ElasticSearch password..."
ELASTIC_PASSWORD=$(docker exec es01 /usr/share/elasticsearch/bin/elasticsearch-reset-password --batch -u elastic | grep "New value:" | awk '{print $3}')
docker cp es01:/usr/share/elasticsearch/config/certs/http_ca.crt .
echo "Done!"

# Output file for recording times
echo "Table Size,Index Time,Search Time" > $OUTPUT_CSV

# Table sizes to be processed (in number of rows). The maximum is 5M rows with the Wikipedia dataset
TABLE_SIZES=(10000 50000 100000 200000 300000 400000 500000 600000 700000 800000 900000 1000000 1500000 2000000 2500000 3000000 3500000 4000000 4500000 5000000)

for SIZE in "${TABLE_SIZES[@]}"; do
  echo ""
  echo "Running benchmarking suite on index with $SIZE documents..."

  # Convert data to be consumed by ElasticSearch
  echo "-- Converting data to bulk format consumable by ElasticSearch..."
  python3 helpers/elastify-data.py $WIKI_ARTICLES_FILE $ELASTIC_BULK_FOLDER "$SIZE"

  # Time indexing
  echo "-- Loading data of size $SIZE into wikipedia_articles index..."
  echo "-- Timing indexing..."
  start_time=$( (time find "$ELASTIC_BULK_FOLDER" -type f -name "${SIZE}_*.json" | while IFS= read -r data_filename; do
        curl --cacert http_ca.crt -u elastic:"$ELASTIC_PASSWORD" -X POST -H "Content-Type:application/json" "https://localhost:$PORT/wikipedia_articles/_bulk" --data-binary @"$data_filename"
  done) 2>&1 )
  index_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')
  curl --cacert http_ca.crt -u elastic:"$ELASTIC_PASSWORD" -X POST "https://localhost:$PORT/wikipedia_articles/_refresh"

  # Time search
  echo "-- Timing search..."
  start_time=$( (time curl --cacert http_ca.crt -u elastic:"$ELASTIC_PASSWORD" -X GET "https://localhost:$PORT/wikipedia_articles/_search?pretty" -H 'Content-Type: application/json' -d'
      {
        "query": {
          "query_string": {
            "query": "Canada"
          }
        }
  }' > /dev/null) 2>&1 )
  search_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  # Confirm document count
  doc_count=$(curl --silent --cacert http_ca.crt -u elastic:"$ELASTIC_PASSWORD" "https://localhost:$PORT/_cat/count/wikipedia_articles?format=json" | jq '.[0].count')
  echo ""
  echo "-- Number of documents in wikipedia_articles index for size $SIZE: $doc_count"

  # Record times to CSV
  echo "$SIZE,$index_time,$search_time" >> $OUTPUT_CSV

  # Cleanup: delete the index
  echo "-- Cleaning up..."
  curl --cacert http_ca.crt -u elastic:"$ELASTIC_PASSWORD" -X DELETE https://localhost:$PORT/wikipedia_articles
  echo ""
  echo "Done!"
done

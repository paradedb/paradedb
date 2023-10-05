#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Ensure the "out" directory exists
mkdir -p out

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
  docker docker rm elastic
  echo "Done!"
}

# Register the cleanup function to run when the script exits
trap cleanup EXIT

echo ""
echo "*******************************************************"
echo "Benchmarking ElasticSearch version: $ES_VERSION"
echo "*******************************************************"
echo ""

# 1. Download and run docker container for ElasticSearch
echo "Creating ElasticSearch node..."
docker network create elastic
docker run \
  -d \
  --name es01 \
  --net elastic \
  -p $PORT:9200 \
  -it \
  docker.elastic.co/elasticsearch/elasticsearch:$ES_VERSION

# Wait for Docker container to spin up
echo "Waiting for server to spin up..."
sleep 15

# Produce password and save
docker exec es01 /usr/share/elasticsearch/bin/elasticsearch-reset-password --batch -u elastic
read -r -p "Copy elastic password here: " ELASTIC_PASSWORD
docker cp es01:/usr/share/elasticsearch/config/certs/http_ca.crt .

# Output file for recording times
echo "Table Size,Index Time,Search Time" > $OUTPUT_CSV

# Table sizes to be processed (in number of rows). You can modify this to go up to 5 million rows with the Wikipedia dataset.
TABLE_SIZES=(10000 50000 100000 200000 300000 400000 500000 600000 700000 800000 900000 1000000)

for SIZE in "${TABLE_SIZES[@]}"; do
  # 2. Convert data to be consumed by Elasticsearch
  echo "Converting data to bulk format consumable by Elasticsearch..."
  # TODO: Adjust the elastify-data.py script to output data for the specific SIZE into a folder
  python3 helpers/elastify-data.py $WIKI_ARTICLES_FILE $ELASTIC_BULK_FOLDER "$SIZE"

  # 3. Clear the old index
  # 4. Load data into ElasticSearch node
  curl --cacert http_ca.crt -u elastic:"$ELASTIC_PASSWORD" -X DELETE https://localhost:$PORT/wikipedia_articles

  echo "Loading data of size $SIZE into wikipedia_articles index..."
  start_time=$( (time find "$ELASTIC_BULK_FOLDER" -type f -name "${SIZE}_*.json" | while IFS= read -r data_filename; do
        curl --cacert http_ca.crt -u elastic:"$ELASTIC_PASSWORD" -X POST -H "Content-Type:application/json" "https://localhost:$PORT/wikipedia_articles/_bulk" --data-binary @"$data_filename"
  done) 2>&1 )

  index_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  curl --cacert http_ca.crt -u elastic:"$ELASTIC_PASSWORD" -X POST "https://localhost:$PORT/wikipedia_articles/_refresh"

  # 4. Run and time search
  start_time=$( (time curl --cacert http_ca.crt -u elastic:"$ELASTIC_PASSWORD" -X GET "https://localhost:$PORT/wikipedia_articles/_search?pretty" -H 'Content-Type: application/json' -d'
      {
        "query": {
          "query_string": {
            "query": "Canada"
          }
        }
  }' > /dev/null) 2>&1 )
  search_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  doc_count=$(curl --silent --cacert http_ca.crt -u elastic:"$ELASTIC_PASSWORD" "https://localhost:$PORT/_cat/count/wikipedia_articles?format=json" | jq '.[0].count')
  echo "Number of documents in wikipedia_articles index for size $SIZE: $doc_count"

  # Record times to CSV
  echo "$SIZE,$index_time,$search_time" >> $OUTPUT_CSV
done

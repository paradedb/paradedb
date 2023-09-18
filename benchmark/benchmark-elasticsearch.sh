#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Prepare
OUTPUT_CSV=out/benchmark_elasticsearch.csv
echo "Table Size,Index Time,Search Time" > $OUTPUT_CSV
TABLE_SIZES=(10000 50000 100000 200000 300000 400000 500000 600000 700000 800000 900000 1000000)

# 1. Download and run docker container for Elasticsearch
echo "Creating Elasticsearch node..."
docker network create elastic
docker pull docker.elastic.co/elasticsearch/elasticsearch:8.9.2
docker run \
  -d \
  --name es01 \
  --net elastic \
  -p 9200:9200 \
  -it \
  docker.elastic.co/elasticsearch/elasticsearch:8.9.2

# Wait for docker container to spin up
echo "Waiting for server to spin up..."
sleep 15

# Produce password and save
docker exec es01 /usr/share/elasticsearch/bin/elasticsearch-reset-password --batch -u elastic
read -r -p "Copy elastic password here: " ELASTIC_PASSWORD
docker cp es01:/usr/share/elasticsearch/config/certs/http_ca.crt .

# 2. Convert data to be consumed by Elasticsearch
echo "Converting data to bulk format consumable by Elasticsearch..."
WIKI_ARTICLES_FILE=wiki-articles.json
ELASTIC_BULK_FOLDER=out/elastic_bulk_output

for SIZE in "${TABLE_SIZES[@]}"; do
  # TODO: Adjust the elastify-data.py script to output data for the specific SIZE into a folder
  python3 elastify-data.py $WIKI_ARTICLES_FILE $ELASTIC_BULK_FOLDER "$SIZE"

  # 3. Clear the old index
  # 4. Load data into Elasticsearch node
  curl --cacert http_ca.crt -u elastic:"$ELASTIC_PASSWORD" -X DELETE https://localhost:9200/wikipedia_articles

  echo "Loading data of size $SIZE into wikipedia_articles index..."
  start_time=$( (time find "$ELASTIC_BULK_FOLDER" -type f -name "${SIZE}_*.json" | while IFS= read -r data_filename; do
        curl --cacert http_ca.crt -u elastic:"$ELASTIC_PASSWORD" -X POST -H "Content-Type:application/json" "https://localhost:9200/wikipedia_articles/_bulk" --data-binary @"$data_filename"
  done) 2>&1 )

  index_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  curl --cacert http_ca.crt -u elastic:"$ELASTIC_PASSWORD" -X POST "https://localhost:9200/wikipedia_articles/_refresh"

  # 4. Run and time search
  start_time=$( (time curl --cacert http_ca.crt -u elastic:"$ELASTIC_PASSWORD" -X GET "https://localhost:9200/wikipedia_articles/_search?pretty" -H 'Content-Type: application/json' -d'
      {
        "query": {
          "query_string": {
            "query": "Canada"
          }
        }
  }' > /dev/null) 2>&1 )
  search_time=$(echo "$start_time" | grep real | awk '{ split($2, array, "m|s"); print array[1]*60000 + array[2]*1000 }')

  doc_count=$(curl --silent --cacert http_ca.crt -u elastic:"$ELASTIC_PASSWORD" "https://localhost:9200/_cat/count/wikipedia_articles?format=json" | jq '.[0].count')
  echo "Number of documents in wikipedia_articles index for size $SIZE: $doc_count"

  # Record times to CSV
  echo "$SIZE,$index_time,$search_time" >> $OUTPUT_CSV
done

# 5. Destroy
echo "Destroying container..."
docker kill es01
docker rm es01
docker network rm elastic

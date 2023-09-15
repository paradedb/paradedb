#!/bin/bash

# 1. Download and run docker container for Elasticsearch
# Follow instructions for using the Elasticsearch docker container:
# https://www.elastic.co/guide/en/elasticsearch/reference/current/docker.html
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
read -p "Copy elastic password here: " ELASTIC_PASSWORD
docker cp es01:/usr/share/elasticsearch/config/certs/http_ca.crt .

# 2. Convert data to be consumed by Elasticsearch
echo "Converting data to bulk format consumable by Elasticsearch..."
WIKI_ARTICLES_FILE=wiki-articles.json
ELASTIC_BULK_FOLDER=elastic_bulk_output
# python3 elastify-data.py $WIKI_ARTICLES_FILE $ELASTIC_BULK_FOLDER

# 3. Load data into Elasticsearch node
echo "Creating wikipedia_articles index..."
curl --cacert http_ca.crt -u elastic:$ELASTIC_PASSWORD -X PUT https://localhost:9200/wikipedia_articles

echo "Loading data into wikipedia_articles index..."
INDEX_START_TIME=$SECONDS
for data_filename in $ELASTIC_BULK_FOLDER/*; do curl --cacert http_ca.crt -u elastic:$ELASTIC_PASSWORD -X POST -H "Content-Type:application/json" "https://localhost:9200/wikipedia_articles/_bulk" --data-binary @$data_filename; done > bulk_load_elasticsearch.txt
INDEXING_TIME=$(($SECONDS - $INDEX_START_TIME))
echo "Indexing time: $((INDEXING_TIME / 60)):$((INDEXING_TIME % 60))"

# 4. Run and time search (TODO: does this already only return the top 10?)
echo "Time search query..."
time curl --cacert http_ca.crt -u elastic:$ELASTIC_PASSWORD -X GET "https://localhost:9200/wikipedia_articles/_search?pretty" -H 'Content-Type: application/json' -d' 
{ "query": { 
	"query_string": {
      "query": "america"
    } 
} }
' > search_output_elasticsearch.txt

# 4. Destroy 
echo "Destroying container..."
docker kill es01
docker rm es01
docker network rm elastic

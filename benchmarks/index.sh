#!/bin/bash

set -Eeuo pipefail

if [ $# -ne 1 ]; then
  echo "Usage: $0 <postgres_url>"
  exit 1
fi

POSTGRES_URL=$1
OUTPUT_FILE="index.md"

echo "| Duration (min) | Index Size (MB) | Table Size (MB) | Estimated Rows |" > "$OUTPUT_FILE"
echo "|----------------|-----------------|-----------------|----------------|" >> "$OUTPUT_FILE"

output_file=$(mktemp)

psql "$POSTGRES_URL" -t -c "CREATE EXTENSION IF NOT EXISTS pg_search;" || { echo "Failed to create pg_search extension"; exit 1; }
psql "$POSTGRES_URL" -t -c "DROP INDEX IF EXISTS benchmark_logs_idx;" || { echo "Failed to drop index"; exit 1; }
psql "$POSTGRES_URL" -t -c '\timing' -c "CREATE INDEX benchmark_logs_idx ON benchmark_logs USING bm25 (id, message, country, severity, timestamp, metadata) WITH (key_field = 'id', text_fields = '{\"country\": {\"fast\": true }}', json_fields = '{\"metadata\": { \"fast\": true }}');" > "$output_file" 2>&1 || { echo "Failed to create index"; cat "$output_file"; exit 1; }

duration_min=$(grep 'Time' "$output_file" | awk '{sum+=$2} END {printf "%.3f", sum/60000}')
rm "$output_file"

# Get table size in MB
echo "Calculating table size..."
table_size_mb=$(psql "$POSTGRES_URL" -t -c "
    SELECT pg_relation_size('benchmark_logs') / 1024 / 1024
    FROM pg_class
WHERE relname = 'benchmark_logs';" | tr -d ' ') || { echo "Failed to get table size"; exit 1; }

# Get index size in MB
echo "Calculating index size..."
index_size_mb=$(psql "$POSTGRES_URL" -t -c "
    SELECT pg_relation_size('benchmark_logs_idx') / 1024 / 1024
    FROM pg_class
WHERE relname = 'benchmark_logs_idx';" | tr -d ' ') || { echo "Failed to get index size"; exit 1; }

estimated_rows=$(psql "$POSTGRES_URL" -t -c "
    SELECT reltuples::bigint
    FROM pg_class
WHERE relname = 'benchmark_logs';" | tr -d ' ') || { echo "Failed to get estimated row count"; exit 1; }

echo "| $duration_min | $index_size_mb | $table_size_mb | $estimated_rows |" >> "$OUTPUT_FILE"
echo "Index creation results written to $OUTPUT_FILE"

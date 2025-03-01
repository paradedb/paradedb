#!/bin/bash

set -Eeuo pipefail

# Parse named arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --type)
      TYPE="$2"
      shift 2
      ;;
    --url)
      POSTGRES_URL="$2"
      shift 2
      ;;
    --prewarm)
      PREWARM="$2"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1"
      exit 1
      ;;
  esac
done

PREWARM=${PREWARM:-true}

# Validate arguments
if [ -z "${POSTGRES_URL:-}" ]; then
  echo "Usage: $0 --type <pg_search|tuned_postgres> --postgres-url <postgres_url> [--prewarm <true|false>]"
  exit 1
fi

if [ "$TYPE" != "pg_search" ] && [ "$TYPE" != "tuned_postgres" ]; then
  echo "Type must be either pg_search or tuned_postgres"
  exit 1
fi

OUTPUT_FILE="index_${TYPE}.md"

echo "| Duration (min) | Index Size (MB) |" > "$OUTPUT_FILE"
echo "|----------------|-----------------|" >> "$OUTPUT_FILE"

psql "$POSTGRES_URL" -f "prepare_table/${TYPE}.sql" || { echo "Failed to prepare indexes"; exit 1; }

while IFS='' read -r statement; do
  if [[ -z "${statement// }" ]]; then
    continue
  fi

  echo "$statement"

  duration_ms=$(psql "$POSTGRES_URL" -t -c '\timing' -c "$statement" | grep 'Time' | awk '{print $2}')
  duration_min=$(echo "scale=2; $duration_ms / (1000 * 60)" | bc)
  index_name=$(echo "$statement" | sed -n 's/CREATE INDEX \([^ ]*\).*/\1/p')
  index_size=$(psql "$POSTGRES_URL" -t -c "SELECT pg_relation_size('$index_name') / (1024 * 1024);" | tr -d ' ')

  echo "| $duration_min | $index_size |" >> "$OUTPUT_FILE"
done < "create_index/${TYPE}.sql"

if [ "$PREWARM" = "true" ]; then
  psql "$POSTGRES_URL" -f "prewarm/${TYPE}.sql" || { echo "Failed to prewarm indexes"; exit 1; }
fi

echo "Index creation results written to $OUTPUT_FILE"

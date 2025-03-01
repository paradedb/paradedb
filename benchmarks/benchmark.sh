#!/bin/bash

set -Eeuo pipefail

# Parse named arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --url)
      POSTGRES_URL="$2"
      shift 2
      ;;
    --type)
      TYPE="$2"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1"
      echo "Usage: $0 --url <postgres_url> --type <type>"
      echo "Type must be either 'pg_search' or 'tuned_postgres'"
      exit 1
      ;;
  esac
done

# Check required arguments
if [[ -z "${POSTGRES_URL:-}" || -z "${TYPE:-}" ]]; then
  echo "Usage: $0 --url <postgres_url> --type <type>"
  echo "Type must be either 'pg_search' or 'tuned_postgres'"
  exit 1
fi

if [[ "$TYPE" != "pg_search" && "$TYPE" != "tuned_postgres" ]]; then
  echo "Type must be either 'pg_search' or 'tuned_postgres'"
  exit 1
fi

OUTPUT_FILE="benchmark_${TYPE}.md"

# Create markdown table header
echo "| Query Type | Query | Run 1 (ms) | Run 2 (ms) | Run 3 (ms) | Rows Returned |" > "$OUTPUT_FILE"
echo "|------------|--------|------------|------------|------------|---------------|" >> "$OUTPUT_FILE"

# Iterate through each .sql file in queries directory
for sql_file in queries/"$TYPE"/*.sql; do
  # Extract query type from filename (remove path and .sql extension)
  query_type=$(basename "$sql_file" .sql)

  while IFS='' read -r -d ';' query; do
    # Skip empty queries
    if [[ -z "${query// }" ]]; then
      continue
    fi

    # Run and time each query 3 times
    clean_query=$(echo "$query" | grep -v '^--' | tr '\n' ' ' | sed 's/^ *//;s/ *$//')
    if [[ -z "$clean_query" ]]; then
      continue
    fi
    # Escape pipes for markdown
    md_query=${clean_query//|/\\|}
    results=()
    num_results=0

    printf "Query Type: %s\nQuery: %-80s\n" "$query_type" "$clean_query"

    for i in {1..3}; do
      # Capture both timing and result count
      output_file=$(mktemp)
      # Run query and capture output to file
      psql "$POSTGRES_URL" -t -c '\timing' -c "$clean_query" > "$output_file" 2>&1
      duration_ms=$(grep 'Time' "$output_file" | awk '{print $2}')

      # Count number of rows returned (only on first run)
      if [ "$i" -eq 1 ]; then
        num_results=$(grep -v 'Time' "$output_file" | grep -c -v '^$')
        num_results=$((num_results - 1))
      fi

      rm "$output_file"
      results+=("$duration_ms")
    done

    printf "Run 1: %4.0fms | Run 2: %4.0fms | Run 3: %4.0fms | Results: %d\n\n" "${results[0]}" "${results[1]}" "${results[2]}" "$num_results"

    # Write results to markdown table
    echo "| $query_type | \`$md_query\` | ${results[0]} | ${results[1]} | ${results[2]} | $num_results |" >> "$OUTPUT_FILE"
  done < "$sql_file"
done

echo "Benchmark results written to $OUTPUT_FILE"

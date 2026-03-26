#!/usr/bin/env bash
# Copyright (c) 2023-2026 ParadeDB, Inc.
#
# This file is part of ParadeDB - Postgres for Search and Analytics
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <http://www.gnu.org/licenses/>.

# load.sh — Download StackOverflow CSVs from S3 and COPY them into PostgreSQL.
#
# Usage: bash load.sh <postgres_url>

set -euo pipefail

URL="${1:?Usage: load.sh <postgres_url>}"

S3_PREFIX="s3://paradedb-benchmarks/datasets/stackoverflow/03-09-2026"
DATA_DIR="/tmp/stackoverflow_data"

TABLES=(
  badges
  comments
  post_history
  post_links
  posts_answers
  posts_moderator_nomination
  posts_orphaned_tag_wiki
  posts_privilege_wiki
  posts_questions
  posts_tag_wiki
  posts_tag_wiki_excerpt
  posts_wiki_placeholder
  stackoverflow_posts
  tags
  users
  votes
)

echo "==> Downloading StackOverflow CSVs from S3..."
mkdir -p "$DATA_DIR"
for table in "${TABLES[@]}"; do
  file="${DATA_DIR}/${table}.csv"
  if [ ! -f "$file" ]; then
    echo "    Downloading ${table}.csv ..."
    aws s3 cp "${S3_PREFIX}/${table}.csv" "$file"
  else
    echo "    ${table}.csv already exists, skipping download."
  fi
done

echo "==> Creating tables from CSV headers..."
for table in "${TABLES[@]}"; do
  file="${DATA_DIR}/${table}.csv"
  # Read the CSV header and create columns — all as TEXT to avoid type mismatches.
  header=$(head -n 1 "$file")
  columns=""
  IFS=',' read -ra cols <<<"$header"
  for col in "${cols[@]}"; do
    col=$(echo "$col" | tr -d '\r' | xargs)
    if [ -n "$columns" ]; then
      columns="${columns}, "
    fi
    columns="${columns}${col} TEXT"
  done
  echo "    Creating table ${table} ..."
  psql "$URL" -c "DROP TABLE IF EXISTS ${table} CASCADE;"
  psql "$URL" -c "CREATE TABLE ${table} (${columns});"
done

echo "==> Loading CSV data into PostgreSQL..."
for table in "${TABLES[@]}"; do
  file="${DATA_DIR}/${table}.csv"
  # Read column names from CSV header for explicit column list in COPY.
  header=$(head -n 1 "$file")
  col_list=$(echo "$header" | tr -d '\r')
  echo "    Loading ${table} ..."
  psql "$URL" -c "\\COPY ${table}(${col_list}) FROM '${file}' WITH (FORMAT csv, HEADER true)"
done

echo "==> Running VACUUM ANALYZE on all tables..."
for table in "${TABLES[@]}"; do
  psql "$URL" -c "VACUUM ANALYZE ${table};"
done

echo "==> StackOverflow dataset loaded successfully."

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

# load.sh — Download StackOverflow Parquet data from GCS and load into PostgreSQL.
#
# Usage: bash load.sh <postgres_url> <max_rows_per_table>

set -euo pipefail

URL="${1:?Usage: load.sh <postgres_url> <max_rows_per_table>}"
MAX_ROWS="${2:?Usage: load.sh <postgres_url> <max_rows_per_table>}"

GCS_BUCKET="gs://paradedb-benchmarks/stackoverflow"
DATA_DIR="/tmp/stackoverflow_data"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

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

echo "==> Installing pyarrow..."
pip install --quiet pyarrow

echo "==> Downloading Parquet files from GCS..."
mkdir -p "$DATA_DIR"
for table in "${TABLES[@]}"; do
  table_dir="${DATA_DIR}/${table}"
  if [ -d "$table_dir" ] && ls "$table_dir"/*.parquet &>/dev/null; then
    echo "    ${table}/ already exists, skipping download."
  else
    echo "    Downloading ${table}/ ..."
    mkdir -p "$table_dir"
    gsutil -m cp "${GCS_BUCKET}/${table}/*.parquet" "$table_dir/"
  fi
done

echo "==> Creating tables and loading data (max ${MAX_ROWS} rows per table)..."
for table in "${TABLES[@]}"; do
  table_dir="${DATA_DIR}/${table}"
  echo "    Processing ${table} ..."

  # Generate CREATE TABLE SQL and combined CSV from Parquet files.
  python3 "${SCRIPT_DIR}/load_parquet.py" "$table_dir" "$table" "$MAX_ROWS"

  # Create the table.
  psql "$URL" -f "${table_dir}/create.sql"

  # Load the combined CSV.
  psql "$URL" -c "\\COPY ${table} FROM '${table_dir}/combined.csv' WITH (FORMAT csv, HEADER true)"

  # Free disk space.
  rm -f "${table_dir}/combined.csv"
done

echo "==> Running VACUUM ANALYZE on all tables..."
for table in "${TABLES[@]}"; do
  psql "$URL" -c "VACUUM ANALYZE ${table};"
done

echo "==> StackOverflow dataset loaded successfully."

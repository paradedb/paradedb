#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
OUTPUT_DIR="${1:-$REPO_ROOT/extracted-code-snippets}"
SQL_DIR="$OUTPUT_DIR/sql-snippets"
RESULTS_DIR="$REPO_ROOT/verify/sql"
RESULTS_FILE="$RESULTS_DIR/results.tsv"

mkdir -p "$RESULTS_DIR"

source "${SCRIPT_DIR}/run_paradedb.sh"

if ! command -v psql >/dev/null 2>&1; then
  echo "psql is required to verify SQL snippets." >&2
  exit 1
fi

"${SCRIPT_DIR}/extract_code_snippets.sh" "$REPO_ROOT" "$OUTPUT_DIR" >/dev/null

if [[ ! -d "$SQL_DIR" ]]; then
  echo "SQL snippet directory not found: $SQL_DIR" >&2
  exit 1
fi

bootstrap_sql="$(mktemp "${TMPDIR:-/tmp}/paradedb-docs-bootstrap.XXXXXX.sql")"
trap 'rm -f "$bootstrap_sql"' EXIT

cat >"$bootstrap_sql" <<'SQL'
CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX mock_items_bm25_idx ON mock_items
USING bm25 (id, description, category, rating, in_stock, created_at, metadata, weight_range,  (description::pdb.simple('alias=description_simple')))
WITH (
  key_field = 'id',
  json_fields = '{"metadata":{"fast":true}}'
);

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'orders',
  table_type => 'Orders'
);

ALTER TABLE orders
ADD CONSTRAINT foreign_key_product_id
FOREIGN KEY (product_id)
REFERENCES mock_items(id);

CREATE INDEX orders_idx ON orders
USING bm25 (order_id, product_id, order_quantity, order_total, customer_name)
WITH (key_field = 'order_id');

SQL

psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -X -f "$bootstrap_sql" >/dev/null

printf "status\tsnippet\n" >"$RESULTS_FILE"

pass_count=0
fail_count=0

while IFS= read -r snippet_file; do
  rel_snippet="${snippet_file#$REPO_ROOT/}"

  if psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -X -f "$snippet_file" >/dev/null; then
    printf "pass\t%s\n" "$rel_snippet" >>"$RESULTS_FILE"
    echo "[PASS] $rel_snippet"
    pass_count=$((pass_count + 1))
  else
    printf "fail\t%s\n" "$rel_snippet" >>"$RESULTS_FILE"
    echo "[FAIL] $rel_snippet" >&2
    fail_count=$((fail_count + 1))
  fi
done < <(find "$SQL_DIR" -type f -name '*.sql' | LC_ALL=C sort)

echo "Passed: $pass_count"
echo "Failed: $fail_count"
echo "Results: $RESULTS_FILE"

if [[ $fail_count -gt 0 ]]; then
  exit 1
fi

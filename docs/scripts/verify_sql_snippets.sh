#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
PROJECT_ROOT="$(cd "${REPO_ROOT}/.." && pwd)"
OUTPUT_DIR="${1:-$REPO_ROOT/extracted-code-snippets}"
SQL_DIR="$OUTPUT_DIR/sql-snippets"
SQLALCHEMY_DIR="$OUTPUT_DIR/sqlalchemy-snippets"
SQL_RESULTS_DIR="$REPO_ROOT/verify/sql"
SQL_RESULTS_FILE="$SQL_RESULTS_DIR/results.tsv"
SQLALCHEMY_RESULTS_DIR="$REPO_ROOT/verify/sqlalchemy"
SQLALCHEMY_RESULTS_FILE="$SQLALCHEMY_RESULTS_DIR/results.tsv"
SQLALCHEMY_RUN_DIR="$SQLALCHEMY_RESULTS_DIR/runnables"
SQLALCHEMY_ROOT="${SQLALCHEMY_PARADEDB_ROOT:-$(cd "$PROJECT_ROOT/.." && pwd)/sqlalchemy-paradedb}"
SQLALCHEMY_PYTHON="${SQLALCHEMY_PYTHON:-$SQLALCHEMY_ROOT/.venv/bin/python}"
SQLALCHEMY_HARNESS="$REPO_ROOT/scripts/sqlalchemy_snippet_harness.py"

mkdir -p "$SQL_RESULTS_DIR" "$SQLALCHEMY_RESULTS_DIR" "$SQLALCHEMY_RUN_DIR"

source "${SCRIPT_DIR}/run_paradedb.sh"

if ! command -v psql >/dev/null 2>&1; then
  echo "psql is required to verify SQL snippets." >&2
  exit 1
fi

if [[ ! -d "$SQLALCHEMY_ROOT" ]]; then
  echo "SQLAlchemy repo not found: $SQLALCHEMY_ROOT" >&2
  echo "Set SQLALCHEMY_PARADEDB_ROOT to override the default sibling path." >&2
  exit 1
fi

if [[ ! -x "$SQLALCHEMY_PYTHON" ]]; then
  echo "SQLAlchemy Python interpreter not found: $SQLALCHEMY_PYTHON" >&2
  echo "Set SQLALCHEMY_PYTHON to override the default .venv interpreter." >&2
  exit 1
fi

if [[ ! -f "$SQLALCHEMY_HARNESS" ]]; then
  echo "SQLAlchemy harness not found: $SQLALCHEMY_HARNESS" >&2
  exit 1
fi

"${SCRIPT_DIR}/extract_code_snippets.sh" "$REPO_ROOT" "$OUTPUT_DIR" >/dev/null

if [[ ! -d "$SQL_DIR" ]]; then
  echo "SQL snippet directory not found: $SQL_DIR" >&2
  exit 1
fi

if [[ ! -d "$SQLALCHEMY_DIR" ]]; then
  echo "SQLAlchemy snippet directory not found: $SQLALCHEMY_DIR" >&2
  exit 1
fi

sql_snippet_total="$(find "$SQL_DIR" -type f -name '*.sql' | wc -l | tr -d ' ')"
sqlalchemy_snippet_total="$(find "$SQLALCHEMY_DIR" -type f -name '*.py' | wc -l | tr -d ' ')"

if [[ "$sql_snippet_total" == "0" ]]; then
  echo "No SQL snippets were extracted from the docs." >&2
  exit 1
fi

if [[ "$sqlalchemy_snippet_total" == "0" ]]; then
  echo "No SQLAlchemy snippets were extracted from the docs." >&2
  exit 1
fi

bootstrap_sql="$(mktemp -t paradedb-docs-bootstrap)"
trap 'rm -f "$bootstrap_sql"' EXIT

cat >"$bootstrap_sql" <<'SQL'
DROP TABLE IF EXISTS orders CASCADE;
DROP TABLE IF EXISTS mock_items CASCADE;

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

printf "status\tsnippet\n" >"$SQL_RESULTS_FILE"
printf "status\tsnippet\trunnable\n" >"$SQLALCHEMY_RESULTS_FILE"

sql_pass_count=0
sql_fail_count=0

while IFS= read -r snippet_file; do
  rel_snippet="${snippet_file#$REPO_ROOT/}"

  if psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -X -f "$snippet_file" >/dev/null; then
    printf "pass\t%s\n" "$rel_snippet" >>"$SQL_RESULTS_FILE"
    echo "[PASS] $rel_snippet"
    sql_pass_count=$((sql_pass_count + 1))
  else
    printf "fail\t%s\n" "$rel_snippet" >>"$SQL_RESULTS_FILE"
    echo "[FAIL] $rel_snippet" >&2
    sql_fail_count=$((sql_fail_count + 1))
  fi
done < <(find "$SQL_DIR" -type f -name '*.sql' | LC_ALL=C sort)

find "$SQLALCHEMY_RUN_DIR" -type f -name '*.py' -delete

while IFS= read -r snippet_file; do
  runner_file="$SQLALCHEMY_RUN_DIR/$(basename "$snippet_file")"
  rel_snippet="${snippet_file#$REPO_ROOT/}"

  cat >"$runner_file" <<'PY'
#!/usr/bin/env python3
from pathlib import Path
import sys

DOCS_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(DOCS_ROOT / "scripts"))

from sqlalchemy_snippet_harness import MockItem, Order, engine  # noqa: F401
PY

  printf "\n# Source: %s\n" "$rel_snippet" >>"$runner_file"
  cat "$snippet_file" >>"$runner_file"
  chmod +x "$runner_file"
done < <(find "$SQLALCHEMY_DIR" -type f -name '*.py' | LC_ALL=C sort)

sqlalchemy_pass_count=0
sqlalchemy_fail_count=0

while IFS= read -r runner_file; do
  rel_runner="${runner_file#$REPO_ROOT/}"
  snippet_name="$(basename "$runner_file")"
  rel_snippet="$(
    find "$SQLALCHEMY_DIR" -type f -name "$snippet_name" -print | head -n 1
  )"
  rel_snippet="${rel_snippet#$REPO_ROOT/}"

  if PYTHONPATH="$SQLALCHEMY_ROOT${PYTHONPATH:+:$PYTHONPATH}" \
    DATABASE_URL="$DATABASE_URL" \
    "$SQLALCHEMY_PYTHON" "$runner_file" >/dev/null; then
    printf "pass\t%s\t%s\n" "$rel_snippet" "$rel_runner" >>"$SQLALCHEMY_RESULTS_FILE"
    echo "[PASS] $rel_runner"
    sqlalchemy_pass_count=$((sqlalchemy_pass_count + 1))
  else
    printf "fail\t%s\t%s\n" "$rel_snippet" "$rel_runner" >>"$SQLALCHEMY_RESULTS_FILE"
    echo "[FAIL] $rel_runner" >&2
    sqlalchemy_fail_count=$((sqlalchemy_fail_count + 1))
  fi
done < <(find "$SQLALCHEMY_RUN_DIR" -type f -name '*.py' | LC_ALL=C sort)

echo "SQL passed: $sql_pass_count"
echo "SQL failed: $sql_fail_count"
echo "SQL results: $SQL_RESULTS_FILE"
echo "SQLAlchemy passed: $sqlalchemy_pass_count"
echo "SQLAlchemy failed: $sqlalchemy_fail_count"
echo "SQLAlchemy results: $SQLALCHEMY_RESULTS_FILE"

if [[ $sql_fail_count -gt 0 || $sqlalchemy_fail_count -gt 0 ]]; then
  exit 1
fi

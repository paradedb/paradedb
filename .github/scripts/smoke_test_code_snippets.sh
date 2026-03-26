#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
DOCS_ROOT="${REPO_ROOT}/docs"
OUTPUT_DIR="${1:-$SCRIPT_DIR/verify}"
SQL_DIR="$OUTPUT_DIR/sql"
DJANGO_DIR="$OUTPUT_DIR/django"
RAILS_DIR="$OUTPUT_DIR/rails"
SQLALCHEMY_DIR="$OUTPUT_DIR/sqlalchemy"
PARADEDB_CONTAINER_NAME="paradedb-docs-verify"
PARADEDB_IMAGE="paradedb/paradedb:0.22.2-pg18"
PARADEDB_HOST_PORT="5422"
DATABASE_URL="postgresql://postgres:postgres@localhost:${PARADEDB_HOST_PORT}/postgres"
export DATABASE_URL
export PGPASSWORD="postgres"

mkdir -p "$SQL_DIR" "$DJANGO_DIR" "$RAILS_DIR" "$SQLALCHEMY_DIR"

DJANGO_PYTHON_ENV_DIR="$(mktemp -d -t paradedb-docs-django-python.XXXXXX)"
DJANGO_PYTHON_BIN="$DJANGO_PYTHON_ENV_DIR/bin/python"
SQLALCHEMY_PYTHON_ENV_DIR="$(mktemp -d -t paradedb-docs-sqlalchemy-python.XXXXXX)"
SQLALCHEMY_PYTHON_BIN="$SQLALCHEMY_PYTHON_ENV_DIR/bin/python"
RUBY_GEM_HOME="$(mktemp -d -t paradedb-docs-ruby.XXXXXX)"
bootstrap_sql=""

cleanup() {
  [[ -n "$bootstrap_sql" ]] && rm -f "$bootstrap_sql"
  rm -rf "$DJANGO_PYTHON_ENV_DIR"
  rm -rf "$SQLALCHEMY_PYTHON_ENV_DIR"
  rm -rf "$RUBY_GEM_HOME"
}

trap cleanup EXIT

if docker ps --format '{{.Names}}' | grep -Fxq "$PARADEDB_CONTAINER_NAME"; then
  echo "ParadeDB container ${PARADEDB_CONTAINER_NAME} is already running..."
elif docker ps -a --format '{{.Names}}' | grep -Fxq "$PARADEDB_CONTAINER_NAME"; then
  echo "Starting existing ParadeDB container ${PARADEDB_CONTAINER_NAME}..."
  docker start "$PARADEDB_CONTAINER_NAME" >/dev/null
else
  echo "Starting ParadeDB container ${PARADEDB_CONTAINER_NAME} from ${PARADEDB_IMAGE}..."
  docker run -d \
    --name "$PARADEDB_CONTAINER_NAME" \
    -e POSTGRES_PASSWORD=postgres \
    -p "${PARADEDB_HOST_PORT}:5432" \
    "$PARADEDB_IMAGE" >/dev/null
fi

echo "Waiting for ParadeDB to become ready..."
for _ in {1..30}; do
  if docker exec "$PARADEDB_CONTAINER_NAME" pg_isready -U postgres -d postgres >/dev/null 2>&1; then
    break
  fi
  sleep 2
done

if ! docker exec "$PARADEDB_CONTAINER_NAME" pg_isready -U postgres -d postgres >/dev/null 2>&1; then
  echo "ParadeDB did not become ready in time" >&2
  exit 1
fi

echo "Creating temporary Python environment for Django snippet verification..."
python3 -m venv "$DJANGO_PYTHON_ENV_DIR"

echo "Installing latest django-paradedb from PyPI..."
PIP_DISABLE_PIP_VERSION_CHECK=1 "$DJANGO_PYTHON_BIN" -m pip install --quiet --upgrade \
  "django-paradedb" \
  "psycopg[binary]"

echo "Creating temporary Python environment for SQLAlchemy snippet verification..."
python3 -m venv "$SQLALCHEMY_PYTHON_ENV_DIR"

echo "Installing latest sqlalchemy-paradedb from PyPI..."
PIP_DISABLE_PIP_VERSION_CHECK=1 "$SQLALCHEMY_PYTHON_BIN" -m pip install --quiet --upgrade \
  "sqlalchemy-paradedb" \
  "psycopg[binary]"

echo "Installing latest rails-paradedb from RubyGems..."
GEM_HOME="$RUBY_GEM_HOME" GEM_PATH="$RUBY_GEM_HOME" \
  gem install --silent --no-document --install-dir "$RUBY_GEM_HOME" \
  "rails-paradedb" \
  "pg"

"$SQLALCHEMY_PYTHON_BIN" "${SCRIPT_DIR}/extract_code_snippets.py" "$DOCS_ROOT" "$OUTPUT_DIR" >/dev/null


bootstrap_sql="$(mktemp -t paradedb-docs-bootstrap)"

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

sql_pass_count=0
sql_fail_count=0

while IFS= read -r snippet_file; do
  rel_snippet="${snippet_file#"$REPO_ROOT"/}"

  if psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -X -f "$snippet_file" >/dev/null; then
    echo "[SUCCESS] $rel_snippet" >&2
    sql_pass_count=$((sql_pass_count + 1))
  else
    echo "[FAIL] $rel_snippet" >&2
    sql_fail_count=$((sql_fail_count + 1))
  fi
done < <(find "$SQL_DIR" -type f -name '*.sql' | LC_ALL=C sort)

django_pass_count=0
django_fail_count=0

while IFS= read -r snippet_file; do
  rel_snippet="${snippet_file#"$REPO_ROOT"/}"

  if {
    cat "${SCRIPT_DIR}/django_snippet_harness.py"
    cat <<PY

# Source: $rel_snippet
PY
    cat "$snippet_file"
  } | "$DJANGO_PYTHON_BIN" - >/dev/null
  then
    echo "[SUCCESS] $rel_snippet" >&2
    django_pass_count=$((django_pass_count + 1))
  else
    echo "[FAIL] $rel_snippet" >&2
    django_fail_count=$((django_fail_count + 1))
  fi
done < <(find "$DJANGO_DIR" -type f -name '*.py' | LC_ALL=C sort)

rails_pass_count=0
rails_fail_count=0

while IFS= read -r snippet_file; do
  rel_snippet="${snippet_file#"$REPO_ROOT"/}"

  if {
    cat "${SCRIPT_DIR}/rails_snippet_harness.rb"
    cat <<RUBY

# Source: $rel_snippet
RUBY
    cat "$snippet_file"
  } | RUBYLIB="$SCRIPT_DIR${RUBYLIB:+:$RUBYLIB}" \
      GEM_HOME="$RUBY_GEM_HOME" \
      GEM_PATH="$RUBY_GEM_HOME" \
      ruby - >/dev/null; then
    echo "[SUCCESS] $rel_snippet" >&2
    rails_pass_count=$((rails_pass_count + 1))
  else
    echo "[FAIL] $rel_snippet" >&2
    rails_fail_count=$((rails_fail_count + 1))
  fi
done < <(find "$RAILS_DIR" -type f -name '*.rb' | LC_ALL=C sort)

sqlalchemy_pass_count=0
sqlalchemy_fail_count=0

while IFS= read -r snippet_file; do
  rel_snippet="${snippet_file#"$REPO_ROOT"/}"

  if {
    cat <<PY
from sqlalchemy_snippet_harness import MockItem, Order, engine

# Source: $rel_snippet
PY
    cat "$snippet_file"
  } | PYTHONPATH="$SCRIPT_DIR${PYTHONPATH:+:$PYTHONPATH}" "$SQLALCHEMY_PYTHON_BIN" - >/dev/null; then
    echo "[SUCCESS] $rel_snippet" >&2
    sqlalchemy_pass_count=$((sqlalchemy_pass_count + 1))
  else
    echo "[FAIL] $rel_snippet" >&2
    sqlalchemy_fail_count=$((sqlalchemy_fail_count + 1))
  fi
done < <(find "$SQLALCHEMY_DIR" -type f -name '*.py' | LC_ALL=C sort)

echo "SQL passed: $sql_pass_count failed: $sql_fail_count"
echo "Django passed: $django_pass_count failed: $django_fail_count"
echo "Rails passed: $rails_pass_count failed: $rails_fail_count"
echo "SQLAlchemy passed: $sqlalchemy_pass_count failed: $sqlalchemy_fail_count"

if [[ $sql_fail_count -gt 0 || $django_fail_count -gt 0 || $rails_fail_count -gt 0 || $sqlalchemy_fail_count -gt 0 ]]; then
  exit 1
fi

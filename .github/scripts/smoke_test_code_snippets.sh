#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
VERIFY_DIR="${SCRIPT_DIR}/verify"
SQL_DIR="${VERIFY_DIR}/sql"
DJANGO_DIR="${VERIFY_DIR}/django"
RAILS_DIR="${VERIFY_DIR}/rails"
SQLALCHEMY_DIR="${VERIFY_DIR}/sqlalchemy"
RESET_INDEXES_SQL="${SCRIPT_DIR}/reset_code_snippet_indexes.sql"
PARADEDB_HOST="${PARADEDB_HOST:-localhost}"
PARADEDB_PORT="${PARADEDB_PORT:-28818}"
PARADEDB_DATABASE="${PARADEDB_DATABASE:-postgres}"
PARADEDB_USER="${PARADEDB_USER:-$(id -un)}"
PARADEDB_PASSWORD="${PARADEDB_PASSWORD:-}"
PYTHON_ENV_DIR="$(mktemp -d -t paradedb-docs-python.XXXXXX)"
PYTHON_BIN="$PYTHON_ENV_DIR/bin/python"
RUBY_GEM_HOME="$(mktemp -d -t paradedb-docs-ruby.XXXXXX)"

cleanup() {
  rm -rf "$PYTHON_ENV_DIR"
  rm -rf "$RUBY_GEM_HOME"
}

trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

export PARADEDB_HOST PARADEDB_PORT PARADEDB_DATABASE PARADEDB_USER PARADEDB_PASSWORD
export PGHOST="$PARADEDB_HOST"
export PGPORT="$PARADEDB_PORT"
export PGDATABASE="$PARADEDB_DATABASE"
export PGUSER="$PARADEDB_USER"
export PGPASSWORD="$PARADEDB_PASSWORD"

PSQL=(psql -v ON_ERROR_STOP=1)

GREEN=$'\033[32m'
RED=$'\033[31m'
RESET=$'\033[0m'

exit_if_interrupted() {
  local status="$1"

  if [[ $status -eq 130 || $status -eq 143 ]]; then
    exit "$status"
  fi
}

run_psql_file() {
  local sql_file="$1"
  local output
  local status

  if output="$("${PSQL[@]}" -f "$sql_file" 2>&1 >/dev/null)"; then
    :
  else
    status=$?
    printf '%s\n' "$output" >&2
    exit_if_interrupted "$status"
    return 1
  fi

  if [[ -n "$output" ]]; then
    printf '%s\n' "$output" >&2
  fi

  if grep -Eq '(^|:) WARNING:' <<<"$output"; then
    return 1
  fi
}

reset_snippet_indexes() {
  run_psql_file "$RESET_INDEXES_SQL"
}

echo "Creating temporary Python environment for Python snippet verification..."
python3 -m venv "$PYTHON_ENV_DIR"

echo "Installing latest Django and SQLAlchemy ParadeDB clients from PyPI..."
PIP_DISABLE_PIP_VERSION_CHECK=1 "$PYTHON_BIN" -m pip install --quiet --upgrade \
  "django-paradedb" \
  "sqlalchemy-paradedb" \
  "psycopg[binary]"

echo "Installing latest rails-paradedb from RubyGems..."
GEM_HOME="$RUBY_GEM_HOME" GEM_PATH="$RUBY_GEM_HOME" \
  gem install --silent --no-document --install-dir "$RUBY_GEM_HOME" \
  "rails-paradedb" \
  "pg"

"$PYTHON_BIN" "${SCRIPT_DIR}/extract_code_snippets.py" >/dev/null

run_psql_file "${SCRIPT_DIR}/bootstrap_code_snippet_tables.sql"

sql_pass_count=0
sql_fail_count=0

while IFS= read -r snippet_file; do
  rel_snippet="${snippet_file#"$REPO_ROOT"/}"

  if reset_snippet_indexes && run_psql_file "$snippet_file"; then
    echo "${GREEN}[SUCCESS]${RESET} $rel_snippet" >&2
    sql_pass_count=$((sql_pass_count + 1))
  else
    echo "${RED}[FAIL]${RESET} $rel_snippet" >&2
    sql_fail_count=$((sql_fail_count + 1))
  fi
done < <(find "$SQL_DIR" -type f -name '*.sql' | LC_ALL=C sort)

django_pass_count=0
django_fail_count=0

while IFS= read -r snippet_file; do
  rel_snippet="${snippet_file#"$REPO_ROOT"/}"

  if reset_snippet_indexes && {
    cat "${SCRIPT_DIR}/django_snippet_harness.py"
    cat <<PY

# Source: $rel_snippet
PY
    cat "$snippet_file"
  } | "$PYTHON_BIN" - >/dev/null
  then
    echo "${GREEN}[SUCCESS]${RESET} $rel_snippet" >&2
    django_pass_count=$((django_pass_count + 1))
  else
    exit_if_interrupted "$?"
    echo "${RED}[FAIL]${RESET} $rel_snippet" >&2
    django_fail_count=$((django_fail_count + 1))
  fi
done < <(find "$DJANGO_DIR" -type f -name '*.py' | LC_ALL=C sort)

rails_pass_count=0
rails_fail_count=0

while IFS= read -r snippet_file; do
  rel_snippet="${snippet_file#"$REPO_ROOT"/}"

  if reset_snippet_indexes && {
    cat "${SCRIPT_DIR}/rails_snippet_harness.rb"
    cat <<RUBY

# Source: $rel_snippet
RUBY
    cat "$snippet_file"
  } | RUBYLIB="$SCRIPT_DIR${RUBYLIB:+:$RUBYLIB}" \
      GEM_HOME="$RUBY_GEM_HOME" \
      GEM_PATH="$RUBY_GEM_HOME" \
      ruby - >/dev/null; then
    echo "${GREEN}[SUCCESS]${RESET} $rel_snippet" >&2
    rails_pass_count=$((rails_pass_count + 1))
  else
    exit_if_interrupted "$?"
    echo "${RED}[FAIL]${RESET} $rel_snippet" >&2
    rails_fail_count=$((rails_fail_count + 1))
  fi
done < <(find "$RAILS_DIR" -type f -name '*.rb' | LC_ALL=C sort)

sqlalchemy_pass_count=0
sqlalchemy_fail_count=0

while IFS= read -r snippet_file; do
  rel_snippet="${snippet_file#"$REPO_ROOT"/}"

  if reset_snippet_indexes && {
    cat <<PY
from sqlalchemy_snippet_harness import MockItem, Order, engine

# Source: $rel_snippet
PY
    cat "$snippet_file"
  } | PYTHONPATH="$SCRIPT_DIR${PYTHONPATH:+:$PYTHONPATH}" "$PYTHON_BIN" - >/dev/null; then
    echo "${GREEN}[SUCCESS]${RESET} $rel_snippet" >&2
    sqlalchemy_pass_count=$((sqlalchemy_pass_count + 1))
  else
    exit_if_interrupted "$?"
    echo "${RED}[FAIL]${RESET} $rel_snippet" >&2
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

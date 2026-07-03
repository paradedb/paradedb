#!/usr/bin/env bash

set -euo pipefail

# If you don't want check the snippets for all languages at once, pass in the list you'd like to check:
# scripts/smoke_test_code_snippets.sh sql rails
ORMS=${*:-'sql django sqlalchemy rails drizzle efcore'}
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
VERIFY_DIR="${SCRIPT_DIR}/verify"
SQL_DIR="${VERIFY_DIR}/sql"
DJANGO_DIR="${VERIFY_DIR}/django"
RAILS_DIR="${VERIFY_DIR}/rails"
SQLALCHEMY_DIR="${VERIFY_DIR}/sqlalchemy"
EFCORE_DIR="${VERIFY_DIR}/efcore"
PARADEDB_HOST="${PARADEDB_HOST:-localhost}"
PARADEDB_PORT="${PARADEDB_PORT:-28818}"
PARADEDB_DATABASE="${PARADEDB_DATABASE:-postgres}"
PARADEDB_USER="${PARADEDB_USER:-$(id -un)}"
PARADEDB_PASSWORD="${PARADEDB_PASSWORD:-}"
PYTHON_ENV_DIR="$(mktemp -d -t paradedb-docs-python.XXXXXX)"
PYTHON_BIN="$PYTHON_ENV_DIR/bin/python"
RUBY_GEM_HOME="$(mktemp -d -t paradedb-docs-ruby.XXXXXX)"
JAVASCRIPT_ENV_DIR="$(mktemp -d -t paradedb-docs-javascript.XXXXXX)"
CSHARP_ENV_DIR="$(mktemp -d -t paradedb-docs-csharp.XXXXXX)"

cleanup() {
  rm -rf "$PYTHON_ENV_DIR"
  rm -rf "$RUBY_GEM_HOME"
  rm -rf "$JAVASCRIPT_ENV_DIR"
  rm -rf "$CSHARP_ENV_DIR"
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
export DATABASE_URL="${DATABASE_URL:-postgres://${PARADEDB_USER}${PARADEDB_PASSWORD:+:${PARADEDB_PASSWORD}}@${PARADEDB_HOST}:${PARADEDB_PORT}/${PARADEDB_DATABASE}}"

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

create_snippet_indexes() {
  run_psql_file "${SCRIPT_DIR}/create_code_snippet_indexes.sql"
}

drop_snippet_indexes() {
  run_psql_file "${SCRIPT_DIR}/drop_code_snippet_indexes.sql"
}

python3 "${SCRIPT_DIR}/extract_code_snippets.py" >/dev/null


sql_pass_count=0
sql_fail_count=0
if [[ $ORMS =~ "sql" ]]; then
  run_psql_file "${SCRIPT_DIR}/bootstrap_code_snippet_tables.sql"


  while IFS= read -r snippet_file; do
    rel_snippet="${snippet_file#"$REPO_ROOT"/}"

    drop_snippet_indexes

    if ! grep -Fq 'CREATE INDEX' "$snippet_file"; then
      create_snippet_indexes
    fi

    if run_psql_file "$snippet_file"; then
      echo "${GREEN}[SUCCESS]${RESET} $rel_snippet" >&2
      sql_pass_count=$((sql_pass_count + 1))
    else
      echo "${RED}[FAIL]${RESET} $rel_snippet" >&2
      sql_fail_count=$((sql_fail_count + 1))
    fi
  done < <(find "$SQL_DIR" -type f -name '*.sql' | LC_ALL=C sort)
fi

django_pass_count=0
django_fail_count=0
if [[ $ORMS =~ "django" ]]; then
  if [[ ! -x "$PYTHON_BIN" ]]; then
    echo "Creating temporary Python environment for Python snippet verification..."
    python3 -m venv "$PYTHON_ENV_DIR"
  fi

  echo "Installing Django ParadeDB client from PyPI..."
  PIP_DISABLE_PIP_VERSION_CHECK=1 "$PYTHON_BIN" -m pip install --quiet --upgrade \
    "django-paradedb==0.8.0" \
    "psycopg[binary]"

  while IFS= read -r snippet_file; do
    rel_snippet="${snippet_file#"$REPO_ROOT"/}"

    drop_snippet_indexes

    if ! grep -Eq 'schema_editor\.add_index' "$snippet_file"; then
      create_snippet_indexes
    fi

    if {
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
fi

rails_pass_count=0
rails_fail_count=0
if [[ $ORMS =~ "rails" ]]; then
  echo "Installing rails-paradedb from RubyGems..."
  GEM_HOME="$RUBY_GEM_HOME" GEM_PATH="$RUBY_GEM_HOME" \
    gem install --silent --no-document --install-dir "$RUBY_GEM_HOME" \
    "rails-paradedb:0.8.0" \
    "pg"

  while IFS= read -r snippet_file; do
    rel_snippet="${snippet_file#"$REPO_ROOT"/}"

    drop_snippet_indexes

    if ! grep -Fq 'add_bm25_index' "$snippet_file"; then
      create_snippet_indexes
    fi

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
      echo "${GREEN}[SUCCESS]${RESET} $rel_snippet" >&2
      rails_pass_count=$((rails_pass_count + 1))
    else
      exit_if_interrupted "$?"
      echo "${RED}[FAIL]${RESET} $rel_snippet" >&2
      rails_fail_count=$((rails_fail_count + 1))
    fi
  done < <(find "$RAILS_DIR" -type f -name '*.rb' | LC_ALL=C sort)
fi


sqlalchemy_pass_count=0
sqlalchemy_fail_count=0
if [[ $ORMS =~ "sqlalchemy" ]]; then
  if [[ ! -x "$PYTHON_BIN" ]]; then
    echo "Creating temporary Python environment for Python snippet verification..."
    python3 -m venv "$PYTHON_ENV_DIR"
  fi

  echo "Installing SQLAlchemy ParadeDB client from PyPI..."
  PIP_DISABLE_PIP_VERSION_CHECK=1 "$PYTHON_BIN" -m pip install --quiet --upgrade \
    "sqlalchemy-paradedb==0.6.0" \
    "psycopg[binary]"

  while IFS= read -r snippet_file; do
    rel_snippet="${snippet_file#"$REPO_ROOT"/}"

    drop_snippet_indexes

    if ! grep -Fq 'idx.create' "$snippet_file"; then
      create_snippet_indexes
    fi

    if {
      cat <<PY
from sqlalchemy_snippet_harness import MockItem, Order, ArrayDemo, engine

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
fi

drizzle_pass_count=0
drizzle_fail_count=0
if [[ $ORMS =~ "drizzle" ]]; then
  echo "Installing @paradedb/drizzle-paradedb from npm..."
  npm --prefix "$JAVASCRIPT_ENV_DIR" install --silent \
    "@paradedb/drizzle-paradedb@0.1.0" \
    "drizzle-orm" \
    "postgres" \
    "tsx"

  while IFS= read -r snippet_file; do
    rel_snippet="${snippet_file#"$REPO_ROOT"/}"

    run_psql_file "${SCRIPT_DIR}/bootstrap_code_snippet_tables.sql"
    drop_snippet_indexes

    if ! grep -Fq 'bm25Index' "$snippet_file"; then
      create_snippet_indexes
    fi

    if {
      cat "${SCRIPT_DIR}/drizzle_snippet_harness.ts"
      cat <<TS
// Source: $rel_snippet
TS
      cat "$snippet_file"
      cat <<'TS'

await client.end();
TS
    } | (cd "$JAVASCRIPT_ENV_DIR" && npm exec -- tsx -) >/dev/null; then
      echo "${GREEN}[SUCCESS]${RESET} $rel_snippet" >&2
      drizzle_pass_count=$((drizzle_pass_count + 1))
    else
      exit_if_interrupted "$?"
      echo "${RED}[FAIL]${RESET} $rel_snippet" >&2
      drizzle_fail_count=$((drizzle_fail_count + 1))
    fi
  done < <(find "${VERIFY_DIR}/drizzle" -type f -name '*.ts' | LC_ALL=C sort)
fi

efcore_pass_count=0
efcore_fail_count=0
if [[ $ORMS =~ "efcore" ]]; then
  echo "Installing ParadeDB.EntityFrameworkCore from NuGet..."
  dotnet new console --framework net10.0 --output "$CSHARP_ENV_DIR" >/dev/null
  dotnet add "$CSHARP_ENV_DIR" package ParadeDB.EntityFrameworkCore \
    --version 0.1.0 \
    >/dev/null
  dotnet restore "$CSHARP_ENV_DIR" -p:NuGetAudit=false >/dev/null

  while IFS= read -r snippet_file; do
    rel_snippet="${snippet_file#"$REPO_ROOT"/}"

    run_psql_file "${SCRIPT_DIR}/bootstrap_code_snippet_tables.sql"
    drop_snippet_indexes
    if ! grep -Fq 'CREATE INDEX' "$snippet_file"; then
      create_snippet_indexes
    fi

    while IFS= read -r harness_line; do
      if [[ $harness_line == "// __PARADEDB_SNIPPET__" ]]; then
        if ! grep -Fq 'modelBuilder.' "$snippet_file"; then
          printf '// Source: %s\n' "$rel_snippet"
          cat "$snippet_file"
        fi
      elif [[ $harness_line == "        // __PARADEDB_MODEL_SNIPPET__" ]]; then
        if grep -Fq 'modelBuilder.' "$snippet_file"; then
          printf '        // Source: %s\n' "$rel_snippet"
          sed 's/^/        /' "$snippet_file"
        fi
      else
        printf '%s\n' "$harness_line"
      fi
    done <"${SCRIPT_DIR}/efcore_snippet_harness.cs" >"${CSHARP_ENV_DIR}/Program.cs"

    if dotnet run --no-restore --project "$CSHARP_ENV_DIR"; then
      echo "${GREEN}[SUCCESS]${RESET} $rel_snippet" >&2
      efcore_pass_count=$((efcore_pass_count + 1))
    else
      exit_if_interrupted "$?"
      echo "${RED}[FAIL]${RESET} $rel_snippet" >&2
      efcore_fail_count=$((efcore_fail_count + 1))
    fi
  done < <(find "$EFCORE_DIR" -type f -name '*.cs' | LC_ALL=C sort)
fi

echo "SQL passed: $sql_pass_count failed: $sql_fail_count"
echo "Django passed: $django_pass_count failed: $django_fail_count"
echo "Rails passed: $rails_pass_count failed: $rails_fail_count"
echo "SQLAlchemy passed: $sqlalchemy_pass_count failed: $sqlalchemy_fail_count"
echo "Drizzle passed: $drizzle_pass_count failed: $drizzle_fail_count"
echo "EF Core passed: $efcore_pass_count failed: $efcore_fail_count"

if [[ $sql_fail_count -gt 0 || $django_fail_count -gt 0 || $rails_fail_count -gt 0 || $sqlalchemy_fail_count -gt 0 || $drizzle_fail_count -gt 0 || $efcore_fail_count -gt 0 ]]; then
  exit 1
fi

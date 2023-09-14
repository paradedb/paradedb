#!/bin/bash

# This script runs integration tests on the pg_search extension using pg_regress. To add new tests, add
# a new .sql file to the test/sql directory and add the corresponding .out file to the test/expected
# directory, and it will automatically get executed by this script. To run unit tests, use `cargo pgrx test`.

# Exit on subcommand errors
set -Eeuo pipefail

TMPDIR="$(mktemp -d)"
TESTDIR="$(dirname "$0")"
export PGDATA="$TMPDIR"
export PGHOST="$TMPDIR"
export PGUSER=postgres
export PGDATABASE=postgres
export PGPASSWORD=password

# Create a temporary password file
PWFILE=$(mktemp)
echo "$PGPASSWORD" > "$PWFILE"

# Ensure a clean environment
trap 'pg_ctl stop -m i; rm -f "$PWFILE"' sigint sigterm exit  # <-- Also remove the password file on exit
rm -rf "$TMPDIR"

# Initialize the test database
initdb --no-locale --encoding=UTF8 --nosync -U "$PGUSER" --auth-local=md5 --auth-host=md5 --pwfile="$PWFILE"
pg_ctl start -o "-F -c listen_addresses=\"\" -c log_min_messages=WARNING -k $PGDATA"
createdb test_db

# Determine paths based on OS and PostgreSQL version
OS_NAME=$(uname)
PG_VERSION=$(pg_config --version | awk '{print $2}' | cut -d '.' -f1)
case "$OS_NAME" in
  Darwin)
    REGRESS="/opt/homebrew/opt/postgresql@$PG_VERSION/lib/postgresql/pgxs/src/test/regress/pg_regress"
    ;;
  Linux)
    REGRESS="/usr/lib/postgresql/$PG_VERSION/lib/pgxs/src/test/regress/pg_regress"
    ;;
esac

# Check that the required extensions are installed on the test system
REQUIRED_EXTENSIONS=("pg_search" "pg_bm25" "vector")
for ext in "${REQUIRED_EXTENSIONS[@]}"; do
  CONTROL_FILE_PATH=""
  case "$OS_NAME" in
    Darwin)
      CONTROL_FILE_PATH="/opt/homebrew/opt/postgresql@$PG_VERSION/share/postgresql@$PG_VERSION/extension/$ext.control"
      ;;
    Linux)
      CONTROL_FILE_PATH="/usr/share/postgresql/$PG_VERSION/extension/$ext.control"
      ;;
  esac

  if [[ ! -f "$CONTROL_FILE_PATH" ]]; then
    if [[ "$ext" == "pg_search" ]]; then
      echo "Error: The $ext PostgreSQL extension isn't installed. Please install it with 'cargo pgrx install' and re-run this script."
    else
      echo "Error: The $ext PostgreSQL extension isn't installed. Please install it with './configure.sh' and re-run this script."
    fi
    exit 1
  fi
done

# Get a list of all tests
while IFS= read -r line; do
  TESTS+=("$line")
done < <(find "${TESTDIR}/sql" -type f -name "*.sql" -exec basename {} \; | sed -e 's/\..*$//' | sort)

# Execute tests using pg_regress
psql -v ON_ERROR_STOP=1 -f "${TESTDIR}/fixtures.sql" -d test_db
${REGRESS} --use-existing --dbname=test_db --inputdir="${TESTDIR}" "${TESTS[@]}"

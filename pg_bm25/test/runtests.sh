#!/bin/bash

# This script runs integration tests on the pg_bm25 extension using pg_regress. To add new tests, add
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
    CONTROL_FILE_PATH="/opt/homebrew/opt/postgresql@$PG_VERSION/share/postgresql@$PG_VERSION/extension/pg_bm25.control"
    ;;
  Linux)
    REGRESS="/usr/lib/postgresql/$PG_VERSION/lib/pgxs/src/test/regress/pg_regress"
    CONTROL_FILE_PATH="/usr/share/postgresql/$PG_VERSION/extension/pg_bm25.control"
    ;;
esac

# Check that the pg_bm25 extension is installed on the test system
if [[ ! -f "$CONTROL_FILE_PATH" ]]; then
  echo "Error: The pg_bm25 PostgreSQL extension isn't installed. Please install the extension with cargo pgrx install and re-run this script."
  exit 1
fi

# Get a list of all tests
while IFS= read -r line; do
  TESTS+=("$line")
done < <(find "${TESTDIR}/sql" -type f -name "*.sql" -exec basename {} \; | sed -e 's/\..*$//' | sort)

# Execute tests using pg_regress
psql -v ON_ERROR_STOP=1 -f "${TESTDIR}/fixtures.sql" -d test_db
${REGRESS} --use-existing --dbname=test_db --inputdir="${TESTDIR}" "${TESTS[@]}"

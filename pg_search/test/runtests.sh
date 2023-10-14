#!/bin/bash

# This script runs integration tests on the pg_search extension using pg_regress. To add
# new tests, add
# a new .sql file to the test/sql directory and add the corresponding .out file to the test/expected
# directory, and it will automatically get executed by this script. To run unit tests, use `cargo pgrx test`.

# Exit on subcommand errors
set -Eeuo pipefail

# Handle params
usage() {
  echo "Usage: $0 [OPTIONS]"
  echo "Options:"
  echo " -h (optional),   Display this help message"
  echo " -p (required),   Processing type, either <sequential> or <threaded>"
  echo " -v (optional),   PG version(s) separated by comma <11,12,13>"
  exit 1
}

# Do not allow script to be called without params
if [[ ! $* =~ ^\-.+ ]]
then
  usage
fi

# Instantiate vars
FLAG_PG_VER=false
FLAG_PROCESS_TYPE=false

# Assign flags to vars and check
while getopts "hp:v:" flag
do
  case $flag in
    h)
      usage
      ;;
    p)
      FLAG_PROCESS_TYPE=$OPTARG
    case "$FLAG_PROCESS_TYPE" in sequential | threaded ): # Do nothing
          ;;
        *)
          usage
          ;;
      esac
      ;;
    v)
      FLAG_PG_VER=$OPTARG
      ;;
    *)
      usage
      ;;
  esac
done


OS_NAME=$(uname)
TESTDIR="$(dirname "$0")"
export PGUSER=postgres
export PGDATABASE=postgres
export PGPASSWORD=password


# All pgrx-supported PostgreSQL versions to configure for
OS_NAME=$(uname)
if [ "$FLAG_PG_VER" = false ]; then
  # No arguments provided; use default versions
  case "$OS_NAME" in
    Darwin)
      PG_VERSIONS=("15.4" "14.9" "13.12" "12.16" "11.21")
      ;;
    Linux)
      PG_VERSIONS=("15" "14" "13" "12" "11")
      ;;
  esac
else
  IFS=',' read -ra PG_VERSIONS <<< "$FLAG_PG_VER"  # Split the argument by comma into an array
fi

function run_tests() {
  TMPDIR="$(mktemp -d)"
  export PGDATA="$TMPDIR"
  export PGHOST="$TMPDIR"

  echo ""
  echo "*************************************************"
  echo "* Running tests for PostgreSQL version: $PG_VERSION"
  echo "*************************************************"
  echo ""

  # Get the paths to the psql & pg_regress binaries for the current PostgreSQL version
  case "$OS_NAME" in
    Darwin)
      PG_BIN_PATH="$HOME/.pgrx/$PG_VERSION/pgrx-install/bin"
      REGRESS="$HOME/.pgrx/$PG_VERSION/pgrx-install/lib/postgresql/pgxs/src/test/regress/pg_regress"
      ;;
    Linux)
      PG_BIN_PATH="/usr/lib/postgresql/$PG_VERSION/bin"
      REGRESS="/usr/lib/postgresql/$PG_VERSION/lib/pgxs/src/test/regress/pg_regress"
      ;;
  esac

  # Create a temporary password file
  PWFILE=$(mktemp)
  echo "$PGPASSWORD" > "$PWFILE"

  # Ensure a clean environment
  trap '$PG_BIN_PATH/pg_ctl stop -m i; rm -f "$PWFILE"' sigint sigterm exit  # <-- Also remove the password file on exit
  rm -rf "$TMPDIR"
  unset TESTS

  # Initialize the test database
  "$PG_BIN_PATH/initdb" --no-locale --encoding=UTF8 --nosync -U "$PGUSER" --auth-local=md5 --auth-host=md5 --pwfile="$PWFILE"
  "$PG_BIN_PATH/pg_ctl" start -o "-F -c listen_addresses=\"\" -c log_min_messages=WARNING -k $PGDATA"
  "$PG_BIN_PATH/createdb" test_db

  # Install the dependencies with
  echo ""
  echo "Installing dependencies (pg_bm25 and pgvector) onto the test database..."
  "$TESTDIR/../configure.sh" "$PG_VERSION"

  # Use cargo-pgx to install the extension for the specified version
  echo ""
  echo "Installing pg_search extension onto the test database..."
  cargo pgrx install --pg-config="$PG_BIN_PATH/pg_config" --release

  # Get a list of all tests
  while IFS= read -r line; do
    TESTS+=("$line")
  done < <(find "${TESTDIR}/sql" -type f -name "*.sql" -exec basename {} \; | sed -e 's/\..*$//' | sort)

  # Execute tests using pg_regress
  echo ""
  echo "Running tests..."
  "$PG_BIN_PATH/psql" -v ON_ERROR_STOP=1 -f "${TESTDIR}/fixtures.sql" -d test_db
  ${REGRESS} --use-existing --dbname=test_db --inputdir="${TESTDIR}" "${TESTS[@]}"
}

# Loop over PostgreSQL versions
for PG_VERSION in "${PG_VERSIONS[@]}"; do
  echo ""
  if [ "$FLAG_PROCESS_TYPE" = "threaded" ]; then
    echo "Running tests in parallel..."
    run_tests &
  else
    echo "Running tests sequentially..."
    run_tests
  fi
done
wait # wait for all child processes to finish

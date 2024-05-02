#!/bin/bash

# This script runs integration tests on the pg_lakehouse extension using cargo test.
# This is only necessary in CI. Tests can be run with cargo test in local dev.

# Exit on subcommand errors
set -Eeuo pipefail

# Handle params
usage() {
  echo "Usage: $0 [OPTIONS]"
  echo "Options:"
  echo " -h (optional),   Display this help message"
  echo " -p (required),   Processing type, either <sequential> or <threaded>"
  echo " -v (optional),   PG version(s) separated by comma <12,13,14>"
  echo " -u (optional),   Version to test upgrading to before running tests (only meant for use in CI) <0.3.7>"
  echo " -a (optional),   Architecture to use for testing extension upgrade (only meant for use in CI) <amd64,arm64>"
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
FLAG_UPGRADE_VER=""
FLAG_ARCH="amd64"

# Assign flags to vars and check
while getopts "hp:v:u:a:" flag
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
    u)
      FLAG_UPGRADE_VER=$OPTARG
      ;;
    a)
      FLAG_ARCH=$OPTARG
      ;;
    *)
      usage
      ;;
  esac
done

# Determine the base directory of the script
BASEDIR=$(pwd)

# Vars
OS_NAME=$(uname)
export PGUSER=postgres
export PGDATABASE=postgres
export PGPASSWORD=password

# All pgrx-supported PostgreSQL versions to configure for
OS_NAME=$(uname)
if [ "$FLAG_PG_VER" = false ]; then
  # No arguments provided; use default versions
  PG_VERSIONS=("16" "15" "14" "13" "12")
else
  IFS=',' read -ra PG_VERSIONS <<< "$FLAG_PG_VER"  # Split the argument by comma into an array
fi

function run_tests() {
  TMPDIR="$(mktemp -d)"
  export PGDATA="$TMPDIR"
  export PGHOST="$TMPDIR"

  # Get the paths to the psql & pg_regress binaries for the current PostgreSQL version
  case "$OS_NAME" in
    Darwin)
      # Check arch to set proper pg_config path
      if [ "$(uname -m)" = "arm64" ]; then
        PG_BIN_PATH="/opt/homebrew/opt/postgresql@$PG_VERSION/bin"
      elif [ "$(uname -m)" = "x86_64" ]; then
        PG_BIN_PATH="/usr/local/opt/postgresql@$PG_VERSION/bin"
      else
        echo "Unknown arch, exiting..."
        exit 1
      fi
      ;;
    Linux)
      PG_BIN_PATH="/usr/lib/postgresql/$PG_VERSION/bin"
      ;;
  esac

  # Create a temporary password file
  PWFILE=$(mktemp)
  echo "$PGPASSWORD" > "$PWFILE"

  # Ensure a clean environment
  unset TESTS

  # Initialize the test database
  echo "Initializing PostgreSQL test database..."
  "$PG_BIN_PATH/initdb" --no-locale --encoding=UTF8 --nosync -U "$PGUSER" --auth-local=md5 --auth-host=md5 --pwfile="$PWFILE"
  "$PG_BIN_PATH/pg_ctl" start -o "-F -c listen_addresses=\"\" -c log_min_messages=WARNING -k $PGDATA"
  "$PG_BIN_PATH/createdb" test_db

  # Set PostgreSQL logging configuration
  echo "Setting test database logging configuration..."
  "$PG_BIN_PATH/psql" -v ON_ERROR_STOP=1 -c "ALTER SYSTEM SET logging_collector TO 'on';" -d test_db
  "$PG_BIN_PATH/psql" -v ON_ERROR_STOP=1 -c "ALTER SYSTEM SET log_directory TO '$BASEDIR/test/';" -d test_db
  "$PG_BIN_PATH/psql" -v ON_ERROR_STOP=1 -c "ALTER SYSTEM SET log_filename TO 'test_logs.log';" -d test_db

  # Configure search_path to include the paradedb schema
  "$PG_BIN_PATH/psql" -v ON_ERROR_STOP=1 -c "ALTER USER $PGUSER SET search_path TO public,paradedb;" -d test_db

  # Reload PostgreSQL configuration
  echo "Reloading PostgreSQL configuration..."
  "$PG_BIN_PATH/pg_ctl" restart

  # Don't send telemetry when running tests
  export TELEMETRY=false

  # Configure pgrx to use system PostgreSQL
  echo "Initializing pgrx environment..."
  cargo pgrx init "--pg$PG_VERSION=$PG_BIN_PATH/pg_config"

  # First, download & install the first release at which we started supporting upgrades for Postgres 16 (v0.7.0)
  BASE_RELEASE="0.7.0"

  # This block runs a test whether our extension can upgrade to the current version
  if [ -n "$FLAG_UPGRADE_VER" ] && [ "$FLAG_UPGRADE_VER" != "$BASE_RELEASE" ]; then
    echo "Running extension upgrade test..."
    DOWNLOAD_URL="https://github.com/paradedb/paradedb/releases/download/v$BASE_RELEASE/pg_lakehouse-v$BASE_RELEASE-pg$PG_VERSION-$FLAG_ARCH-ubuntu2204.deb"
    curl -LOJ "$DOWNLOAD_URL"
    sudo dpkg -i "pg_lakehouse-v$BASE_RELEASE-pg$PG_VERSION-$FLAG_ARCH-ubuntu2204.deb"

    # Second, load the extension into the test database
    echo "Loading pg_lakehouse extension version v$BASE_RELEASE into the test database..."
    "$PG_BIN_PATH/psql" -v ON_ERROR_STOP=1 -c "CREATE EXTENSION pg_lakehouse VERSION '$BASE_RELEASE';" -d test_db

    # Third, build & install the current version of the extension
    echo "Building & installing the current version of the pg_lakehouse extension..."
    sudo chown -R "$(whoami)" "/usr/share/postgresql/$PG_VERSION/extension/" "/usr/lib/postgresql/$PG_VERSION/lib/"
    cargo pgrx install --pg-config="$PG_BIN_PATH/pg_config" --release

    # Fourth, upgrade the extension installed on the test database to the current version
    "$PG_BIN_PATH/psql" -v ON_ERROR_STOP=1 -c "ALTER EXTENSION pg_lakehouse UPDATE TO '$FLAG_UPGRADE_VER';" -d test_db
  else
    # Use cargo-pgx to install the extension for the specified version
    echo "Installing pg_lakehouse extension onto the test database..."
    cargo pgrx install --pg-config="$PG_BIN_PATH/pg_config" --profile dev
  fi

  # Reload PostgreSQL configuration
  echo "Reloading PostgreSQL configuration..."
  "$PG_BIN_PATH/pg_ctl" restart

  DATABASE_PORT=$(psql -c "SHOW port;" -t -A)
  DATABASE_URL="postgresql://${PGUSER}:${PGPASSWORD}@localhost:${DATABASE_PORT}/${PGDATABASE}?host=${PGHOST}"
  export DATABASE_URL
  export PG_VERSION

  # Execute tests using cargo
  echo "Running tests..."
  cargo pgrx test "pg$PG_VERSION"
}

# Loop over PostgreSQL versions
for PG_VERSION in "${PG_VERSIONS[@]}"; do
  echo ""
  echo "***********************************************************"
  echo "* Running tests ($FLAG_PROCESS_TYPE) for PostgreSQL version: $PG_VERSION"
  echo "***********************************************************"
  echo ""

  # Install the specific PostgreSQL version if it's not already installed
  case "$OS_NAME" in
    Darwin)
      brew install postgresql@"$PG_VERSION"  2>&1
      ;;
    Linux)
      sudo apt-get install -y "postgresql-$PG_VERSION" "postgresql-server-dev-$PG_VERSION"  2>&1
      ;;
  esac

  if [ "$FLAG_PROCESS_TYPE" = "threaded" ]; then
    run_tests &
  else
    run_tests
  fi
done

# Wait for all child processes to finish
wait

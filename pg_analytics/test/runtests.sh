#!/bin/bash

# This script runs integration tests on the pg_analytics extension using pg_regress. To add new tests, add
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
  echo " -v (optional),   PG version(s) separated by comma <12,13,14>"
  echo " -u (optional),   Version to test upgrading to before running tests (only meant for use in CI) <0.3.7>"
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

# Assign flags to vars and check
while getopts "hp:v:u:" flag
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
    *)
      usage
      ;;
  esac
done

# Determine the base directory of the script
BASEDIR=$(dirname "$0")
cd "$BASEDIR/../"
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


# Cleanup function
cleanup() {
  # Check if regression.diffs exists and print if present
  if [ -f "$BASEDIR/regression.diffs" ]; then
    echo "Some test(s) failed! Printing the diff between the expected and actual test results..."
    cat "$BASEDIR/regression.diffs"
  fi

  # Clean up the test database and temporary files
  echo "Cleaning up..."
  "$PG_BIN_PATH/dropdb" test_db
  "$PG_BIN_PATH/pg_ctl" stop -m i
  rm -rf "$PWFILE"
  rm -rf "$TMPDIR"
  rm -rf "$BASEDIR/test/test_logs.log"
  rm -rf "$BASEDIR/regression.diffs"
  rm -rf "$BASEDIR/regression.out"

  # Once the tests are done, we reset the pgrx environment to use the project's default, since we
  # can only keep one "version" of `cargo pgrx init` in the pgrx environment at a time (for local development)
  default_pg_version="$(grep 'default' Cargo.toml | cut -d'[' -f2 | tr -d '[]" ' | grep -o '[0-9]\+')"
  if [[ ${PG_VERSIONS[*]} =~ $default_pg_version ]]; then
    echo "Resetting pgrx environment to use default version: $default_pg_version..."
    case "$OS_NAME" in
      Darwin)
        # Check arch to set proper pg_config path
        if [ "$(uname -m)" = "arm64" ]; then
          cargo pgrx init "--pg$default_pg_version=/opt/homebrew/opt/postgresql@$default_pg_version/bin/pg_config"
        elif [ "$(uname -m)" = "x86_64" ]; then
          cargo pgrx init "--pg$default_pg_version=/usr/local/opt/postgresql@$default_pg_version/bin/pg_config"
        else
          echo "Unknown arch, exiting..."
          exit 1
        fi
        ;;
      Linux)
        cargo pgrx init "--pg$default_pg_version=/usr/lib/postgresql/$default_pg_version/bin/pg_config"
        ;;
    esac
  fi
  echo "Done, goodbye!"
}

# Register the cleanup function to run when the script exits
trap cleanup EXIT


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
        # For some reason, the path structure is different specifically for PostgreSQL 14 on macOS
        if [ "$PG_VERSION" = "14" ]; then
          REGRESS="/opt/homebrew/opt/postgresql@$PG_VERSION/lib/postgresql@$PG_VERSION/pgxs/src/test/regress/pg_regress"
        else
          REGRESS="/opt/homebrew/opt/postgresql@$PG_VERSION/lib/postgresql/pgxs/src/test/regress/pg_regress"
        fi
      elif [ "$(uname -m)" = "x86_64" ]; then
        PG_BIN_PATH="/usr/local/opt/postgresql@$PG_VERSION/bin"
        # For some reason, the path structure is different specifically for PostgreSQL 14 on macOS
        if [ "$PG_VERSION" = "14" ]; then
          REGRESS="/usr/local/opt/postgresql@$PG_VERSION/lib/postgresql@$PG_VERSION/pgxs/src/test/regress/pg_regress"
        else
          REGRESS="/usr/local/opt/postgresql@$PG_VERSION/lib/postgresql/pgxs/src/test/regress/pg_regress"
        fi
      else
        echo "Unknown arch, exiting..."
        exit 1
      fi
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

  # Configure pgrx to use system PostgreSQL
  echo "Initializing pgrx environment..."
  cargo pgrx init "--pg$PG_VERSION=$PG_BIN_PATH/pg_config"

  # This block runs a test whether our extension can upgrade to the current version, and then runs our integrationg tests
  if [ -n "$FLAG_UPGRADE_VER" ]; then
    echo "Running extension upgrade test..."
    # Don't send telemetry when running tests
    export TELEMETRY=false

    # First, download & install the first release at which we started supporting upgrades for Postgres 16 (v0.5.2)
    BASE_RELEASE="0.5.2"
    DOWNLOAD_URL="https://github.com/paradedb/paradedb/releases/download/v$BASE_RELEASE/pg_analytics-v$BASE_RELEASE-pg$PG_VERSION-amd64-ubuntu2204.deb"
    curl -LOJ "$DOWNLOAD_URL"
    sudo dpkg -i "pg_analytics-v$BASE_RELEASE-pg$PG_VERSION-amd64-ubuntu2204.deb"

    # Second, load the extension into the test database
    echo "Loading pg_analytics extension version v$BASE_RELEASE into the test database..."
    "$PG_BIN_PATH/psql" -v ON_ERROR_STOP=1 -c "CREATE EXTENSION pg_analytics VERSION '$BASE_RELEASE';" -d test_db

    # Third, build & install the current version of the extension
    echo "Building & installing the current version of the pg_analytics extension..."
    sudo chown -R "$(whoami)" "/usr/share/postgresql/$PG_VERSION/extension/" "/usr/lib/postgresql/$PG_VERSION/lib/"
    cargo pgrx install --pg-config="$PG_BIN_PATH/pg_config" --release

    # Fourth, upgrade the extension installed on the test database to the current version
    "$PG_BIN_PATH/psql" -v ON_ERROR_STOP=1 -c "ALTER EXTENSION pg_analytics UPDATE TO '$FLAG_UPGRADE_VER';" -d test_db
  else
    # Use cargo-pgx to install the extension for the specified version
    echo "Installing pg_analytics extension onto the test database..."
    cargo pgrx install --pg-config="$PG_BIN_PATH/pg_config" --profile dev
  fi

  # Get a list of all tests
  while IFS= read -r line; do
    TESTS+=("$line")
  done < <(find "${BASEDIR}/test/sql" -type f -name "*.sql" -exec basename {} \; | sed -e 's/\..*$//' | sort)

  # Execute the fixtures to create the test data
  echo "Loading test data..."
  "$PG_BIN_PATH/psql" -v ON_ERROR_STOP=1 -f "${BASEDIR}/test/fixtures.sql" -d test_db

  # Execute tests using pg_regress
  echo "Running tests..."
  ${REGRESS} --use-existing --dbname=test_db --inputdir="${BASEDIR}/test/" "${TESTS[@]}"
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

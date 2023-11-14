#!/bin/bash

# This script runs integration tests on the pg_sparse extension using pg_regress. To add new tests, add
# a new .sql file to the test/sql directory and add the corresponding .out file to the test/expected
# directory, and it will automatically get executed by this script.

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

OS_NAME=$(uname)
TESTDIR="$(dirname "$0")"
export PGUSER=postgres
export PGDATABASE=postgres
export PGPASSWORD=password

# Set the directory to output PostgreSQL logs to
CURRENT_DIR_NAME=$(basename "$(pwd)")
if [[ $CURRENT_DIR_NAME != *test* ]]; then
  LOG_DIR="$(pwd)/test"
else
  LOG_DIR="$(pwd)"
fi

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
  trap '$PG_BIN_PATH/pg_ctl stop -m i; rm -f "$PWFILE"' sigint sigterm exit  # <-- Also remove the password file on exit
  rm -rf "$TMPDIR"
  unset TESTS

  # Initialize the test database
  echo "Initializing PostgreSQL test database..."
  "$PG_BIN_PATH/initdb" --no-locale --encoding=UTF8 --nosync -U "$PGUSER" --auth-local=md5 --auth-host=md5 --pwfile="$PWFILE" > /dev/null
  "$PG_BIN_PATH/pg_ctl" start -o "-F -c listen_addresses=\"\" -c log_min_messages=WARNING -k $PGDATA" > /dev/null
  "$PG_BIN_PATH/createdb" test_db

  # Set PostgreSQL logging configuration
  echo "Setting test database logging configuration..."
  "$PG_BIN_PATH/psql" -v ON_ERROR_STOP=1 -c "ALTER SYSTEM SET logging_collector TO 'on';" -d test_db > /dev/null
  "$PG_BIN_PATH/psql" -v ON_ERROR_STOP=1 -c "ALTER SYSTEM SET log_directory TO '$LOG_DIR';" -d test_db > /dev/null
  "$PG_BIN_PATH/psql" -v ON_ERROR_STOP=1 -c "ALTER SYSTEM SET log_filename TO 'test_logs.log';" -d test_db > /dev/null

  # Configure search_path to include the paradedb schema
  "$PG_BIN_PATH/psql" -v ON_ERROR_STOP=1 -c "ALTER USER $PGUSER SET search_path TO public,paradedb;" -d test_db > /dev/null

  # Reload PostgreSQL configuration
  echo "Reloading PostgreSQL configuration..."
  "$PG_BIN_PATH/pg_ctl" restart > /dev/null

  # Set permissions on the extension directory so that we can install the extension
  sudo chown -R "$(whoami)" "/usr/share/postgresql/$PG_VERSION/extension/" "/usr/lib/postgresql/$PG_VERSION/lib/"

  # Configure pgrx to use system PostgreSQL
  echo "Initializing pgrx environment..."
  cargo pgrx init "--pg$PG_VERSION=$PG_BIN_PATH/pg_config" > /dev/null

  # This block runs a test whether our extension can upgrade to the current version, and then runs our integrationg tests
  if [ -n "$FLAG_UPGRADE_VER" ]; then
    # echo "Running extension upgrade test..."
    # # Don't send telemetry when running tests
    # export TELEMETRY=false

    # # TODO: Figure out how we want to do versioning...

    # # First, download & install the first release at which we started supporting upgrades (v0.5.1)
    # BASE_RELEASE="0.5.1"
    # DOWNLOAD_URL="https://github.com/paradedb/paradedb/releases/download/v$BASE_RELEASE/pg_sparse-v$BASE_RELEASE-pg$PG_VERSION-amd64-linux-gnu.deb"
    # curl -LOJ "$DOWNLOAD_URL" > /dev/null
    # sudo dpkg -i "pg_sparse-v$BASE_RELEASE-pg$PG_VERSION-amd64-linux-gnu.deb" > /dev/null

    # # Second, load the extension into the test database
    # echo "Loading pg_sparse extension version v$BASE_RELEASE into the test database..."
    # "$PG_BIN_PATH/psql" -v ON_ERROR_STOP=1 -c "CREATE EXTENSION pg_sparse VERSION '$BASE_RELEASE';" -d test_db

    # # Third, build & install the current version of the extension
    # echo "Building & installing the current version of the pg_sparse extension..."
    # cargo pgrx install --pg-config="$PG_BIN_PATH/pg_config" --release

    # # Fourth, upgrade the extension installed on the test database to the current version
    # "$PG_BIN_PATH/psql" -v ON_ERROR_STOP=1 -c "ALTER EXTENSION pg_sparse UPDATE TO '$FLAG_UPGRADE_VER';" -d test_db
  else
    # Use cargo-pgx to install the extension for the specified version
    echo "Installing pg_sparse extension onto the test database..."
    make
    make install
  fi

  # Get a list of all tests
  while IFS= read -r line; do
    TESTS+=("$line")
  done < <(find "${TESTDIR}/sql" -type f -name "*.sql" -exec basename {} \; | sed -e 's/\..*$//' | sort)

  # Execute the fixtures to create the test data
  echo "Loading test data..."
  "$PG_BIN_PATH/psql" -v ON_ERROR_STOP=1 -f "${TESTDIR}/fixtures.sql" -d test_db > /dev/null

  # Execute tests using pg_regress
  echo "Running tests..."
  ${REGRESS} --use-existing --dbname=test_db --inputdir="${TESTDIR}" "${TESTS[@]}"

  # Uncomment this to display test ERROR logs if you need to debug. Note that many of these errors are
  # expected, since we are testing error handling/invalid cases in our regression tests.
  # echo "Displaying PostgreSQL ERROR logs from tests..."
  # grep "ERROR" "$LOG_DIR/test_logs.log"
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
      brew install postgresql@"$PG_VERSION" > /dev/null 2>&1
      ;;
    Linux)
      sudo apt-get install -y "postgresql-$PG_VERSION" "postgresql-server-dev-$PG_VERSION" > /dev/null 2>&1
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
# Once the tests are done, we reset the pgrx environment to use the project's default, since we
# can only keep one "version" of `cargo pgrx init` in the pgrx environment at a time (for local development)
default_pg_version="$(grep 'default' Cargo.toml | cut -d'[' -f2 | tr -d '[]" ' | grep -o '[0-9]\+')"
if [[ ${PG_VERSIONS[*]} =~ $default_pg_version ]]; then
  case "$OS_NAME" in
    Darwin)
      # Check arch to set proper pg_config path
      if [ "$(uname -m)" = "arm64" ]; then
        cargo pgrx init "--pg$default_pg_version=/opt/homebrew/opt/postgresql@$default_pg_version/bin/pg_config" > /dev/null
      elif [ "$(uname -m)" = "x86_64" ]; then
        cargo pgrx init "--pg$default_pg_version=/usr/local/opt/postgresql@$default_pg_version/bin/pg_config" > /dev/null
      else
        echo "Unknown arch, exiting..."
        exit 1
      fi
      ;;
    Linux)
      cargo pgrx init "--pg$default_pg_version=/usr/lib/postgresql/$default_pg_version/bin/pg_config" > /dev/null
      ;;
  esac
fi

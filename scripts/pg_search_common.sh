#!/bin/bash

# pg_search_common.sh - Common setup code for pg_search scripts
#
# This script contains common functionality for setting up a PostgreSQL environment
# for the pg_search extension. It is meant to be sourced by other scripts.
#
# Possible PostgreSQL version values:
#  - 15.19
#  - 16.15
#  - 17.11
#  - 18.6 (default)
#
# After sourcing, the following are available:
#  - BUILD_PARAMS: array of build flags (--release, --profile <value>)
#  - EXTRA_ARGS: array of remaining arguments (everything else)
#  - DATABASE_URL: connection string for the running database
#  - PG_CONFIG: path to pg_config binary

set -Eeuo pipefail

# Parse arguments: separate build flags from the rest
BUILD_PARAMS=()
EXTRA_ARGS=()

i=1
while [ "$i" -le "$#" ]; do
  arg="${!i}"
  if [ "$arg" = "--release" ]; then
    BUILD_PARAMS+=("--release")
  elif [ "$arg" = "--profile" ]; then
    i=$((i + 1))
    if [ "$i" -le "$#" ]; then
      PROFILE_VALUE="${!i}"
      if [[ "$PROFILE_VALUE" == --* ]]; then
        echo "Error: --profile requires a value"
        exit 1
      fi
      BUILD_PARAMS+=("--profile" "${PROFILE_VALUE}")
    else
      echo "Error: --profile requires a value"
      exit 1
    fi
  else
    EXTRA_ARGS+=("$arg")
  fi
  i=$((i + 1))
done

# Get the directory where this script is located
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

# Change to pg_search directory
cd "${SCRIPT_DIR}/../pg_search"

# Set PostgreSQL version or use default 18.6
PGVER=${PGVER:-18.6}

# Extract major version and set port and feature flag
BASEVER=${PGVER%%.*}
PORT=288${BASEVER} # pgrx prefixes the PostgreSQL major version with 288 (e.g., 28818)
FEATURE=pg${BASEVER} # Feature flag (e.g., pg18)

# Enable command echo for debugging the setup steps below. It is disabled again
# at the end of the script so it does not leak into the sourcing script's own
# commands (psql, cargo test).
set -x

# Stop any existing pgrx server with this feature
cargo pgrx stop "${FEATURE}" --package pg_search

# Install pg_search extension, conditionally using --release
cargo pgrx install --package pg_search ${BUILD_PARAMS[@]+"${BUILD_PARAMS[@]}"} --pg-config "${HOME}/.pgrx/${PGVER}/pgrx-install/bin/pg_config"

# Start the PostgreSQL server with the installed extension
RUST_BACKTRACE=1 cargo pgrx start "${FEATURE}" --package pg_search

# Create a new database for testing
createdb -h localhost -p "${PORT}" pg_search || true

# Set database connection URL
export DATABASE_URL="postgresql://${USER}@localhost:${PORT}/pg_search"

# Clean up any previous logs
rm -rf /tmp/ephemeral_postgres_logs/*

# Export the PG_CONFIG variable for use by sourcing scripts
export PG_CONFIG="${HOME}/.pgrx/${PGVER}/pgrx-install/bin/pg_config"

# Setup is done, stop echoing so the sourcing script's commands run quietly
set +x

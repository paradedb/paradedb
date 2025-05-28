#!/bin/bash

# pg_search_common.sh - Common setup code for pg_search scripts
#
# This script contains common functionality for setting up a PostgreSQL environment
# for the pg_search extension. It is meant to be sourced by other scripts.
#
# Possible PostgreSQL version values:
#  - 14.17
#  - 15.12
#  - 16.8
#  - 17.4 (default)

set -Eeuo pipefail

# Parse arguments for the --release flag
BUILD_PARAMS=("--features=icu")

# Loop through arguments
i=1
while [ $i -le $# ]; do
  arg="${!i}"
  if [ "$arg" = "--release" ]; then
    BUILD_PARAMS=("${BUILD_PARAMS[@]}" "--release")
  elif [ "$arg" = "--profile" ]; then
    i=$((i+1))
    if [ $i -le $# ]; then
      PROFILE_VALUE="${!i}"
      # Check if the next argument is another flag
      if [[ "$PROFILE_VALUE" == --* ]]; then
        echo "Error: --profile requires a value"
        exit 1
      fi
      BUILD_PARAMS=("${BUILD_PARAMS[@]}" "--profile" "${PROFILE_VALUE}")
    else
      echo "Error: --profile requires a value"
      exit 1
    fi
  fi
  i=$((i+1))
done

# Get the directory where this script is located
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)

# Change to pg_search directory
cd "${SCRIPT_DIR}/../pg_search" || exit 1

# Set PostgreSQL version or use default 17.4
PGVER=${PGVER:-17.4}

# Extract major version and set port and feature flag
BASEVER=$(echo "${PGVER}" | cut -f1 -d.)
PORT=288${BASEVER}  # Port 2880 + major version (e.g., 28817 for version 17.4)
FEATURE=pg${BASEVER}  # Feature flag (e.g., pg17)

# Enable command echo for debugging
set -x

# Stop any existing pgrx server with this feature
cargo pgrx stop "${FEATURE}" --package pg_search

# Install pg_search extension with ICU support, conditionally using --release
cargo pgrx install --package pg_search "${BUILD_PARAMS[@]}" --pg-config "${HOME}/.pgrx/${PGVER}/pgrx-install/bin/pg_config" || exit $? # ksh88: there's a space between --profile and the value

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

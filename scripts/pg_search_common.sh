#! /bin/sh

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

# Get the directory where this script is located
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)

# Change to pg_search directory
cd ${SCRIPT_DIR}/../pg_search

# Set PostgreSQL version or use default 17.4
PGVER=${PGVER:-17.4}

# Extract major version and set port and feature flag
BASEVER=$(echo ${PGVER} | cut -f1 -d.)
PORT=288${BASEVER}  # Port 2880 + major version (e.g., 28817 for version 17.4)
FEATURE=pg${BASEVER}  # Feature flag (e.g., pg17)

# Enable command echo for debugging
set -x

# Stop any existing pgrx server with this feature
cargo pgrx stop $FEATURE --package pg_search

# Install pg_search extension with ICU support
cargo pgrx install --package pg_search --features=icu --pg-config ~/.pgrx/${PGVER}/pgrx-install/bin/pg_config || exit $?

# Start the PostgreSQL server with the installed extension
RUST_BACKTRACE=1 cargo pgrx start $FEATURE --package pg_search

# Create a new database for testing
createdb -h localhost -p ${PORT} pg_search

# Set database connection URL
export DATABASE_URL=postgresql://${USER}@localhost:${PORT}/pg_search

# Clean up any previous logs
rm -rf /tmp/ephemeral_postgres_logs/*

# Export the PG_CONFIG variable for use by sourcing scripts
export PG_CONFIG="~/.pgrx/${PGVER}/pgrx-install/bin/pg_config"

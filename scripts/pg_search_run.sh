#!/bin/bash

# pg_search_run.sh - Script to install, start and interact with pg_search extension
#
# This script manages the PostgreSQL environment for running pg_search extension.
# It installs the extension, starts a PostgreSQL server, creates a test database,
# and connects to it with psql.
#
# Usage:
#   PGVER=<version> ./pg_search_run.sh [psql arguments]
#   Example: PGVER=17.4 ./pg_search_run.sh psql -c "SELECT 1"

# Source the common setup script
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=./pg_search_common.sh
source "${SCRIPT_DIR}/pg_search_common.sh"

# Connect to the database with psql and pass any additional arguments
psql "${DATABASE_URL}" "$@"

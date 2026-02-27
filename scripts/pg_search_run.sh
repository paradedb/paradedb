#!/bin/bash

# pg_search_run.sh - Script to install, start and interact with pg_search extension
#
# This script manages the PostgreSQL environment for running pg_search extension.
# It installs the extension, starts a PostgreSQL server, creates a test database,
# and connects to it with psql.
#
# Usage:
#   PGVER=<version> ./pg_search_run.sh [--release] [psql arguments]
#   Example: PGVER=17.4 ./pg_search_run.sh --release psql -c "SELECT 1"

CURRENT_DIR=$(pwd)

set -Eeuo pipefail

# Get script directory
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)

# Source the common setup script — it parses args into BUILD_PARAMS and EXTRA_ARGS
# shellcheck source=./scripts/pg_search_common.sh
source "${SCRIPT_DIR}/pg_search_common.sh" "$@"

cd "${CURRENT_DIR}"

# Connect to the database with psql and pass any non-build arguments
psql "${DATABASE_URL}" "${EXTRA_ARGS[@]+"${EXTRA_ARGS[@]}"}"

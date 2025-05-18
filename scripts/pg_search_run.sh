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

# Extract --release flag if present
COMMON_ARGS=()
for arg in "$@"; do
  if [ "$arg" = "--release" ]; then
    COMMON_ARGS+=("$arg")
  fi
done

# Source the common setup script with appropriate arguments
# shellcheck source=./scripts/pg_search_common.sh
source "${SCRIPT_DIR}/pg_search_common.sh" "${COMMON_ARGS[@]+"${COMMON_ARGS[@]}"}"

cd "${CURRENT_DIR}"

# Connect to the database with psql and pass any additional arguments
psql "${DATABASE_URL}" "$@"

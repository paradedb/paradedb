#!/bin/bash

# pg_search_test.sh - Script to install, start and run tests for pg_search extension
#
# This script sets up a PostgreSQL environment for testing pg_search extension.
# It installs the extension, starts a PostgreSQL server, creates a test database,
# and runs the test suite.
#
# Usage:
#   PGVER=<version> ./pg_search_test.sh [--release] [--test test_name]
#   Example: PGVER=17.4 ./pg_search_test.sh --release --test sorting

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

# Run the test suite with backtrace enabled and pass along all arguments
RUST_BACKTRACE=1 cargo test --package tests --package tokenizers --features=icu "$@"

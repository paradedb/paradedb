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

# Source the common setup script — it parses args into BUILD_PARAMS and EXTRA_ARGS
# shellcheck source=./scripts/pg_search_common.sh
source "${SCRIPT_DIR}/pg_search_common.sh" "$@"

# Run the test suite with backtrace enabled and pass any non-build arguments
RUST_BACKTRACE=1 cargo test --package tests --package tokenizers "${EXTRA_ARGS[@]+"${EXTRA_ARGS[@]}"}"

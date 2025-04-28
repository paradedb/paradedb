#! /bin/sh

# pg_search_test.sh - Script to install, start and run tests for pg_search extension
#
# This script sets up a PostgreSQL environment for testing pg_search extension.
# It installs the extension, starts a PostgreSQL server, creates a test database,
# and runs the test suite.
#
# Usage:
#   PGVER=<version> ./pg_search_test.sh [--test test_name]
#   Example: PGVER=17.4 ./pg_search_test.sh --test sorting

# Source the common setup script
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
. ${SCRIPT_DIR}/pg_search_common.sh

# Run the test suite with backtrace enabled and pass along all arguments
RUST_BACKTRACE=1 cargo test --package tests --package tokenizers --features=icu $* -- $TEST_ARGS

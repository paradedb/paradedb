#!/bin/bash

# This script installs the pgvector extension (from a specified tag) and the pg_bm25
# extension (from the current commit) within your pgrx environment, since these
# are required for developing the pg_search extension. Note that you need to run this
# script every time you make changes to the pg_bm25 extension or pgvector releases a
# new version which you'd like reflected while developing the pg_search extension.

# Exit on subcommand errors
set -Eeuo pipefail

# The pgvector version to install
PGVECTOR_VERSION="v0.5.0"

# All pgrx-supported PostgreSQL versions to configure for
OS_NAME=$(uname)
if [ $# -eq 0 ]; then
  # No arguments provided; use default versions
  case "$OS_NAME" in
    Darwin)
      PG_VERSIONS=("15.4" "14.9" "13.12" "12.16" "11.21")
      ;;
    Linux)
      PG_VERSIONS=("15" "14" "13" "12" "11")
      ;;
  esac
else
  IFS=',' read -ra PG_VERSIONS <<< "$1"  # Split the argument by comma into an array
fi

echo "Installing pgvector and pg_bm25 into your pgrx environment..."

# Clone pgvector if it doesn't exist
if [ ! -d "pgvector/" ]; then
  echo "Cloning pgvector..."
  git clone https://github.com/pgvector/pgvector.git pgvector/
fi

echo "Installing pgvector..."
cd pgvector/
git fetch --tags
git checkout $PGVECTOR_VERSION

# Install pgvector for all specified pgrx-compatible PostgreSQL versions
for version in "${PG_VERSIONS[@]}"; do
  echo "Installing pgvector for pgrx PostgreSQL $version..."
  case "$OS_NAME" in
    Darwin)
      make clean
      PG_CONFIG="$HOME/.pgrx/$version/pgrx-install/bin/pg_config" make && PG_CONFIG="$HOME/.pgrx/$version/pgrx-install/bin/pg_config" make install
      ;;
    Linux)
      sudo make clean
      sudo PG_CONFIG="/usr/lib/postgresql/$version/bin/pg_config" make && sudo PG_CONFIG="/usr/lib/postgresql/$version/bin/pg_config" make install
      ;;
  esac
done

echo "Installing pg_bm25..."
cd ../../pg_bm25

# Build and install pg_bm25 into the pgrx environment
for version in "${PG_VERSIONS[@]}"; do
  echo "Installing pg_bm25 for pgrx PostgreSQL $version..."
  cargo clean
  case "$OS_NAME" in
    Darwin)
      cargo pgrx install --pg-config="$HOME/.pgrx/$version/pgrx-install/bin/pg_config"
      ;;
    Linux)
      cargo pgrx install --pg-config="/usr/lib/postgresql/$version/bin/pg_config"
      ;;
  esac
done

echo "Done! You can now develop pg_search by running 'cargo pgrx run'!"

#!/bin/bash

# This script installs the pgvector extension (from a specified tag) and the pg_bm25
# extension (from the current commit) within your pgrx environment, since these
# are required for developing the pg_search extension. Note that you need to run this
# script every time you make changes to the pg_bm25 extension or pgvector releases a
# new version which you'd like reflected while developing the pg_search extension.

# Exit on subcommand errors
set -Eeuo pipefail

OS_NAME=$(uname)
CONFIGDIR="$(dirname "$0")"
PGVECTOR_VERSION=v$(jq -r '.extensions.pgvector.version' "$CONFIGDIR/../conf/third_party_pg_extensions.json")

# All pgrx-supported PostgreSQL versions to configure for
if [ $# -eq 0 ]; then
  # No arguments provided; use default versions
  case "$OS_NAME" in
    Darwin)
      PG_VERSIONS=("16.1" "15.5" "14.10" "13.13" "12.17")
      ;;
    Linux)
      PG_VERSIONS=("16" "15" "14" "13" "12")
      ;;
  esac
else
  IFS=',' read -ra PG_VERSIONS <<< "$1"  # Split the argument by comma into an array
fi

echo "Installing pgvector and pg_bm25 into your pgrx environment..."
echo ""

# Clone pgvector if it doesn't exist
if [ ! -d "pgvector/" ]; then
  echo "Cloning pgvector..."
  git clone https://github.com/pgvector/pgvector.git pgvector/
fi

echo "Installing pgvector..."
echo ""
cd pgvector/
git fetch --tags
git checkout "$PGVECTOR_VERSION"

# Install pgvector for all specified pgrx-compatible PostgreSQL versions. We compile
# pgvector without specifying PG_CONFIG, so that it won't redefine macros that are
# already defined in the pgrx environment, but we specify PG_CONFIG when installing
# pgvector to make it available to the pgrx environment at runtime.
for version in "${PG_VERSIONS[@]}"; do
  echo "Installing pgvector for pgrx PostgreSQL $version..."
  case "$OS_NAME" in
    Darwin)
      make clean
      make && make install PG_CONFIG="/opt/homebrew/opt/postgresql@$version/bin/pg_config"      
      ;;
    Linux)
      sudo make clean
      sudo make && sudo PG_CONFIG="/usr/lib/postgresql/$version/bin/pg_config" make install
      ;;
  esac
done

echo ""
echo "Installing pg_bm25..."
cd "$CONFIGDIR/../../pg_bm25"

# Build and install pg_bm25 into the pgrx environment
for version in "${PG_VERSIONS[@]}"; do
  echo "Installing pg_bm25 for pgrx PostgreSQL $version..."
  case "$OS_NAME" in
    Darwin)
      cargo pgrx install --pg-config="/opt/homebrew/opt/postgresql@$version/bin/pg_config" --profile dev
      ;;
    Linux)
      cargo pgrx install --pg-config="/usr/lib/postgresql/$version/bin/pg_config" --profile dev
      ;;
  esac
done

echo "Done! You can now develop pg_search by running 'cargo pgrx run'!"

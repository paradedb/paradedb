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
  PG_VERSIONS=("16" "15" "14" "13" "12")
else
  IFS=',' read -ra PG_VERSIONS <<< "$1"  # Split the argument by comma into an array
fi

echo "Installing system PostgreSQL..."
echo ""

# We install, if necessary, all supported PostgreSQL versions into the system
for version in "${PG_VERSIONS[@]}"; do
  echo "Installing PostgreSQL $version..."
  case "$OS_NAME" in
    Darwin)
      brew install postgresql@"$version" > /dev/null 2>&1
      ;;
    Linux)
      sudo apt-get install -y "postgresql-$version" "postgresql-server-dev-$version" > /dev/null 2>&1
      ;;
  esac
done

echo "Installing pgvector and pg_bm25 into your system PostgreSQL environment..."
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
  echo "Installing pgvector for PostgreSQL $version..."
  case "$OS_NAME" in
    Darwin)
      make clean
      # Check arch to set proper pg_config path
      if [ "$(uname -m)" = "arm64" ]; then
        make PG_CONFIG="/opt/homebrew/opt/postgresql@$version/bin/pg_config"
        make install PG_CONFIG="/opt/homebrew/opt/postgresql@$version/bin/pg_config"
      elif [ "$(uname -m)" = "x86_64" ]; then
        make PG_CONFIG="/usr/local/opt/postgresql@$version/bin/pg_config"
        make install PG_CONFIG="/usr/local/opt/postgresql@$version/bin/pg_config"
      else
        echo "Unknown arch, exiting..."
        exit 1
      fi
      ;;
    Linux)
      sudo make clean
      sudo PG_CONFIG="/usr/lib/postgresql/$version/bin/pg_config" make
      sudo PG_CONFIG="/usr/lib/postgresql/$version/bin/pg_config" make install
      ;;
  esac
done

echo ""
echo "Installing pg_sparse..."
echo ""
cd "$CONFIGDIR/../../pg_sparse"

# Build and install pg_sparse into the pgrx environment
for version in "${PG_VERSIONS[@]}"; do
  echo "Installing pg_sparse for PostgreSQL $version..."
  case "$OS_NAME" in
    Darwin)
      make clean
      # Check arch to set proper pg_config path
      if [ "$(uname -m)" = "arm64" ]; then
        make PG_CONFIG="/opt/homebrew/opt/postgresql@$version/bin/pg_config"
        make install PG_CONFIG="/opt/homebrew/opt/postgresql@$version/bin/pg_config"
      elif [ "$(uname -m)" = "x86_64" ]; then
        make PG_CONFIG="/usr/local/opt/postgresql@$version/bin/pg_config"
        make install PG_CONFIG="/usr/local/opt/postgresql@$version/bin/pg_config"
      else
        echo "Unknown arch, exiting..."
        exit 1
      fi
      ;;
    Linux)
      sudo make clean
      sudo PG_CONFIG="/usr/lib/postgresql/$version/bin/pg_config" make
      sudo PG_CONFIG="/usr/lib/postgresql/$version/bin/pg_config" make install
      ;;
  esac
done

echo ""
echo "Installing pg_bm25..."
cd "$CONFIGDIR/../../pg_bm25"

# Build and install pg_bm25 into the pgrx environment
for version in "${PG_VERSIONS[@]}"; do
  echo "Installing pg_bm25 for PostgreSQL $version..."
  case "$OS_NAME" in
    Darwin)
      # Check arch to set proper pg_config path
      if [ "$(uname -m)" = "arm64" ]; then
        cargo pgrx init "--pg$version=/opt/homebrew/opt/postgresql@$version/bin/pg_config" > /dev/null
        cargo pgrx install --pg-config="/opt/homebrew/opt/postgresql@$version/bin/pg_config" --profile dev
      elif [ "$(uname -m)" = "x86_64" ]; then
        cargo pgrx init "--pg$version=/usr/local/opt/postgresql@$version/bin/pg_config" > /dev/null
        cargo pgrx install --pg-config="/usr/local/opt/postgresql@$version/bin/pg_config" --profile dev
      else
        echo "Unknown arch, exiting..."
        exit 1
      fi
      ;;
    Linux)
      cargo pgrx init "--pg$version=/usr/lib/postgresql/$version/bin/pg_config"
      cargo pgrx install --pg-config="/usr/lib/postgresql/$version/bin/pg_config" --profile dev
      ;;
  esac
done

# We can only keep one "version" of `cargo pgrx init` in the pgrx environment at a time, so we make one final call to
# `cargo pgrx init` to load the project's default pgrx PostgreSQL version (for local development)
default_pg_version="$(grep 'default' Cargo.toml | cut -d'[' -f2 | tr -d '[]" ' | grep -o '[0-9]\+')"
if [[ ${PG_VERSIONS[*]} =~ $default_pg_version ]]; then
  case "$OS_NAME" in
    Darwin)
      # Check arch to set proper pg_config path
      if [ "$(uname -m)" = "arm64" ]; then
        cargo pgrx init "--pg$default_pg_version=/opt/homebrew/opt/postgresql@$default_pg_version/bin/pg_config" > /dev/null
      elif [ "$(uname -m)" = "x86_64" ]; then
        cargo pgrx init "--pg$default_pg_version=/usr/local/opt/postgresql@$default_pg_version/bin/pg_config" > /dev/null
      else
        echo "Unknown arch, exiting..."
        exit 1
      fi
      ;;
    Linux)
      cargo pgrx init "--pg$default_pg_version=/usr/lib/postgresql/$default_pg_version/bin/pg_config" > /dev/null
      ;;
  esac
fi

echo "Done! You can now develop pg_search by running 'cargo pgrx run'!"

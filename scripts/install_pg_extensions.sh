#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Function to install a single PostgreSQL extension
# Usage:
# install_pg_extension <name> <version> <url>
# Example:
# install_pg_extension "pg_cron" "1.0.0" "https://github.com/citusdata/pg_cron/archive/refs/tags/v1.0.0.tar.gz"
install_pg_extension() {
  local PG_EXTENSION_NAME=$1
  local PG_EXTENSION_VERSION=$2
  local PG_EXTENSION_URL=$3

  # Download & extract source code
  curl -L "$PG_EXTENSION_URL" -o "/tmp/$PG_EXTENSION_NAME.tar.gz"
  tar -xvf "/tmp/$PG_EXTENSION_NAME.tar.gz" -C /tmp
  cd "/tmp/$PG_EXTENSION_NAME-$PG_EXTENSION_VERSION"

  # Build and package as a .deb
  if [ "$PG_EXTENSION_NAME" == "postgis" ]; then
    ./autogen.sh
    ./configure
  elif [ "$PG_EXTENSION_NAME" == "pgrouting" ]; then
    mkdir build
    cd build
    cmake ..
  fi
  make "-j$(nproc)" && checkinstall -D --nodoc --install=no --fstrans=no --backup=no --pakdir=/tmp
}

# Iterate over all arguments, which are expected to be comma-separated values of the format NAME,VERSION,URL
for extension in "$@"; do
  IFS=',' read -ra EXTENSION_DETAILS <<< "$extension"
  install_pg_extension "${EXTENSION_DETAILS[0]}" "${EXTENSION_DETAILS[1]}" "${EXTENSION_DETAILS[2]}"
done

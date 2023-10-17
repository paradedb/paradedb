#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail


# Function to reformat a version string to semVer (i.e. x.y.z)
# Example:
# sanitize_version "ver_1.4.8" --> 1.4.8
# sanitize_version "REL15_1_5_0" --> 1.5.0
# sanitize_version "2.3.4" --> 2.3.4
sanitize_version() {
  local VERSION="$1"
  echo "$VERSION" | sed -E 's/[^0-9]*([0-9]+\.[0-9]+\.[0-9]+).*/\1/;s/[^0-9]*[0-9]+_([0-9]+)_([0-9]+)_([0-9]+).*/\1.\2.\3/'
}


# Function to check if a PostgreSQL extension exists on PGXN
# Example:
# check_pgxn_package_exists "pg_bm25" "0.1.0"
check_pgxn_package_exists() {
    local EXTENSION="$1"
    local VERSION="$2"
    
    # Try to fetch the details for the specified version of the extension using pgxnclient
    pgxn info "${EXTENSION}==${VERSION}" &> /dev/null

    # Check the exit status of the pgxnclient command
    if [[ $? -eq 0 ]]; then
        echo "Extension $EXTENSION version $VERSION exists on PGXN."
        return 0
    else
        echo "Extension $EXTENSION version $VERSION does not exist on PGXN."
        return 1
    fi
}

# Function to download and install a PostgreSQL extension from PGXN 
# Example:
# install_pgxn_package_version "pg_bm25" "0.1.0"
install_pgxn_package_version() {
    local EXTENSION="$1"
    local VERSION="$2"
    
    # Download and install the specified version of the extension
    pgxn install "${EXTENSION}==${VERSION}"
    if [[ $? -eq 0 ]]; then
        echo "Extension $EXTENSION version $VERSION installed successfully."
        return 0
    else
        echo "Failed to install extension $EXTENSION version $VERSION."
        return 1
    fi
}

# Function to compile & package a single PostgreSQL extension as a .deb
# Example:
# build_pg_and_package_extension "pg_cron" "1.0.0" "https://github.com/citusdata/pg_cron/archive/refs/tags/v1.0.0.tar.gz"
build_and_package_pg_extension() {
  local PG_EXTENSION_NAME=$1
  local PG_EXTENSION_VERSION=$2
  local PG_EXTENSION_URL=$3

  # Download & extract source code
  mkdir -p "/tmp/$PG_EXTENSION_NAME-$PG_EXTENSION_VERSION"
  curl -L "$PG_EXTENSION_URL" -o "/tmp/$PG_EXTENSION_NAME.tar.gz"
  tar -xvf "/tmp/$PG_EXTENSION_NAME.tar.gz" --strip-components=1 -C "/tmp/$PG_EXTENSION_NAME-$PG_EXTENSION_VERSION"
  cd "/tmp/$PG_EXTENSION_NAME-$PG_EXTENSION_VERSION"

  # Set OPTFLAGS to an empty string if it's not already set
  OPTFLAGS=${OPTFLAGS:-""}

  # Build and package as a .deb
  if [ "$PG_EXTENSION_NAME" == "pgvector" ]; then
    # Disable -march=native to avoid "illegal instruction" errors on macOS arm64 by
    # setting OPTFLAGS to an empty string
    OPTFLAGS=""
  elif [ "$PG_EXTENSION_NAME" == "postgis" ]; then
    ./autogen.sh
    ./configure
  elif [ "$PG_EXTENSION_NAME" == "pgrouting" ]; then
    mkdir build && cd build
    cmake ..
  fi
  make OPTFLAGS="$OPTFLAGS" "-j$(nproc)"
  checkinstall -D --nodoc --install=no --fstrans=no --backup=no --pakdir=/tmp
}

# Function install a PostgreSQL extension either via PGXN or by compiling it from source
# Example:
# install_pg_extension "pg_cron" "1.0.0" "https://github.com/citusdata/pg_cron/archive/refs/tags/v1.0.0.tar.gz"
install_pg_extension() {
  local PG_EXTENSION_NAME=$1
  local PG_EXTENSION_VERSION=$2
  local PG_EXTENSION_URL=$3

  # Checkinstall uses the version in the folder name as the package version, which
  # needs to be semVer compliant, so we sanitize the version first
  SANITIZED_VERSION=$(sanitize_version "$PG_EXTENSION_VERSION")

  # If package exists on PGXN, download and install it
  if check_pgxn_package_exists "$PG_EXTENSION_NAME" "$SANITIZED_VERSION"; then
    echo "Extension $PG_EXTENSION_NAME version $SANITIZED_VERSION exists on PGXN, installing..."
    install_pgxn_package_version "$PG_EXTENSION_NAME" "$SANITIZED_VERSION"
    return 0
  fi

  # Otherwise, we need to compile it from source
  build_and_package_pg_extension "$PG_EXTENSION_NAME" "$SANITIZED_VERSION" "$PG_EXTENSION_URL"
}


# Iterate over all arguments, which are expected to be comma-separated values of the format NAME,VERSION,URL
for EXTENSION in "$@"; do
  IFS=',' read -ra EXTENSION_DETAILS <<< "$EXTENSION"
  install_pg_extension "${EXTENSION_DETAILS[0]}" "${EXTENSION_DETAILS[1]}" "${EXTENSION_DETAILS[2]}"
done

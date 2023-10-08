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


# Function to compile & package a single PostgreSQL extension as a .deb
# Example:
# install_pg_extension "pg_cron" "1.0.0" "https://github.com/citusdata/pg_cron/archive/refs/tags/v1.0.0.tar.gz"
install_pg_extension() {
    local PG_EXTENSION_NAME=$1
    local PG_EXTENSION_VERSION=$2
    local PG_EXTENSION_URL=$3

    # Checkinstall uses the version in the folder name as the package version, which
    # needs to be semVer compliant, so we sanitize the version first
    SANITIZED_VERSION=$(sanitize_version "$PG_EXTENSION_VERSION")

    # Download & extract source code
    mkdir -p "/tmp/$PG_EXTENSION_NAME-$SANITIZED_VERSION"
    curl -L "$PG_EXTENSION_URL" -o "/tmp/$PG_EXTENSION_NAME.tar.gz"
    tar -xvf "/tmp/$PG_EXTENSION_NAME.tar.gz" --strip-components=1 -C "/tmp/$PG_EXTENSION_NAME-$SANITIZED_VERSION"
    cd "/tmp/$PG_EXTENSION_NAME-$SANITIZED_VERSION"

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


# Iterate over all arguments, which are expected to be comma-separated values of the format NAME,VERSION,URL
for EXTENSION in "$@"; do
    IFS=',' read -ra EXTENSION_DETAILS <<< "$EXTENSION"
    install_pg_extension "${EXTENSION_DETAILS[0]}" "${EXTENSION_DETAILS[1]}" "${EXTENSION_DETAILS[2]}"
done

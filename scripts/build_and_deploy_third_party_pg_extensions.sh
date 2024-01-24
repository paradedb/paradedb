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
# build_and_package_pg_extension "pg_cron" "1.0.0" "https://github.com/citusdata/pg_cron/archive/refs/tags/v1.0.0.tar.gz"
build_and_package_pg_extension() {
  local PG_EXTENSION_NAME=$1
  local PG_EXTENSION_VERSION=$2
  local PG_EXTENSION_URL=$3

  # Download & extract source code
  mkdir -p "/tmp/$PG_EXTENSION_NAME-$PG_EXTENSION_VERSION"
  curl -L "$PG_EXTENSION_URL" -o "/tmp/$PG_EXTENSION_NAME.tar.gz"
  tar -xvf "/tmp/$PG_EXTENSION_NAME.tar.gz" --strip-components=1 -C "/tmp/$PG_EXTENSION_NAME-$PG_EXTENSION_VERSION"
  cd "/tmp/$PG_EXTENSION_NAME-$PG_EXTENSION_VERSION"

  # Set pg_config path
  export PG_CONFIG=/usr/lib/postgresql/$PG_MAJOR_VERSION/bin/pg_config

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
    # We need to make the build directory the same name as the extension directory for checkinstall
    mkdir "$PG_EXTENSION_NAME-$PG_EXTENSION_VERSION" && cd "$PG_EXTENSION_NAME-$PG_EXTENSION_VERSION"
    cmake ..
  elif [ "$PG_EXTENSION_NAME" == "citus" ]; then
    ./configure
  fi
  make USE_PGXS=1 OPTFLAGS="$OPTFLAGS" "-j$(nproc)"

  # Create installable package
  mkdir archive
  cp `find . -type f -name "$PG_EXTENSION_NAME*"` archive
  package_dir="$PG_EXTENSION_NAME-$PG_EXTENSION_VERSION-pg$PG_MAJOR_VERSION-$ARCH-ubuntu$UBUNTU_VERSION"

  # Copy files into directory structure
  mkdir -p ${package_dir}/usr/lib/postgresql/lib
  mkdir -p ${package_dir}/var/lib/postgresql/extension
  cp archive/*.so ${package_dir}/usr/lib/postgresql/lib
  cp archive/*.control ${package_dir}/var/lib/postgresql/extension
  cp archive/*.sql ${package_dir}/var/lib/postgresql/extension

  # Symlinks to copy files into directory structure
  mkdir -p ${package_dir}/usr/lib/postgresql/$PG_MAJOR_VERSION/lib
  mkdir -p ${package_dir}/usr/share/postgresql/$PG_MAJOR_VERSION/extension
  cp archive/*.so ${package_dir}/usr/lib/postgresql/$PG_MAJOR_VERSION/lib
  cp archive/*.control ${package_dir}/usr/share/postgresql/$PG_MAJOR_VERSION/extension
  cp archive/*.sql ${package_dir}/usr/share/postgresql/$PG_MAJOR_VERSION/extension

  # Create control file (package name cannot have underscore)
  mkdir -p ${package_dir}/DEBIAN
  touch ${package_dir}/DEBIAN/control
  deb_version=$PG_EXTENSION_VERSION
  CONTROL_FILE="${package_dir}/DEBIAN/control"
  echo "Package: $PG_EXTENSION_NAME" >> $CONTROL_FILE
  echo "Version: ${deb_version}" >> $CONTROL_FILE
  echo "Architecture: $ARCH" >> $CONTROL_FILE
  echo "Maintainer: ParadeDB <support@paradedb.com>" >> $CONTROL_FILE
  echo "Description: ParadeDB Internal Compilation of $PG_EXTENSION_NAME" >> $CONTROL_FILE

  # Create .deb package
  sudo chown -R root:root ${package_dir}
  sudo chmod -R 00755 ${package_dir}
  sudo dpkg-deb --build --root-owner-group ${package_dir}
}


# Function to build & publish a single PostgreSQL extension to GitHub Releases
# Example:
# build_and_publish_pg_extension "pg_cron" "1.0.0" "https://github.com/citusdata/pg_cron/archive/refs/tags/v1.0.0.tar.gz"
build_and_publish_pg_extension() {
  local PG_EXTENSION_NAME=$1
  local PG_EXTENSION_VERSION=$2
  local PG_EXTENSION_URL=$3

  # Checkinstall uses the version in the folder name as the package version, which
  # needs to be semVer compliant, so we sanitize the version first before using it anywhere
  SANITIZED_PG_EXTENSION_VERSION=$(sanitize_version "$PG_EXTENSION_VERSION")

  # Retrieve the versions of Ubuntu and Postgres we're running on, to specify in the .deb filename
  UBUNTU_VERSION=$(lsb_release -rs | sed 's/\.//')

  # Check if the GitHub Release exists
  release_url="https://github.com/paradedb/third-party-pg_extensions/releases/tag/$PG_EXTENSION_NAME-v$SANITIZED_PG_EXTENSION_VERSION-pg$PG_MAJOR_VERSION-$ARCH-ubuntu$UBUNTU_VERSION"
  if curl --output /dev/null --silent --head --fail "$release_url"; then
    echo "Release for $PG_EXTENSION_NAME version $PG_EXTENSION_VERSION $ARCH on PostgreSQL $PG_MAJOR_VERSION and Ubuntu 22.04 already exists, skipping..."
  else
    # Build and package the extension as a .deb
    build_and_package_pg_extension "$PG_EXTENSION_NAME" "$SANITIZED_PG_EXTENSION_VERSION" "$PG_EXTENSION_URL"

    # Create a new GitHub release for the extension. Note, GITHUB_TOKEN is read from the CI environment
    release_response=$(curl -s -X POST https://api.github.com/repos/paradedb/third-party-pg_extensions/releases \
        -H "Authorization: token $GITHUB_TOKEN" \
        -H "Content-Type: application/json" \
        -d '{
        "tag_name": "'"$PG_EXTENSION_NAME"'-v'"$SANITIZED_PG_EXTENSION_VERSION"'-pg'"$PG_MAJOR_VERSION"'-'"$ARCH"'-ubuntu'"$UBUNTU_VERSION"'",
        "name": "'"$PG_EXTENSION_NAME"' '"$SANITIZED_PG_EXTENSION_VERSION"' '"$ARCH"'",
        "body": "Internal ParadeDB Release for '"$PG_EXTENSION_NAME"' version '"$SANITIZED_PG_EXTENSION_VERSION"' for arch '"$ARCH"'. This release is not intended for public use."
    }')
    upload_url=$(echo "$release_response" | jq .upload_url --raw-output | sed "s/{?name,label}//")

    # Upload the .deb file to the newly created GitHub release
    asset_name="$PG_EXTENSION_NAME-v$SANITIZED_PG_EXTENSION_VERSION-pg$PG_MAJOR_VERSION-$ARCH-ubuntu$UBUNTU_VERSION.deb"
    deb_file_path="/tmp/${PG_EXTENSION_NAME//_/-}_$SANITIZED_PG_EXTENSION_VERSION-1_$ARCH.deb"
    curl -X POST "${upload_url}?name=${asset_name}" \
      -H "Authorization: token $GITHUB_TOKEN" \
      -H "Content-Type: application/vnd.DEBIAN.binary-package" \
      --data-binary "@${deb_file_path}"
  fi
}


# Iterate over all arguments, which are expected to be comma-separated values of the format NAME,VERSION,URL
for EXTENSION in "$@"; do
  IFS=',' read -ra EXTENSION_DETAILS <<< "$EXTENSION"
  build_and_publish_pg_extension "${EXTENSION_DETAILS[0]}" "${EXTENSION_DETAILS[1]}" "${EXTENSION_DETAILS[2]}"
done

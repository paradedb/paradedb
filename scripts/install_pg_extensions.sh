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

check_pgx_package_exists() {
  local s3_bucket_url="https://pgx-get.s3.amazonaws.com"
  local deb_file="$1"

  # Check if the .deb file exists by sending a HEAD request
  response_code=$(curl -s -o /dev/null -w "%{http_code}" -I "$s3_bucket_url/$deb_file")

  if [ "$response_code" == "200" ]; then
    return 0  # File exists
  else
    return 1  # File doesn't exist
  fi
}

download_pgx_package() {
  local s3_bucket_url="https://pgx-get.s3.amazonaws.com"
  local deb_file="$1"

  # Download the .deb file from S3
  curl -O "$s3_bucket_url/$deb_file"
  echo ".deb file downloaded from S3."
}

upload_pgx_package() {
  local s3_bucket="pgx-get"
  local deb_filename="$1"

  # about the file
  local bucket_filepath="/${s3_bucket}/${deb_filename}"
  local all_users_group="uri=http://acs.amazonaws.com/groups/global/AllUsers"

  # metadata
  local contentType="application/x-compressed-tar"
  local dateValue=`date -R`
  local signature_string="PUT\n\n${contentType}\n${dateValue}\nx-amz-grant-read:${all_users_group}\n${bucket_filepath}"

  # s3 keys -- environment variables
  local s3_access_key="$S3_ACCESS_KEY"
  local s3_secret_key="$S3_SECRET_KEY"

  # prepare signature hash to be sent in Authorization header
  local signature_hash=`echo -en ${signature_string} | openssl sha1 -hmac ${s3_secret_key} -binary | base64`

  # Upload the .deb file to S3 using a PUT request
  curl -X PUT -T "/tmp/${deb_filename}" \
    -H "Host: ${s3_bucket}.s3.amazonaws.com" \
    -H "Date: ${dateValue}" \
    -H "Content-Type: ${contentType}" \
    -H "Authorization: AWS ${s3_access_key}:${signature_hash}" \
    -H "x-amz-grant-read: ${all_users_group}" \
    https://${s3_bucket}.s3.amazonaws.com/${deb_filename}

  echo ""
  echo ".deb file created and uploaded to S3."
}

compile_pgx_package() {
  local PG_EXTENSION_NAME=$1
  local PG_EXTENSION_VERSION=$2
  local PG_EXTENSION_URL=$3
  local ARCH=$4

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
  echo make OPTFLAGS="$OPTFLAGS" "-j$(nproc)"
  make OPTFLAGS="$OPTFLAGS" "-j$(nproc)"
  echo checkinstall -D --nodoc --install=no --fstrans=no --backup=no --pakdir=/tmp 
  checkinstall -D --nodoc --install=no --fstrans=no --backup=no --pakdir=/tmp --arch="$ARCH"
}








# Function to compile & package a single PostgreSQL extension as a .deb
# Example:
# install_pg_extension "pg_cron" "1.0.0" "https://github.com/citusdata/pg_cron/archive/refs/tags/v1.0.0.tar.gz"
install_pg_extension() {
  local PG_EXTENSION_NAME=$1
  local PG_EXTENSION_VERSION=$2
  local PG_EXTENSION_URL=$3

  local ARCH=$(uname -m)

  local FULL_PACKAGE_NAME="${PG_EXTENSION_NAME}_${PG_EXTENSION_VERSION}-1_${ARCH}.deb"

  # If the extension package already exists in S3, we simply retrieve it. Otherwise we compile and upload it
  if check_pgx_package_exists "$FULL_PACKAGE_NAME"; then
    echo "Extension package ${FULL_PACKAGE_NAME} already exists in S3, downloading pre-compiled package..."
    download_pgx_package "${PG_EXTENSION_NAME}_${PG_EXTENSION_VERSION}-1_${ARCH}.deb"
  else
    echo "Extension package ${FULL_PACKAGE_NAME} does not exist in S3, compiling and uploading..."
    compile_pgx_package "$PG_EXTENSION_NAME" "$PG_EXTENSION_VERSION" "$PG_EXTENSION_URL" "$ARCH"
    upload_pgx_package "$FULL_PACKAGE_NAME"
  fi
}


# Iterate over all arguments, which are expected to be comma-separated values of the format NAME,VERSION,URL
for EXTENSION in "$@"; do
  IFS=',' read -ra EXTENSION_DETAILS <<< "$EXTENSION"
  install_pg_extension "${EXTENSION_DETAILS[0]}" "${EXTENSION_DETAILS[1]}" "${EXTENSION_DETAILS[2]}"
done

#!/bin/bash

# TODO: Write description

# Exit on subcommand errors
set -Eeuo pipefail





# Handle params
usage() {
  echo "Usage: $0 [OPTIONS]"
  echo "Options:"
  echo " -h (optional),  Display this help message"
  echo " -t (optional),  Version tag to benchmark:"
  echo "                  - 'x.y.z'  Runs the full ClickBench benchmark against a specific ParadeDB Docker image (e.g. 0.3.1)"
  echo "                  - 'latest' Runs the full ClickBench benchmark the latest ParadeDB Docker image"
  echo "                  - 'local'  Runs the full ClickBench benchmark the current commit inside a local ParadeDB Docker build"
  echo "                  - 'pgrx'   Runs a minified ClickBench benchmark against the current commit inside the pg_analytics pgrx PostgreSQL instance"
  exit 1
}

# Instantiate vars
FLAG_TAG="pgrx"
DOCKER_PORT=5432

# Assign flags to vars and check
while getopts "ht:s:" flag
do
  case $flag in
    h)
      usage
      ;;
    t)
      FLAG_TAG=$OPTARG
      ;;
    *)
      usage
      ;;
  esac
done

# Determine the base directory of the script
BASEDIR=$(dirname "$0")
cd "$BASEDIR/"

# Cleanup function to reset the environment
cleanup() {
  echo ""
  echo "Cleaning up benchmark environment..."
  if [ "$FLAG_TAG" == "pgrx" ]; then
    # Check if PostgreSQL is in recovery mode. This can happen if one of the quer caused a crash. If
    # so, we need to wait for recovery to finish before we can drop the extension.
    for attempt in {1..5}; do
      psql -h localhost -p 28816 -d pg_analytics -t -c 'DROP EXTENSION IF EXISTS pg_analytics CASCADE;' &> /dev/null && break
      echo "PostgreSQL is in recovery mode (likely due to a crash), waiting for recovery to finish..."
      sleep 5
    done
    if [ "$attempt" -eq 5 ]; then
      echo "Failed to drop pg_analytics extension after several attempts. PostgreSQL is likely still in recovery mode."
    fi
    cargo pgrx stop
  else
    if docker ps -q --filter "name=paradedb" | grep -q .; then
      docker kill paradedb > /dev/null 2>&1
    fi
    docker rm paradedb > /dev/null 2>&1
  fi

  # Delete the log.txt file, if it exists
  if [ -f "log.txt" ]; then
    rm -rf "log.txt"
  fi
  echo "Done, goodbye!"
}

# Register the cleanup function to run when the script exits
trap cleanup EXIT

# Download function to retrieve the dataset and verify its checksum
download_and_verify() {
  local url=$1
  local checksum=$2
  local filename=$3

  # Check if the file already exists and verify its checksum
  if [ -e "$filename" ]; then
    if echo "$checksum  $filename" | md5sum -c --status; then
      echo "Dataset '$filename' already exists and is verified, skipping download..."
      return
    else
      echo "Checksum mismatch. Re-downloading '$filename'..."
    fi
  fi

  # Downloading the file
  echo "Downloading $filename dataset..."
  wget --no-verbose --continue -O "$filename.gz" "$url"
  gzip -d "$filename.gz"
  chmod 666 "$filename"
}

echo ""
echo "*********************************************************************************"
echo "* Benchmarking pg_analytics version '$FLAG_TAG' against TPC-H"
echo "*********************************************************************************"
echo ""

if [ "$FLAG_TAG" == "pgrx" ]; then


    # TODO


else

    # TODO

fi

echo ""
echo "Benchmark complete!"

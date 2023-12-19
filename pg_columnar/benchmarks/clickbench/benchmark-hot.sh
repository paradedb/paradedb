#!/bin/bash




# This script benchmarks the performance of pg_columnar using the ClickBench benchmkar suite. It is
# supported on both Ubuntu and macOS, for local development via pgrx as well as CI testing via Docker.
# The local development version runs a smaller subset of the hits dataset, called hits_v1, which is
# roughly 7.5GB. The CI version runs the full dataset, which is roughly 75GB.

# TODO: Make this work for both local development and CI testing, will need to pass a flag, probs

# Exit on subcommand errors
set -Eeuo pipefail

# Handle params
usage() {
  echo "Usage: $0 [OPTIONS]"
  echo "Options:"
  echo " -h (optional),   Display this help message"
  echo " -t (optional),   Docker tag to use for paradedb/paradedb:tag. Use 'local' to build from source. Use 'pgrx' to run against pgrx instead of Docker. Default: 'latest'"
  exit 1
}

# Instantiate vars
FLAG_TAG="latest"
TRIES=3

# Assign flags to vars and check
while getopts "ht:" flag
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
  echo "Cleaning up benchmark environment..."
  psql -h localhost -p 28815 -d pg_columnar -t -c 'DROP EXTENSION pg_columnar CASCADE;'
  cargo pgrx stop
  echo "Done, goodbye!"
}

# Register the cleanup function to run when the script exits
trap cleanup EXIT

echo ""
echo "*******************************************************"
echo "* Running ClickBench against pg_columnar"
echo "*******************************************************"
echo ""

if [ "$FLAG_TAG" == "pgrx" ]; then
  # For local development, we download hits_v1.tsv, which is ~7.5GB
  if [ ! -e hits_v1.tsv ]; then
    curl https://datasets.clickhouse.com/hits/tsv/hits_v1.tsv.xz | unxz --threads=`nproc` > hits_v1.tsv
    chmod 777 ~ hits_v1.tsv
  fi

  # Build pg_columnar and start its Postgres instance
  echo "Building pg_columnar..."
  cargo build
  cargo pgrx start

  # Connect to the PostgreSQL database and execute all commands in the same session
  psql -h localhost -p 28815 -d pg_columnar <<EOF
  \echo
  \echo Creating pg_columnar
  \i create.sql
  \echo
  \echo Loading data...
  \timing on
  COPY hits FROM 'hits_v1.tsv' WITH freeze
  \echo
  \echo Running queries...
  \timing on
  \i queries.sql
  \echo Benchmark complete!
EOF
else
  if [ ! -e hits.tsv ]; then
    wget --no-verbose --continue 'https://datasets.clickhouse.com/hits_compatible/hits.tsv.gz'
    gzip -d hits.tsv.gz
    chmod 777 ~ hits.tsv
  fi

  # TODO: Pull the dockerfile and build it
fi

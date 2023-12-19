#!/bin/bash

# This script runs the ClickBench benchmarks. It is designed to run on both Ubuntu and macOS, for
# local development as well. The local development version only runs a small subset of the dataset. It
# currently runs for pg15.

# Exit on subcommand errors
set -Eeuo pipefail
IFS=$'\n\t'

# Cleanup function to reset the environment
cleanup() {
  echo "Cleaning up benchmark environment..."
  psql -h localhost -p 28815 -d pg_columnar -t -c 'DROP EXTENSION pg_columnar CASCADE;'
  cargo pgrx stop
  echo "Done, goodbye!"
}

# Register the cleanup function to run when the script exits
trap cleanup EXIT


# # Download hits.tsv if we don't already have it
# if [ ! -e hits.tsv ]; then
#     wget --no-verbose --continue 'https://datasets.clickhouse.com/hits_compatible/hits.tsv.gz'
#     gzip -d hits.tsv.gz
#    # Handle permissions
#    chmod 777 ~ hits.tsv
# fi

# For local development, we download hits_v1.tsv, which is ~7.5GB
if [ ! -e hits_v1.tsv ]; then
  curl https://datasets.clickhouse.com/hits/tsv/hits_v1.tsv.xz | unxz --threads=`nproc` > hits_v1.tsv
  chmod 777 ~ hits_v1.tsv
fi

# # Determine the base directory of the script
# BASEDIR=$(dirname "$0")
# cd "$BASEDIR/../"
# BASEDIR=$(pwd)

# Build pg_columnar and start its Postgres instance
cargo build
cargo pgrx start

# Load data into pgrx-managed database
# sudo -h localhost -p 28815 psql pg_columnar -t < clickbench/paradedb/create.sql
# sudo -h localhost -p 28815 psql pg_columnar -t -c '\timing' -c "\\COPY hits FROM 'hits_v1.tsv' WITH freeze"
# sudo -h localhost -p 28815 psql pg_columnar -t -c 'VACUUM ANALYZE hits'

TRIES=3


# Connect to the PostgreSQL database and execute all commands in the same session
psql -h localhost -p 28815 -d pg_columnar -t <<EOF
\echo "1 - creating table"
\i clickbench/paradedb/create.sql
\echo "2 - COPYing hits"
\timing
COPY hits FROM 'hits_v1.tsv' WITH freeze
\echo "3 - running queries"
\timing
\timing
\i clickbench/paradedb/queries.sql
\echo "4 - done"
EOF

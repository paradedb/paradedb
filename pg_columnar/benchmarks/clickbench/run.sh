#!/bin/bash

# This script is used to run the ClickBench benchmark suite over multiple iterations. We
# only use it in the full-suite benchmarking run, which is run via Docker. For local
# benchmarking via pgrx, we run the benchmark once, directly run the 'queries.sql' script.

# Exit on subcommand errors
set -Eeuo pipefail

TRIES=3
OS=$(uname)

while read -r query; do
  sync

  if [[ "$OS" == "Linux" ]]; then
    echo 3 | sudo tee /proc/sys/vm/drop_caches
  elif [[ "$OS" == "Darwin" ]]; then
    echo 3 | tee /proc/sys/vm/drop_caches
  else
    echo "Unsupported OS: $OS"
    exit 1
  fi

  # TODO: Make this work with multiple storage types
  echo "$query";
  for _ in $(seq 1 $TRIES); do
    if [[ "$OS" == "Linux" ]]; then
      sudo -u postgres psql pg_columnar -t -c '\timing' -c "$query" | grep 'Time'
    elif [[ "$OS" == "Darwin" ]]; then
      psql -h localhost -p 28815 -d pg_columnar -t -c '\timing' -c "$query" | grep 'Time'
    fi
  done;
done < clickbench/paradedb/queries.sql

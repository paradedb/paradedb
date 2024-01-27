#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

TRIES=3
export PGPASSWORD='mypassword'

while IFS= read -r query; do
  sync
  echo 3 | sudo tee /proc/sys/vm/drop_caches

  echo "$query";
  # shellcheck disable=SC2034
  for i in $(seq 1 $TRIES); do
    psql -h localhost -U myuser -d mydatabase -p 5432 -t -c '\timing' -c "$query" | grep 'Time'
  done;
done < queries.sql

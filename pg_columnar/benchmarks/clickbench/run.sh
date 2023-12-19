#!/bin/bash

TRIES=3

cat clickbench/paradedb/queries.sql | while read query; do
  sync
  # echo 3 | sudo tee /proc/sys/vm/drop_caches
  echo 3 | tee /proc/sys/vm/drop_caches


  echo "$query";
  for i in $(seq 1 $TRIES); do
    psql -h localhost -p 28815 -d pg_columnar -t -c '\timing' -c "$query" | grep 'Time'
    # sudo -u postgres psql pg_columnar -t -c '\timing' -c "$query" | grep 'Time'

  done;
done;

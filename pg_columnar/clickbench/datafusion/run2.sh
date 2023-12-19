#!/bin/bash

QUERY_NUM=1
cat queries.sql | while read query; do
  sync
  echo 3 | sudo tee /proc/sys/vm/drop_caches >/dev/null

  echo "$query" > /tmp/query.sql

  echo
  echo
  echo -----------------------------------------
  echo
  echo $QUERY_NUM. "$query"
  echo
  echo -----------------------------------------
  echo
  echo


  datafusion-cli -f create.sql /tmp/query.sql

  QUERY_NUM=$((QUERY_NUM + 1))
done

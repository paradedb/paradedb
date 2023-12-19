#!/bin/bash

# Check if an argument is provided
if [ "$#" -ne 1 ]; then
  echo "Usage: $0 [single|partitioned]"
  exit 1
fi

# Set the SQL file based on the argument
if [ "$1" = "single" ]; then
  CREATE_SQL_FILE="create_single.sql"
elif [ "$1" = "partitioned" ]; then
  CREATE_SQL_FILE="create_partitioned.sql"
else
  echo "Invalid argument. Please use 'single' or 'partitioned'."
  exit 1
fi

TRIES=3
QUERY_NUM=1
echo $1
cat queries.sql | while read query; do
  sync
  echo 3 | sudo tee /proc/sys/vm/drop_caches >/dev/null

  echo "$query" > /tmp/query.sql

  echo -n "["
  for i in $(seq 1 $TRIES); do
    # 1. there will be two query result, one for creating table another for executing the select statement
    # 2. each query contains a "Query took xxx seconds", we just grep these 2 lines
    # 3. use sed to take the second line
    # 4. use awk to take the number we want
    RES=`datafusion-cli -f $CREATE_SQL_FILE /tmp/query.sql 2>&1 | grep "Query took" | sed -n 2p | awk '{print $7}'`
    [[ $RES != "" ]] && \
      echo -n "$RES" || \
      echo -n "null"
    [[ "$i" != $TRIES ]] && echo -n ", "
    echo "${QUERY_NUM},${i},${RES}" >> result.csv
  done
  echo "],"

  QUERY_NUM=$((QUERY_NUM + 1))
done

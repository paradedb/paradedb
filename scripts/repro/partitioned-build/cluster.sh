#!/usr/bin/env bash
# Starts a throwaway PostgreSQL cluster that preloads pg_search.
# usage: cluster.sh <pg bin dir> <data dir> <port>
set -euo pipefail
BIN=$1
DATA=$2
PORT=$3

if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  echo "port $PORT is busy" >&2
  exit 1
fi

"$BIN/initdb" -D "$DATA" -U postgres -E UTF8 --locale=C --no-sync >/dev/null
cat >>"$DATA/postgresql.conf" <<CONF
port = $PORT
listen_addresses = 'localhost'
unix_socket_directories = ''
shared_preload_libraries = 'pg_search'
# A merge pins buffers for every segment it reads. 1GB is too small for a merge of 1,000 segments.
shared_buffers = '3GB'
max_worker_processes = 32
max_parallel_workers = 16
# The default partition count is max(4, max_parallel_workers_per_gather) for each partition_by field.
max_parallel_workers_per_gather = 8
max_parallel_maintenance_workers = 4
fsync = off
synchronous_commit = off
full_page_writes = off
wal_level = minimal
max_wal_senders = 0
max_wal_size = '2GB'
log_error_verbosity = verbose
CONF

# With RUST_BACKTRACE set, pgrx puts the Rust stack of a panic in the DETAIL of the server log.
RUST_BACKTRACE=1 "$BIN/pg_ctl" -D "$DATA" -l "$DATA.log" -w start
"$BIN/psql" -h localhost -p "$PORT" -U postgres -d postgres -X -Atc "SHOW data_directory"

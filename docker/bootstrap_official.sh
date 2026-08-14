#!/bin/sh
set -eu
export PGUSER="$POSTGRES_USER"

initialize() {
  psql -v ON_ERROR_STOP=1 -d "$1" <<'EOSQL'
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_search;
EOSQL
}

initialize template1
initialize "$POSTGRES_DB"
[ "$POSTGRES_DB" = paradedb ] || psql -d postgres -c 'CREATE DATABASE paradedb'

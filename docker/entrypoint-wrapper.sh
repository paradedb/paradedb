#!/bin/bash
set -e

export PGDATA=${PGDATA:-/var/lib/postgresql/data}

if [ -s "$PGDATA/PG_VERSION" ]; then
    /usr/local/bin/tune-postgresql.sh
else
    echo "ParadeDB auto-tune: Detected fresh install. Deferring tuning to bootstrap."
fi

exec /usr/local/bin/docker-entrypoint.sh "$@"

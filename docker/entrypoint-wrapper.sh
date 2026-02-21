#!/bin/bash
set -e

export PGDATA=${PGDATA:-/var/lib/postgresql/data}
# We only tune from `entrypoint-wrapper.sh` on pre-existing deployments, as it is otherwise handled by `bootstrap.sh` as part of the initial deploy.
if [ -s "$PGDATA/PG_VERSION" ]; then
	/usr/local/bin/tune-postgresql.sh
else
	echo "ParadeDB auto-tune: Detected fresh install.Deferring tuning to bootstrap."
fi

exec /usr/local/bin/docker-entrypoint.sh "$@"

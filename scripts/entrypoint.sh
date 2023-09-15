#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Determine the PostgreSQL major version
POSTGRES_VERSION_FULL=$(pg_config --version)
POSTGRES_VERSION_MAJOR=$(echo "$POSTGRES_VERSION_FULL" | awk '{print $2}' | cut -d '.' -f1)

# List of extensions to possibly install (if a version variable is set)
declare -A extensions=(
  [pg_bm25]=${PG_BM25_VERSION:-}
  [pgml]=${PGML_VERSION:-}
  [vector]=${PGVECTOR_VERSION:-}
  [pg_search]=${PG_SEARCH_VERSION:-}
  [pg_cron]=${PG_CRON_VERSION:-}
  [pg_net]=${PG_NET_VERSION:-}
  [pg_ivm]=${PG_IVM_VERSION:-}
  [pg_graphql]=${PG_GRAPHQL_VERSION:-}
  [pg_hashids]=${PG_HASHIDS_VERSION:-}
  [pg_jsonschema]=${PG_JSONSCHEMA_VERSION:-}
  [pg_repack]=${PG_REPACK_VERSION:-}
  [pg_stat_monitor]=${PG_STAT_MONITOR_VERSION:-}
  [pg_hint_plan]=${PG_HINT_PLAN_VERSION:-}
  [pgtap]=${PGTAP_VERSION:-}
  [pgaudit]=${PGAUDIT_VERSION:-}
  [postgis]=${POSTGIS_VERSION:-}
  [pgrouting]=${PGROUTING_VERSION:-}
  [http]=${PGSQL_HTTP_VERSION:-}
  [hypopg]=${HYPOPG_VERSION:-}
  [rum]=${RUM_VERSION:-}
  [citus]=${CITUS_VERSION:-}
)

# List of extensions that must be added to shared_preload_libraries
declare -A preload_names=(
  [citus]=citus # Must be first in shared_preload_libraries
  [pgml]=pgml
  [pg_cron]=pg_cron
  [pg_net]=pg_net
  [pgaudit]=pgaudit
)

# Build the shared_preload_libraries list, only including extensions that are installed
# and have a preload name specified
for extension in "${!extensions[@]}"; do
  version=${extensions[$extension]}
  if [ -n "$version" ] && [[ -n "${preload_names[$extension]:-}" ]]; then
    preload_name=${preload_names[$extension]}
    shared_preload_list+="${preload_name},"
  fi
done
# Remove the trailing comma
shared_preload_list=${shared_preload_list%,}

# Update the PostgreSQL configuration
sed -i "s/^cron\.database_name = .*/cron\.database_name = '$POSTGRES_DB'/" "/etc/postgresql/${POSTGRES_VERSION_MAJOR}/main/postgresql.conf"
sed -i "s/^shared_preload_libraries = .*/shared_preload_libraries = '$shared_preload_list'  # (change requires restart)/" "/etc/postgresql/${POSTGRES_VERSION_MAJOR}/main/postgresql.conf"

# Start the PostgreSQL server
service postgresql start

# Setup users
createuser root --superuser --login
psql -c "CREATE ROLE $POSTGRES_USER PASSWORD '$POSTGRES_PASSWORD' SUPERUSER LOGIN"
createdb "$POSTGRES_DB" --owner "$POSTGRES_USER"

echo "PostgreSQL is up - installing extensions..."

# Preinstall extensions for which a version is specified
for extension in "${!extensions[@]}"; do
  version=${extensions[$extension]}
  if [ -n "$version" ]; then
    PGPASSWORD=$POSTGRES_PASSWORD psql -c "CREATE EXTENSION IF NOT EXISTS $extension CASCADE" -d "$POSTGRES_DB" -U "$POSTGRES_USER" -h 127.0.0.1 -p 5432 || echo "Failed to install extension $extension"
  fi
done

echo "PostgreSQL extensions installed - tailing server..."

# Trap SIGINT and SIGTERM signals, stop PostgreSQL, and gracefully shut down
trap "service postgresql stop; echo 'PostgreSQL server has stopped - exiting...'; exit 0" SIGINT SIGTERM

# Keep the container running
tail -f /dev/null

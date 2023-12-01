#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# List of extensions to possibly install (if a version variable is set)
declare -A extensions=(
  [pg_bm25]=${PG_BM25_VERSION:-}
  [pg_search]=${PG_SEARCH_VERSION:-}
  [pg_sparse]=${PG_SPARSE_VERSION:-}
  [vector]=${PGVECTOR_VERSION:-}
  [pg_cron]=${PG_CRON_VERSION:-}
  [pg_net]=${PG_NET_VERSION:-}
  [pg_ivm]=${PG_IVM_VERSION:-}
  [pg_graphql]=${PG_GRAPHQL_VERSION:-}
  [pg_hashids]=${PG_HASHIDS_VERSION:-}
  [pg_jsonschema]=${PG_JSONSCHEMA_VERSION:-}
  [pg_repack]=${PG_REPACK_VERSION:-}
  [pg_stat_monitor]=${PG_STAT_MONITOR_VERSION:-}
  [pg_hint_plan]=${PG_HINT_PLAN_VERSION:-}
  [pgfaceting]=${PGFACETING_VERSION:-}
  [pgtap]=${PGTAP_VERSION:-}
  [pgaudit]=${PGAUDIT_VERSION:-}
  [postgis]=${POSTGIS_VERSION:-}
  [pgrouting]=${PGROUTING_VERSION:-}
  [roaringbitmap]=${PG_ROARINGBITMAP_VERSION:-}
  [http]=${PGSQL_HTTP_VERSION:-}
  [hypopg]=${HYPOPG_VERSION:-}
  [rum]=${RUM_VERSION:-}
  [age]=${AGE_VERSION:-}
  [citus]=${CITUS_VERSION:-}
  [pgsodium]=${PGSODIUM_VERSION:-}
  [pgfincore]=${PGFINCORE_VERSION:-}
  [pg_partman]=${PG_PARTMAN_VERSION:-}
  [pg_jobmon]=${PG_JOBMON_VERSION:-}
  [pg_auto_failover]=${PG_AUTO_FAILOVER_VERSION:-}
  [pg_show_plans]=${PG_SHOW_PLANS_VERSION:-}
  [sqlite_fdw]=${SQLITE_FDW_VERSION:-}
  [pgddl]=${PGDDL_VERSION:-}
  [mysql_fdw]=${MYSQL_FDW_VERSION:-}
)

# List of extensions that must be added to shared_preload_libraries
declare -A preload_names=(
  [pg_cron]=pg_cron
  [pg_net]=pg_net
  [pgaudit]=pgaudit
  [citus]=citus
  [pgsodium]=pgsodium
  [pg_partman]=pg_partman_bgw
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
echo "pg_net.database_name = '$POSTGRES_DB'" >> "${PGDATA}/postgresql.conf"
echo "cron.database_name = '$POSTGRES_DB'" >> "${PGDATA}/postgresql.conf"
sed -i "s/^#shared_preload_libraries = .*/shared_preload_libraries = '$shared_preload_list'  # (change requires restart)/" "${PGDATA}/postgresql.conf"

# Setup the database role (the user passed via -e POSTGRES_USER to the Docker run command)
POSTGRES_USER_ROLE_EXISTS=$(psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -tAc "SELECT 1 FROM pg_roles WHERE rolname='$POSTGRES_USER'")
if [ -z "$POSTGRES_USER_ROLE_EXISTS" ]; then
  psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
  CREATE ROLE $POSTGRES_USER WITH SUPERUSER LOGIN;
EOSQL
fi

# Setup the postgres role (a user named postgres is necessary for pg_net to work)
POSTGRES_ROLE_EXISTS=$(psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -tAc "SELECT 1 FROM pg_roles WHERE rolname='postgres'")
if [ -z "$POSTGRES_ROLE_EXISTS" ]; then
  psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
  CREATE ROLE postgres WITH SUPERUSER LOGIN;
EOSQL
fi

# Configure search_path to include paradedb schema for template1, and default to public (by listing it first)
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
  ALTER DATABASE "$POSTGRES_DB" SET search_path TO public,paradedb;
EOSQL

# Configure search_path to include paradedb schema for template1, so that it is
# inherited by all new databases created, and default to public (by listing it first)
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "template1" <<-EOSQL
  ALTER DATABASE template1 SET search_path TO public,paradedb;
EOSQL

# We need to restart the server for the changes above to be reflected
pg_ctl restart

# We collect basic, anonymous telemetry to help us understand how many people are using
# the project. We only do this if TELEMETRY is set to "true", and only do it once per deployment
if [[ ${TELEMETRY:-} == "true" ]]; then
  curl -s -L --header "Content-Type: application/json" -d '{
    "api_key": "'"$POSTHOG_API_KEY"'",
    "event": "ParadeDB Deployment",
    "distinct_id": "'"$(uuidgen)"'",
    "properties": {
      "commit_sha": "'"${COMMIT_SHA:-}"'"
    }
  }' "$POSTHOG_HOST/capture/" > /dev/null
fi

# Mark telemetry as handled so we don't try to send it again when
# initializing our PostgreSQL extensions. We use a file for IPC
# between this script and our PostgreSQL extensions
echo "true" > /tmp/telemetry

echo "PostgreSQL is up - installing extensions..."

# Preinstall extensions for which a version is specified
for extension in "${!extensions[@]}"; do
  version=${extensions[$extension]}
  if [ -n "$version" ]; then
    PGPASSWORD=$POSTGRES_PASSWORD psql -c "CREATE EXTENSION IF NOT EXISTS $extension CASCADE" -d "$POSTGRES_DB" -U "$POSTGRES_USER" || echo "Failed to install extension $extension"
  fi
done

#!/bin/bash
# shellcheck disable=SC2154

# Executed at container start to bootstrap ParadeDB extensions and Postgres settings.

# Exit on subcommand errors
set -Eeuo pipefail

# Perform all actions as $POSTGRES_USER
export PGUSER="$POSTGRES_USER"

# The `pg_cron` extension can only be installed in the `postgres` database, as per
# our configuration in our Dockerfile. Therefore, we install it separately here.
psql -d postgres -c "CREATE EXTENSION IF NOT EXISTS pg_cron;"

# Always create a `paradedb` database, regardless of what $POSTGRES_DB is set to
if [ "$POSTGRES_DB" != "paradedb" ]; then
  echo "Creating default 'paradedb' database"
  psql -d postgres -c "CREATE DATABASE paradedb;"
fi

# Load ParadeDB and third-party extensions into template1, paradedb, and $POSTGRES_DB
# Creating extensions in template1 ensures that they are available in all new databases.
for DB in template1 paradedb "$POSTGRES_DB"; do
  echo "Loading ParadeDB extensions into $DB"
  psql -d "$DB" <<-'EOSQL'
    CREATE EXTENSION IF NOT EXISTS pg_search;
    CREATE EXTENSION IF NOT EXISTS pg_ivm;
    CREATE EXTENSION IF NOT EXISTS vector;
    CREATE EXTENSION IF NOT EXISTS postgis;
    CREATE EXTENSION IF NOT EXISTS postgis_topology;
    CREATE EXTENSION IF NOT EXISTS fuzzystrmatch;
    CREATE EXTENSION IF NOT EXISTS postgis_tiger_geocoder;
    CREATE EXTENSION IF NOT EXISTS pg_stat_statements;
EOSQL
done

# Add the `paradedb` schema to template1, paradedb, and $POSTGRES_DB
for DB in template1 paradedb "$POSTGRES_DB"; do
  echo "Adding 'paradedb' search_path to $DB"
  psql -d "$DB" -c "ALTER DATABASE \"$DB\" SET search_path TO public,paradedb;"
done

echo "ParadeDB bootstrap completed!"

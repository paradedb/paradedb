#!/bin/bash
# shellcheck disable=SC2154

# Executed at container start to boostrap ParadeDB extensions and Postgres settings.

# Exit on subcommand errors
set -Eeuo pipefail

# Perform all actions as $POSTGRES_USER
export PGUSER="$POSTGRES_USER"

# Add extensions to shared_preload_libraries
PG_CONF="/var/lib/postgresql/data/postgresql.conf"
LIBRARIES_TO_ADD="pg_search,pg_lakehouse,pg_cron"
if [ -f "$PG_CONF" ]; then
  if grep -q "^shared_preload_libraries" "$PG_CONF"; then
    # If the line exists, append new libraries to it
    sed -i "s/^shared_preload_libraries = '\(.*\)'/shared_preload_libraries = '\1,$LIBRARIES_TO_ADD'/" "$PG_CONF"
  else
    # If the line doesn't exist, add it
    echo "shared_preload_libraries = '$LIBRARIES_TO_ADD'" >> "$PG_CONF"
  fi
  echo "Added $LIBRARIES_TO_ADD to shared_preload_libraries in postgresql.conf"
else
  echo "Error: postgresql.conf not found at $PG_CONF"
  exit 1
fi

# This setting is required by pg_cron to CREATE EXTENSION properly. It can only be installed in one database,
# so we install it in the user database. Creating the `pg_cron` extension requires a restart of the PostgreSQL server.
echo "cron.database_name = '$POSTGRES_DB'" >> "$PG_CONF"

echo "Restarting PostgreSQL to apply changes..."
pg_ctl restart

# Create the 'template_paradedb' template db
psql -c "CREATE DATABASE template_paradedb IS_TEMPLATE true;"

# Load ParadeDB extensions into both template_database and $POSTGRES_DB
for DB in template_paradedb "$POSTGRES_DB"; do
  echo "Loading ParadeDB extensions into $DB"
  psql -d "$DB" -c "CREATE EXTENSION IF NOT EXISTS pg_search;"
  psql -d "$DB" -c "CREATE EXTENSION IF NOT EXISTS pg_lakehouse;"
  psql -d "$DB" -c "CREATE EXTENSION IF NOT EXISTS pg_ivm;"
  psql -d "$DB" -c "CREATE EXTENSION IF NOT EXISTS vector;"
  psql -d "$DB" -c "CREATE EXTENSION IF NOT EXISTS vectorscale;"
done

# Add the `paradedb` schema to both template_database and $POSTGRES_DB
for DB in template_paradedb "$POSTGRES_DB"; do
  echo "Adding 'paradedb' schema to $DB"
  psql -d "$DB" -c "ALTER DATABASE \"$DB\" SET search_path TO \"\$user\",public,paradedb;"
done

# TODO: Is this restart required?
echo "Restarting PostgreSQL to apply changes..."
pg_ctl restart

echo "ParadeDB bootstrap completed!"

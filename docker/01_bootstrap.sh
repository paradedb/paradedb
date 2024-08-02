#!/bin/bash

# This script is executed at the initialization of the ParadeDB container
# to configure it with required extensions and Postgres settings

# Exit on subcommand errors
set -Eeuo pipefail

# Perform all actions as $POSTGRES_USER
export PGUSER="$POSTGRES_USER"

# Create the 'template_paradedb' template db
"${psql[@]}" <<- 'EOSQL'
CREATE DATABASE template_paradedb IS_TEMPLATE true;
EOSQL

# Load ParadeDB extensions into both template_database and $POSTGRES_DB
for DB in template_paradedb "$POSTGRES_DB"; do
echo "Loading ParadeDB extensions into $DB"
"${psql[@]}" --dbname="$DB" <<-'EOSQL'
  CREATE EXTENSION IF NOT EXISTS pg_search;
  CREATE EXTENSION IF NOT EXISTS pg_lakehouse;
  CREATE EXTENSION IF NOT EXISTS pg_ivm;
  CREATE EXTENSION IF NOT EXISTS vector;
  CREATE EXTENSION IF NOT EXISTS vectorscale;
EOSQL
done

# Add the `paradedb` schema to both template_database and $POSTGRES_DB
for DB in template_paradedb "$POSTGRES_DB"; do
  echo "Adding 'paradedb' schema to $DB"
  ALTER DATABASE "$DB" SET search_path TO "$user",public,paradedb;
EOSQL
done

# Add ParadeDB extensions to shared_preload_libraries
PG_CONF="/var/lib/postgresql/data/postgresql.conf"
LIBRARIES_TO_ADD="pg_search,pg_lakehouse"
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

echo "Restarting PostgreSQL to apply changes..."
pg_ctl restart

echo "ParadeDB bootstrap completed!"

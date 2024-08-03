#!/bin/bash

# Upgrades ParadeDB extensions to the specified version. Assumes the new version of the 
# extensions is already downloaded and installed on the system.

# Exit on subcommand errors
set -Eeuo pipefail

# Perform all actions as $POSTGRES_USER
export PGUSER="$POSTGRES_USER"

PARADEDB_VERSION="${PARADEDB_VERSION%%+*}"

# Load ParadeDB into both template_database and $POSTGRES_DB
for DB in template_paradedb "$POSTGRES_DB" "${@}"; do
  echo "Updating ParadeDB extensions '$DB' to $PARADEDB_VERSION"
  psql --dbname="$DB" -c "
    -- Upgrade ParadeDB extensions
    -- pg_search
    CREATE EXTENSION IF NOT EXISTS pg_search VERSION '$PARADEDB_VERSION';
    ALTER EXTENSION pg_search  UPDATE TO '$PARADEDB_VERSION';

    -- pg_lakehouse
    CREATE EXTENSION IF NOT EXISTS pg_lakehouse VERSION '$PARADEDB_VERSION';
    ALTER EXTENSION pg_lakehouse UPDATE TO '$PARADEDB_VERSION';
  "
done

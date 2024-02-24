#!/bin/bash

# This script is executed at the initialization of the ParadeDB container
# to configure it with required extensions and Postgres settings

# Exit on subcommand errors
set -Eeuo pipefail

# If no user is set, the default user will be the `postgres` superuser, so we
# set the superuser password to default to the user password in that case
SUPERUSER_PASSWORD=${POSTGRESQL_POSTGRES_PASSWORD:-$POSTGRESQL_PASSWORD}

echo "ParadeDB bootstrap started..."
echo "Configuring PostgreSQL search path..."

# Add the `paradedb` schema to the user database, and default to public (by listing it first)
PGPASSWORD=$POSTGRESQL_PASSWORD psql -U "$POSTGRESQL_USERNAME" -d "$POSTGRESQL_DATABASE" -c "ALTER DATABASE $POSTGRESQL_DATABASE SET search_path TO public,paradedb;"

# Add the `paradedb` schema to the template1 database, to have it inherited by all new databases
# created post-initialization, and default to public (by listing it first)
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d template1 -c "ALTER DATABASE template1 SET search_path TO public,paradedb;"

echo "Installing PostgreSQL extensions..."

# Pre-install all required PostgreSQL extensions to the user database via the `postgres` superuser
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "CREATE EXTENSION IF NOT EXISTS pg_bm25 CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "CREATE EXTENSION IF NOT EXISTS pg_analytics CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "CREATE EXTENSION IF NOT EXISTS svector CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "CREATE EXTENSION IF NOT EXISTS vector CASCADE;"

# # Pre-install pg_cron. It can only be installed in the user database, so we don't add it to the template1 database
# PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "CREATE EXTENSION IF NOT EXISTS pg_cron CASCADE;"

# Add the `pg_cron` extension to the user database. This is required for `pg_cron` to install correctly
echo "cron.database_name = '$POSTGRESQL_DATABASE'" >> "/opt/bitnami/postgresql/conf/postgresql.conf"



# Pre-install all required PostgreSQL extensions to the template1 database, to have them inherited by all new
# databases created post-initialization, via the `postgres` user
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d template1 -c "CREATE EXTENSION IF NOT EXISTS pg_bm25 CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d template1 -c "CREATE EXTENSION IF NOT EXISTS pg_analytics CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d template1 -c "CREATE EXTENSION IF NOT EXISTS svector CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d template1 -c "CREATE EXTENSION IF NOT EXISTS vector CASCADE;"

echo "Sending anonymous deployment telemetry (to turn off, unset PARADEDB_TELEMETRY)..."

# We collect basic, anonymous telemetry to help us understand how many people are using
# the project. We only do this if PARADEDB_TELEMETRY is set to "true"
if [[ ${PARADEDB_TELEMETRY:-} == "true" ]]; then
  # For privacy reasons, we generate an anonymous UUID for each new deployment
  UUID_FILE="/bitnami/postgresql/data/paradedb_uuid"
  if [ ! -f "$UUID_FILE" ]; then
    uuidgen > "$UUID_FILE"
  fi
  DISTINCT_ID=$(cat "$UUID_FILE")

  # Send the deployment event to PostHog
  curl -s -L --header "Content-Type: application/json" -d '{
    "api_key": "'"$POSTHOG_API_KEY"'",
    "event": "ParadeDB Deployment",
    "distinct_id": "'"$DISTINCT_ID"'",
    "properties": {
      "commit_sha": "'"${COMMIT_SHA:-}"'"
    }
  }' "$POSTHOG_HOST/capture/"

  # Mark telemetry as handled so we don't try to send it again when
  # initializing our PostgreSQL extensions. We use a file for IPC
  # between this script and our PostgreSQL extensions
  echo "true" > /tmp/telemetry
fi

echo "ParadeDB bootstrap completed!"

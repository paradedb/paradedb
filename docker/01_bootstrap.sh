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

# This setting is required by pg_cron to CREATE EXTENSION properly. It can only be installed in one database, so we install it in
# the user database. Creating the `pg_cron` extension requires a restart of the PostgreSQL server, so we can't do it here. A restart
# is already part of the launch process of the Bitnami PostgreSQL container post this point, so the extension can be created by the
# user after the container has been launched.
#
# For simplicity, and because we don't expect most users to use pg_cron, we don't force a restart here and we don't pre-create the
# extension, leaving it to the user to do it if they want to use it.
echo "cron.database_name = '$POSTGRESQL_DATABASE'" >> "/opt/bitnami/postgresql/conf/postgresql.conf"

# Pre-install all required PostgreSQL extensions to the user database via the `postgres` superuser
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "CREATE EXTENSION IF NOT EXISTS pg_bm25 CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "CREATE EXTENSION IF NOT EXISTS pg_analytics CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "CREATE EXTENSION IF NOT EXISTS svector CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "CREATE EXTENSION IF NOT EXISTS vector CASCADE;"

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

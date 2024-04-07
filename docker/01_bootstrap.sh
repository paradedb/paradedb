#!/bin/bash

# This script is executed at the initialization of the ParadeDB container
# to configure it with required extensions and Postgres settings

# Exit on subcommand errors
set -Eeuo pipefail

# If no user is set, the default user will be the `postgres` superuser, so we
# set the superuser password to default to the user password in that case
SUPERUSER_PASSWORD=${POSTGRESQL_POSTGRES_PASSWORD:-$POSTGRESQL_PASSWORD}

echo "ParadeDB bootstrap started..."
echo "Configuring PostgreSQL permissions..."

# Grant pg_read_all_settings role to the user (necessary for pg_analytics and general database introspection)
# PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "GRANT pg_read_all_settings TO \"$POSTGRESQL_USERNAME\";"

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
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "CREATE EXTENSION IF NOT EXISTS pg_analytics CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "CREATE EXTENSION IF NOT EXISTS svector CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "CREATE EXTENSION IF NOT EXISTS vector CASCADE;"

# Pre-install all required PostgreSQL extensions to the template1 database, to have them inherited by all new
# databases created post-initialization, via the `postgres` user
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d template1 -c "CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d template1 -c "CREATE EXTENSION IF NOT EXISTS pg_analytics CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d template1 -c "CREATE EXTENSION IF NOT EXISTS svector CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d template1 -c "CREATE EXTENSION IF NOT EXISTS vector CASCADE;"

echo "ParadeDB bootstrap completed!"

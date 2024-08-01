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

echo "Configuring PostgreSQL permissions..."

# Grant pg_read_all_settings role to the user (necessary for general database introspection)
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "GRANT pg_read_all_settings TO \"$POSTGRESQL_USERNAME\";"

echo "Installing PostgreSQL extensions..."

# Pre-install all required PostgreSQL extensions to the user database via the `postgres` superuser
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "CREATE EXTENSION IF NOT EXISTS pg_lakehouse CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "CREATE EXTENSION IF NOT EXISTS pg_ivm CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "CREATE EXTENSION IF NOT EXISTS vector CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d "$POSTGRESQL_DATABASE" -c "CREATE EXTENSION IF NOT EXISTS vectorscale CASCADE;"

# Pre-install all required PostgreSQL extensions to the template1 database, to have them inherited by all new
# databases created post-initialization, via the `postgres` user
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d template1 -c "CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d template1 -c "CREATE EXTENSION IF NOT EXISTS pg_lakehouse CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d template1 -c "CREATE EXTENSION IF NOT EXISTS pg_ivm CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d template1 -c "CREATE EXTENSION IF NOT EXISTS vector CASCADE;"
PGPASSWORD=$SUPERUSER_PASSWORD psql -U postgres -d template1 -c "CREATE EXTENSION IF NOT EXISTS vectorscale CASCADE;"

echo "ParadeDB bootstrap completed!"

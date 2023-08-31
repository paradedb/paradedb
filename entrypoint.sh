#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Start PostgreSQL
docker-entrypoint.sh postgres &

# Give PostgreSQL time to start
sleep 10

# Wait for the PostgreSQL server to start
until pg_isready -h localhost -p 5432 -U "$POSTGRES_USER" -d "$POSTGRES_DB"; do
  echo "PostgreSQL is unavailable - sleeping..."
  sleep 1
done

echo "PostgreSQL is up - installing extensions..."

# Preinstall some extensions for the user
psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "CREATE EXTENSION IF NOT EXISTS pg_bm25;"
psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "CREATE EXTENSION IF NOT EXISTS pg_ivm;"
psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "CREATE EXTENSION IF NOT EXISTS pg_graphql;"

# Wait for the PostgreSQL server to stop
wait $!

echo "PostgreSQL server has stopped - exiting..."

#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Start PostgreSQL
docker-entrypoint.sh postgres &

# Give Postgres time to start
sleep 10

# Wait for the PostgreSQL server to start
until pg_isready -h localhost -p 5432 -U "$POSTGRES_USER" -d "$POSTGRES_DB"; do
  echo "Postgres is unavailable - sleeping"
  sleep 1
done

echo "Postgres is up - installing extension"

# Load the compiled extension
psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "CREATE EXTENSION IF NOT EXISTS retake_extension;"

# Wait for the PostgreSQL server to stop
wait $!

echo "PostgreSQL server has stopped"

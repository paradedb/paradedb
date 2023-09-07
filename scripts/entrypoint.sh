#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Start the PostgreSQL server
service postgresql start

# Setup users
sudo -u postgres createuser root --superuser --login
sudo -u postgres psql -c "CREATE ROLE $POSTGRES_USER PASSWORD '$POSTGRES_PASSWORD' SUPERUSER LOGIN"
sudo -u postgres createdb "$POSTGRES_DB" --owner "$POSTGRES_USER"

echo "PostgreSQL is up - installing extensions..."

# List of extensions to install
extensions=(
  pg_bm25
  pg_cron
  pg_net
  pg_ivm
  pg_graphql
  pg_hashids
  pg_jsonschema
  pg_repack
  pg_stat_monitor
  pg_hint_plan
  pgml
  pgaudit
  postgis
  pgrouting
  vector
  http
  hypopg
  rum
  citus
)

# Preinstall extensions for the user
for extension in "${extensions[@]}"; do
  PGPASSWORD=$POSTGRES_PASSWORD psql -c "CREATE EXTENSION IF NOT EXISTS $extension" -d "$POSTGRES_DB" -U "$POSTGRES_USER" -h 127.0.0.1 -p 5432
done

echo "PostgreSQL extensions installed - tailing server..."

# Keep the container running
tail -f /dev/null

echo "PostgreSQL server has stopped - exiting..."

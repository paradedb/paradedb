
# Do I need #!/bin/bash


# Is this cauasing the issue?
# # Exit on subcommand errors
# set -Eeuo pipefail


# TODO: Add pg_cron too

# List of extensions to pre-install in ParadeDB
extensions=(
  pg_bm25
  pg_analytics
  svector
  vector
)

# List of extensions that must be added to shared_preload_libraries to be installed. Extensions that
# get added to shared_preload_libraries must also be listed in `extensions` above in order to get installed.
preload_names=(
  pg_bm25
  pg_analytics
)

# # Build the shared_preload_libraries list, only including extensions that have a preload name specified
# shared_preload_list=""
# for preload_name in "${preload_names[@]}"; do
#   shared_preload_list+="${preload_name},"
# done
# # Remove the trailing comma
# shared_preload_list=${shared_preload_list%,}
# echo "shared_preload_libraries = $shared_preload_list"

# # Update the PostgreSQL configuration
# sed -i "s/^#shared_preload_libraries = .*/shared_preload_libraries = '$shared_preload_list'  # (change requires restart)/" "${PGDATA}/postgresql.conf"


# # Configure search_path to include paradedb schema for template1, and default to public (by listing it first)
# psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
#   ALTER DATABASE "$POSTGRES_DB" SET search_path TO public,paradedb;
# EOSQL

# # Configure search_path to include paradedb schema for template1, so that it is
# # inherited by all new databases created, and default to public (by listing it first)
# psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "template1" <<-EOSQL
#   ALTER DATABASE template1 SET search_path TO public,paradedb;
# EOSQL


# # We need to restart the server for the changes above to be reflected
# pg_ctl restart

# # We collect basic, anonymous telemetry to help us understand how many people are using
# # the project. We only do this if TELEMETRY is set to "true"
# if [[ ${TELEMETRY:-} == "true" ]]; then
#   # For privacy reasons, we generate an anonymous UUID for each new deployment
#   UUID_FILE="/var/lib/postgresql/data/paradedb_uuid"
#   if [ ! -f "$UUID_FILE" ]; then
#     uuidgen > "$UUID_FILE"
#   fi
#   DISTINCT_ID=$(cat "$UUID_FILE")

#   # Send the deployment event to PostHog
#   curl -s -L --header "Content-Type: application/json" -d '{
#     "api_key": "'"$POSTHOG_API_KEY"'",
#     "event": "ParadeDB Deployment",
#     "distinct_id": "'"$DISTINCT_ID"'",
#     "properties": {
#       "commit_sha": "'"${COMMIT_SHA:-}"'"
#     }
#   }' "$POSTHOG_HOST/capture/"

#   # Mark telemetry as handled so we don't try to send it again when
#   # initializing our PostgreSQL extensions. We use a file for IPC
#   # between this script and our PostgreSQL extensions
#   echo "true" > /tmp/telemetry
# fi

# echo "PostgreSQL is up - installing extensions..."

# Preinstall extensions
for extension in "${extensions[@]}"; do
  PGPASSWORD=$POSTGRES_PASSWORD psql -c "CREATE EXTENSION $extension CASCADE" -d "$POSTGRES_DB" -U "$POSTGRES_USER" || echo "Failed to install extension $extension"
done



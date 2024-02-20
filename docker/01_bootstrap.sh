






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





# Define PostgreSQL database and extension details
DATABASE_NAME=${POSTGRES_DB:template1}

# Add extension as superuser
echo "Adding extension as superuser..."
# These need to be done via postgres user cuz C language permission
# This adds the extension so that any new database within this postmaster will automatically have the extension
PGPASSWORD=postgres psql -U postgres -d $DATABASE_NAME -c "CREATE EXTENSION IF NOT EXISTS pg_bm25 CASCADE;"
PGPASSWORD=postgres psql -U postgres -d $DATABASE_NAME -c "CREATE EXTENSION IF NOT EXISTS pg_analytics CASCADE;"
PGPASSWORD=postgres psql -U postgres -d $DATABASE_NAME -c "CREATE EXTENSION IF NOT EXISTS vector CASCADE;"
PGPASSWORD=postgres psql -U postgres -d $DATABASE_NAME -c "CREATE EXTENSION IF NOT EXISTS svector CASCADE;"

# Add to the current database, since it gets created prior to this script
PGPASSWORD=postgres psql -U postgres -d "template1" -c "CREATE EXTENSION IF NOT EXISTS pg_bm25 CASCADE;"
PGPASSWORD=postgres psql -U postgres -d "template1" -c "CREATE EXTENSION IF NOT EXISTS pg_analytics CASCADE;"
PGPASSWORD=postgres psql -U postgres -d "template1" -c "CREATE EXTENSION IF NOT EXISTS vector CASCADE;"
PGPASSWORD=postgres psql -U postgres -d "template1" -c "CREATE EXTENSION IF NOT EXISTS svector CASCADE;"



echo "Configure search_path..."

# Configure search_path to include paradedb schema for template1, and default to public (by listing it first)
PGPASSWORD=$POSTGRES_PASSWORD psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
  ALTER DATABASE "$POSTGRES_DB" SET search_path TO public,paradedb;
EOSQL

# Configure search_path to include paradedb schema for template1, so that it is
# inherited by all new databases created, and default to public (by listing it first)
PGPASSWORD=postgres psql -v ON_ERROR_STOP=1 --username "postgres" --dbname "template1" <<-EOSQL
  ALTER DATABASE template1 SET search_path TO public,paradedb;
EOSQL


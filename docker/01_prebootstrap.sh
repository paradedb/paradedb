#!/bin/bash

# This script is executed at the initialization of the ParadeDB container
# to configure it with required extensions and Postgres settings

# Exit on subcommand errors
set -Eeuo pipefail


# Add the `pg_cron` extension to the user database. This is required for `pg_cron` to install correctly
echo "cron.database_name = '$POSTGRESQL_DATABASE'" >> "${POSTGRESQL_DATA_DIR}/postgresql.conf"



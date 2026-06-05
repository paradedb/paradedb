#!/bin/bash
# shellcheck disable=SC2154

# Executed at container start to bootstrap ParadeDB extensions and Postgres settings.

# Exit on subcommand errors
set -Eeuo pipefail

# Perform all actions as $POSTGRES_USER
export PGUSER="$POSTGRES_USER"

# The `pg_cron` extension can only be installed in the `postgres` database, as per
# our configuration in our Dockerfile. Therefore, we install it separately here.
psql -d postgres -c "CREATE EXTENSION IF NOT EXISTS pg_cron;"

# Always create a `paradedb` database, regardless of what $POSTGRES_DB is set to
if [ "$POSTGRES_DB" != "paradedb" ]; then
  echo "Creating default 'paradedb' database"
  psql -d postgres -c "CREATE DATABASE paradedb;"
fi

# Load ParadeDB and third-party extensions into template1, paradedb, and $POSTGRES_DB
# Creating extensions in template1 ensures that they are available in all new databases.
for DB in template1 paradedb "$POSTGRES_DB"; do
  echo "Loading ParadeDB extensions into $DB"
  psql -d "$DB" <<-'EOSQL'
    CREATE EXTENSION IF NOT EXISTS pg_search;
    CREATE EXTENSION IF NOT EXISTS pg_ivm;
    CREATE EXTENSION IF NOT EXISTS vector;
    CREATE EXTENSION IF NOT EXISTS postgis;
    CREATE EXTENSION IF NOT EXISTS postgis_topology;
    CREATE EXTENSION IF NOT EXISTS fuzzystrmatch;
    CREATE EXTENSION IF NOT EXISTS postgis_tiger_geocoder;
    CREATE EXTENSION IF NOT EXISTS pg_stat_statements;
EOSQL
done

# Add the `paradedb` schema to template1, paradedb, and $POSTGRES_DB
for DB in template1 paradedb "$POSTGRES_DB"; do
  echo "Adding 'paradedb' search_path to $DB"
  psql -d "$DB" -c "ALTER DATABASE \"$DB\" SET search_path TO public,paradedb;"
done

echo "Tuning postgresql.conf based on available container resources..."
if [ -f /sys/fs/cgroup/memory.max ] && [ "$(cat /sys/fs/cgroup/memory.max)" != "max" ]; then
  TOTAL_RAM_MB=$(awk "BEGIN {print int($(cat /sys/fs/cgroup/memory.max) / 1024 / 1024)}")
elif [ -f /sys/fs/cgroup/memory/memory.limit_in_bytes ]; then
  CGROUP_V1_LIMIT=$(cat /sys/fs/cgroup/memory/memory.limit_in_bytes)
  # cgroup v1 reports ~2^63 when no limit is set -- fall through to /proc/meminfo
  if [ "$CGROUP_V1_LIMIT" -lt 68719476736 ]; then # < 64TB = real limit
    TOTAL_RAM_MB=$(awk "BEGIN {print int($CGROUP_V1_LIMIT / 1024 / 1024)}")
  else
    TOTAL_RAM_MB=$(grep MemTotal /proc/meminfo | awk '{print int($2 / 1024)}')
  fi
else
  # Fallback to host /proc/meminfo if cgroups are not available
  # TODO: this awk command fails
  TOTAL_RAM_MB=$(grep MemTotal /proc/meminfo | awk '{print int($2 / 1024)}')
fi

if [ -z "$TOTAL_RAM_MB" ]; then
  echo "ParadeDB auto-tune: WARNING: Could not detect system RAM. Exiting."
  exit 0
fi

if [ "$TOTAL_RAM_MB" -lt 512 ]; then
  echo "ParadeDB auto-tune: System RAM (${TOTAL_RAM_MB}MB) below 512MB minimum. Exiting."
  exit 0
fi

if [ -f /sys/fs/cgroup/cpu.max ] && [ "$(awk '{print $1}' /sys/fs/cgroup/cpu.max)" != "max" ]; then
  # cgroup v2
  CPU_QUOTA=$(awk '{print $1}' /sys/fs/cgroup/cpu.max)
  CPU_PERIOD=$(awk '{print $2}' /sys/fs/cgroup/cpu.max)
  CPU_COUNT=$(awk "BEGIN {printf \"%.0f\", $CPU_QUOTA / $CPU_PERIOD}")
elif [ -f /sys/fs/cgroup/cpu/cpu.cfs_quota_us ] && [ "$(cat /sys/fs/cgroup/cpu/cpu.cfs_quota_us)" != "-1" ]; then
  # cgroup v1
  CPU_QUOTA=$(cat /sys/fs/cgroup/cpu/cpu.cfs_quota_us)
  CPU_PERIOD=$(cat /sys/fs/cgroup/cpu/cpu.cfs_period_us)
  CPU_COUNT=$(awk "BEGIN {printf \"%.0f\", $CPU_QUOTA / $CPU_PERIOD}")
else
  CPU_COUNT=$(nproc)
fi

# Ensure CPU_COUNT is at least 1
if [ "$CPU_COUNT" -lt 1 ]; then
  CPU_COUNT=1
fi

SHARED_BUFFERS_MB=$(awk "BEGIN {s=int($TOTAL_RAM_MB * 0.25); print (s > 16384 ? 16384 : s)}")
MAX_CONNECTIONS=100 # This is the postgres default
WORK_MEM_MB=$(awk "BEGIN {w=int(($TOTAL_RAM_MB - $SHARED_BUFFERS_MB) / ($MAX_CONNECTIONS * 3)); print (w < 4 ? 4 : w)}")
PARALLEL_WORKERS=$(awk "BEGIN {w=int($CPU_COUNT * 5); print (w > 128 ? 128 : w)}")

echo "ParadeDB auto-tune: Writing configuration to $PGDATA/postgresql.conf"
tee -a "$PGDATA/postgresql.conf" <<EOF
# Begin ParadeDB tuning recommendations
# Parameters based on auto-detected $CPU_COUNT CPUs and ${TOTAL_RAM_MB}MB RAM
shared_buffers = '${SHARED_BUFFERS_MB}MB' # 25% of RAM, capped at 16GB
effective_cache_size = '$(awk "BEGIN {print int($TOTAL_RAM_MB * 0.75)}")MB' # 75% of RAM
maintenance_work_mem = '$(awk "BEGIN {m=int($TOTAL_RAM_MB / 16); print (m > 2048 ? 2048 : m)}")MB' # RAM / 16, capped at 2GB
work_mem = '${WORK_MEM_MB}MB' # (RAM - shared_buffers) / (3 * max_connections)
max_parallel_workers = '$PARALLEL_WORKERS' # CPUs * 5, capped at 128
max_worker_processes = '$(awk "BEGIN {print int($PARALLEL_WORKERS + 8)}")' # max_parallel_workers + 8
max_parallel_workers_per_gather = '$(awk "BEGIN {p=int($CPU_COUNT / 2); print (p < 1 ? 1 : p)}")' # CPUs / 2, at least 1
max_parallel_maintenance_workers = '$(awk "BEGIN {p=int($CPU_COUNT / 2); print (p > 8 ? 8 : (p < 2 ? 2 : p))}")' # CPUs / 2, at least 2, at most 8
# End ParadeDB tuning recommendations
EOF

echo "ParadeDB bootstrap completed!"

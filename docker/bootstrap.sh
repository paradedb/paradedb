#!/bin/bash
# shellcheck disable=SC2154

# Executed at container start to bootstrap ParadeDB extensions and Postgres settings.
# This file is NOT executed when running in CloudNativePG as it circumvents the normal
# Postgres container entrypoint.

# Exit on subcommand errors
set -Eeuo pipefail

# Perform all actions as $POSTGRES_USER
export PGUSER="$POSTGRES_USER"
PDB_TUNE=${PDB_TUNE:-true}

container_memory_mb() {
  local bytes=""

  if [ -r /sys/fs/cgroup/memory.max ]; then
    # cgroup v2: "max" means the container has no explicit memory limit.
    bytes=$(cat /sys/fs/cgroup/memory.max)
    [ "$bytes" = "max" ] && bytes=""
  elif [ -r /sys/fs/cgroup/memory/memory.limit_in_bytes ]; then
    # cgroup v1: very large values are used as an effectively-unlimited sentinel.
    bytes=$(cat /sys/fs/cgroup/memory/memory.limit_in_bytes)
    [ "$bytes" -ge 10000000000000000 ] && bytes=""
  fi

  # If cgroups are unavailable or unlimited, fall back to host-visible memory.
  if [ -z "$bytes" ]; then
    awk '/MemTotal/ {print int($2 / 1024)}' /proc/meminfo
  else
    awk "BEGIN {print int($bytes / 1024 / 1024)}"
  fi
}

container_cpu_count() {
  local quota="" period="" cpus=""

  if [ -r /sys/fs/cgroup/cpu.max ]; then
    # cgroup v2: "max" means there is no quota, only the scheduler default.
    read -r quota period < /sys/fs/cgroup/cpu.max
    [ "$quota" = "max" ] && quota=""
  elif [ -r /sys/fs/cgroup/cpu/cpu.cfs_quota_us ]; then
    # cgroup v1: a negative quota means the CPU quota is unlimited.
    quota=$(cat /sys/fs/cgroup/cpu/cpu.cfs_quota_us)
    period=$(cat /sys/fs/cgroup/cpu/cpu.cfs_period_us)
    [ "$quota" -lt 0 ] && quota=""
  fi

  if [ -n "$quota" ]; then
    # quota / period gives the number of CPUs allowed by the cgroup.
    cpus=$(awk "BEGIN {print $quota / $period}")
  else
    # If there is no CPU quota, use the CPU count visible to the process.
    cpus=$(nproc)
  fi

  # Choose the minimum of the CPU count detected between the cgroup and nproc in case the cgroup quota is higher than the cpuset
  # Set 1 CPU as the minimum.
  awk -v cpus="$cpus" -v visible_cpus="$(nproc)" 'BEGIN {cpus = cpus > visible_cpus ? visible_cpus : cpus; print (cpus < 1 ? 1 : cpus)}'
}

tune() {
  if [[ "$PDB_TUNE" == "false" ]]; then
    echo "Auto-tuning is disabled, skipping."
    return 0
  fi

  TOTAL_RAM_MB=$(container_memory_mb)
  CPU_COUNT=$(container_cpu_count)

  if [ "$TOTAL_RAM_MB" -lt 512 ]; then
    echo "Available memory is less than 512mb, skipping auto tuning."
    return 0
  fi

  SHARED_BUFFERS_MB=$(awk "BEGIN {s=int($TOTAL_RAM_MB * 0.25); print (s > 16384 ? 16384 : s)}")
  MAX_CONNECTIONS=100 # This is the postgres default
  WORK_MEM_MB=$(awk "BEGIN {w=int(($TOTAL_RAM_MB - $SHARED_BUFFERS_MB) / ($MAX_CONNECTIONS * 3)); print (w < 15 ? 15 : w)}")
  PARALLEL_WORKERS=$(awk "BEGIN {print int($CPU_COUNT * 5)}")

  echo "ParadeDB auto-tune: Writing configuration to $PGDATA/postgresql.conf"
  {
    echo
    echo "# Begin ParadeDB tuning recommendations"
    echo "# Parameters based on auto-detected $CPU_COUNT CPUs and ${TOTAL_RAM_MB}MB RAM"
    printf "%-45s # %s\n" "shared_buffers = '${SHARED_BUFFERS_MB}MB'" "25% of RAM, capped at 16GB"
    printf "%-45s # %s\n" "effective_cache_size = '$(awk "BEGIN {print int($TOTAL_RAM_MB * 0.75)}")MB'" "75% of RAM"
    printf "%-45s # %s\n" "maintenance_work_mem = '$(awk "BEGIN {m=int($TOTAL_RAM_MB / 16); print (m > 2048 ? 2048 : m)}")MB'" "RAM / 16, capped at 2GB"
    printf "%-45s # %s\n" "work_mem = '${WORK_MEM_MB}MB'" "(RAM - shared_buffers) / (3 * max_connections), at least 15MB"
    printf "%-45s # %s\n" "max_parallel_workers = '$PARALLEL_WORKERS'" "CPUs * 5"
    printf "%-45s # %s\n" "max_worker_processes = '$(awk "BEGIN {print int($PARALLEL_WORKERS + 8)}")'" "max_parallel_workers + 8"
    printf "%-45s # %s\n" "max_parallel_workers_per_gather = '$(awk "BEGIN {p=int($CPU_COUNT / 2); print (p < 1 ? 1 : p)}")'" "CPUs / 2, at least 1"
    printf "%-45s # %s\n" "max_parallel_maintenance_workers = '$(awk "BEGIN {p=int($CPU_COUNT / 2); print (p > 8 ? 8 : (p < 2 ? 2 : p))}")'" "CPUs / 2, at least 2, at most 8"
    echo "# End ParadeDB tuning recommendations"
  } | tee -a "$PGDATA/postgresql.conf"
}

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
    CREATE EXTENSION IF NOT EXISTS vector;
    CREATE EXTENSION IF NOT EXISTS pg_search;
    CREATE EXTENSION IF NOT EXISTS pg_ivm;
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


# Tune postgresql.conf settings for the available hardware
tune

echo "ParadeDB bootstrap completed!"

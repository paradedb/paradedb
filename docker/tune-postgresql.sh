#!/bin/bash

set -Eeuo pipefail

PGDATA=${PGDATA:-/var/lib/postgresql/data}
CONF_FILE="$PGDATA/postgresql.auto.conf"

tune_param() {
  local param=$1
  local value=$2
  local env_override=$3

  local final_val=${env_override:-$value}

  if [ ! -f "$CONF_FILE" ]; then
    touch "$CONF_FILE"
    chown postgres:postgres "$CONF_FILE" 2>/dev/null || true
  fi

  if grep -q "^$param =" "$CONF_FILE"; then
    sed -i "s|^$param =.*|$param = '$final_val'|" "$CONF_FILE"
  else
    echo "$param = '$final_val'" >> "$CONF_FILE"
  fi
}

if [ "${PG_TUNE_ENABLED:-true}" = "false" ]; then
  echo "ParadeDB auto-tune: Disabled via PG_TUNE_ENABLED"
  exit 0
fi

if [ ! -d "$PGDATA" ]; then
  echo "ParadeDB auto-tune: PGDATA ($PGDATA) does not exist. Skipping tuning."
  exit 0
fi

if [ -f /sys/fs/cgroup/memory.max ] && [ "$(cat /sys/fs/cgroup/memory.max)" != "max" ]; then
  TOTAL_RAM_BYTES=$(cat /sys/fs/cgroup/memory.max)
elif [ -f /sys/fs/cgroup/memory/memory.limit_in_bytes ]; then
  TOTAL_RAM_BYTES=$(cat /sys/fs/cgroup/memory/memory.limit_in_bytes)
else
  TOTAL_RAM_BYTES=$(grep MemTotal /proc/meminfo | awk '{print $2 * 1024}')
fi

TOTAL_RAM_MB=$((TOTAL_RAM_BYTES / 1024 / 1024))

if [ "$TOTAL_RAM_MB" -lt 512 ]; then
  echo "ParadeDB auto-tune: System RAM ($TOTAL_RAM_MB MB) is too low. Skipping."
  exit 0
fi

CPU_COUNT=$(nproc)

SB_MB=$(awk "BEGIN {print int($TOTAL_RAM_MB * 0.25)}")
ECS_MB=$(awk "BEGIN {print int($TOTAL_RAM_MB * 0.75)}")
MWM_MB=$(awk "BEGIN {m=int($TOTAL_RAM_MB / 16); print (m > 2048 ? 2048 : m)}")
WM_MB=$(awk "BEGIN {print int(($TOTAL_RAM_MB - $SB_MB) / (100 * 3))}")
WAL_MB=$(awk "BEGIN {w=int($SB_MB / 32); print (w > 64 ? 64 : w)}")

PARALLEL_GATHER=$(awk "BEGIN {p=int($CPU_COUNT / 2); print (p < 1 ? 1 : p)}")

echo "ParadeDB auto-tune: Applying settings for $TOTAL_RAM_MB MB RAM and $CPU_COUNT CPUs"

tune_param "shared_buffers" "${SB_MB}MB" "${PG_TUNE_SHARED_BUFFERS:-}"
tune_param "effective_cache_size" "${ECS_MB}MB" "${PG_TUNE_EFFECTIVE_CACHE_SIZE:-}"
tune_param "maintenance_work_mem" "${MWM_MB}MB" "${PG_TUNE_MAINTENANCE_WORK_MEM:-}"
tune_param "work_mem" "${WM_MB}MB" "${PG_TUNE_WORK_MEM:-}"
tune_param "wal_buffers" "${WAL_MB}MB" "${PG_TUNE_WAL_BUFFERS:-}"
tune_param "max_worker_processes" "$CPU_COUNT" "${PG_TUNE_MAX_WORKER_PROCESSES:-}"
tune_param "max_parallel_workers" "$CPU_COUNT" "${PG_TUNE_MAX_PARALLEL_WORKERS:-}"
tune_param "max_parallel_workers_per_gather" "$PARALLEL_GATHER" "${PG_TUNE_MAX_PARALLEL_GATHER:-}"
tune_param "random_page_cost" "1.1" "${PG_TUNE_RANDOM_PAGE_COST:-}"
tune_param "effective_io_concurrency" "200" "${PG_TUNE_IO_CONCURRENCY:-}"

echo "ParadeDB auto-tune: Configuration written to $CONF_FILE"

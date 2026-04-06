#!/bin/bash

set -Eeuo pipefail

PGDATA=${PGDATA:-/var/lib/postgresql/data}

# NOTE: This script writes directly to postgresql.auto.conf, which is the same file
# used by PostgreSQL's 'ALTER SYSTEM' command. Manual 'ALTER SYSTEM' changes to
# tuned parameters will be preserved on container restart unless overridden by PG_TUNE_*
# environment variables. To permanently pin a specific value, users must use PG_TUNE_*
# environment variables.
CONF_FILE="$PGDATA/postgresql.auto.conf"

tune_param() {
  local param=$1
  local value=$2
  local env_override=$3

  if [ ! -f "$CONF_FILE" ]; then
    touch "$CONF_FILE"
    chown postgres:postgres "$CONF_FILE" 2>/dev/null || echo "ParadeDB auto-tune: Warning: could not chown $CONF_FILE"
  fi

  # 1. Highest Priority: Docker Environment Variable Override (Always Overwrites)
  if [ -n "$env_override" ]; then
    if grep -qE "^\s*$param\s*=" "$CONF_FILE"; then
      sed -i "s|^\s*$param\s*=.*|$param = '$env_override'|" "$CONF_FILE"
    else
      echo "$param = '$env_override'" >> "$CONF_FILE"
    fi
    echo "ParadeDB auto-tune: $param = $env_override (via env var)"

  # 2. Force Recalculate: If PG_TUNE_FORCE is true, overwrite existing settings with new math
  elif [ "${PG_TUNE_FORCE:-false}" = "true" ]; then
    if grep -qE "^\s*$param\s*=" "$CONF_FILE"; then
      sed -i "s|^\s*$param\s*=.*|$param = '$value'|" "$CONF_FILE"
    else
      echo "$param = '$value'" >> "$CONF_FILE"
    fi
    echo "ParadeDB auto-tune: $param = $value (forced recalculation)"

  # 3. Standard Behavior: Only tune if parameter is MISSING (Respects ALTER SYSTEM)
  elif ! grep -qE "^\s*$param\s*=" "$CONF_FILE" 2>/dev/null; then
    echo "$param = '$value'" >> "$CONF_FILE"
    echo "ParadeDB auto-tune: $param = $value (auto-tuned)"
  
  # 4. Parameter exists and not forcing: Skip
  else
    echo "ParadeDB auto-tune: $param is already set, skipping (use PG_TUNE_FORCE=true to overwrite)"
  fi
}

if [ "${PG_TUNE_ENABLED:-true}" = "false" ]; then
  echo "ParadeDB auto-tune: Disabled via PG_TUNE_ENABLED"
  exit 0
fi

# Skip auto-tuning in Kubernetes environments to avoid conflicts with resource limits
if [ -n "${KUBERNETES_SERVICE_HOST:-}" ] && [ -n "${KUBERNETES_SERVICE_PORT:-}" ] && [ "${PG_TUNE_ENABLED:-}" != "true" ]; then
  echo "ParadeDB auto-tune: Kubernetes environment detected. Disabling auto-tuning."
  exit 0
fi

if [ ! -d "$PGDATA" ]; then
  echo "ParadeDB auto-tune: PGDATA ($PGDATA) does not exist. Skipping tuning."
  exit 0
fi

# To handle large numbers that appear in scientific notation (8.3e+09), which Bash math cannot process but awk can.
if [ -f /sys/fs/cgroup/memory.max ] && [ "$(cat /sys/fs/cgroup/memory.max)" != "max" ]; then
  TOTAL_RAM_MB=$(awk "BEGIN {printf \"%.0f\", $(cat /sys/fs/cgroup/memory.max) / 1024 / 1024}")
elif [ -f /sys/fs/cgroup/memory/memory.limit_in_bytes ]; then
  CGROUP_V1_LIMIT=$(cat /sys/fs/cgroup/memory/memory.limit_in_bytes)
  # cgroup v1 reports ~2^63 when no limit is set -- fall through to /proc/meminfo
  if [ "$CGROUP_V1_LIMIT" -lt 68719476736 ]; then # < 64TB = real limit
    TOTAL_RAM_MB=$(awk "BEGIN {printf \"%.0f\", $CGROUP_V1_LIMIT / 1024 / 1024}")
  else
    TOTAL_RAM_MB=$(grep MemTotal /proc/meminfo | awk '{printf "%.0f", $2 / 1024}')
  fi
else
  # Fallback to host /proc/meminfo if cgroups are not available
  TOTAL_RAM_MB=$(grep MemTotal /proc/meminfo | awk '{printf "%.0f", $2 / 1024}')
fi

# Safety floor: Do not auto-tune on very small instances (< 512MB)
if [ -z "$TOTAL_RAM_MB" ] || [ "$TOTAL_RAM_MB" -lt 512 ]; then
  echo "ParadeDB auto-tune: System RAM ($TOTAL_RAM_MB MB) is too low. Skipping."
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

SB_MB=$(awk "BEGIN {print int($TOTAL_RAM_MB * 0.25)}")
ECS_MB=$(awk "BEGIN {print int($TOTAL_RAM_MB * 0.75)}")
MWM_MB=$(awk "BEGIN {m=int($TOTAL_RAM_MB / 16); print (m > 2048 ? 2048 : m)}")
MAX_CONN=${PG_TUNE_MAX_CONNECTIONS:-100}
WM_MB=$(awk "BEGIN {w=int(($TOTAL_RAM_MB - $SB_MB) / ($MAX_CONN * 3)); print (w < 4 ? 4 : w)}")
WAL_MB=$(awk "BEGIN {w=int($SB_MB / 32); print (w > 64 ? 64 : w)}")

# Global worker pools: 5x CPU count based on production observations, +8 for background tasks
PARALLEL_WORKERS=$(awk "BEGIN {print int($CPU_COUNT * 5)}")
WORKER_PROCESSES=$(awk "BEGIN {print int($PARALLEL_WORKERS + 8)}")

# Per-query workers (half of CPUs, min 1)
PARALLEL_GATHER=$(awk "BEGIN {p=int($CPU_COUNT / 2); print (p < 1 ? 1 : p)}")
# Maintenance workers (half of CPUs, min 2, max 8)
PMW=$(awk "BEGIN {p=int($CPU_COUNT / 2); print (p > 8 ? 8 : (p < 2 ? 2 : p))}")

echo "ParadeDB auto-tune: Applying settings for $TOTAL_RAM_MB MB RAM and $CPU_COUNT CPUs"

tune_param "shared_buffers" "${SB_MB}MB" "${PG_TUNE_SHARED_BUFFERS:-}"
tune_param "effective_cache_size" "${ECS_MB}MB" "${PG_TUNE_EFFECTIVE_CACHE_SIZE:-}"
tune_param "maintenance_work_mem" "${MWM_MB}MB" "${PG_TUNE_MAINTENANCE_WORK_MEM:-}"
tune_param "work_mem" "${WM_MB}MB" "${PG_TUNE_WORK_MEM:-}"
tune_param "wal_buffers" "${WAL_MB}MB" "${PG_TUNE_WAL_BUFFERS:-}"

# Global worker pools
tune_param "max_parallel_workers" "$PARALLEL_WORKERS" "${PG_TUNE_MAX_PARALLEL_WORKERS:-}"
tune_param "max_worker_processes" "$WORKER_PROCESSES" "${PG_TUNE_MAX_WORKER_PROCESSES:-}"

# Per-query and maintenance workers
tune_param "max_parallel_workers_per_gather" "$PARALLEL_GATHER" "${PG_TUNE_MAX_PARALLEL_WORKERS_PER_GATHER:-}"
tune_param "max_parallel_maintenance_workers" "$PMW" "${PG_TUNE_MAX_PARALLEL_MAINTENANCE_WORKERS:-}"

tune_param "random_page_cost" "1.1" "${PG_TUNE_RANDOM_PAGE_COST:-}"
tune_param "effective_io_concurrency" "200" "${PG_TUNE_EFFECTIVE_IO_CONCURRENCY:-}"

echo "ParadeDB auto-tune: Configuration written to $CONF_FILE"

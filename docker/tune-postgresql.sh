#!/bin/bash

set -Eeuo pipefail

CONF_FILE="$PGDATA/postgresql.conf"

# Ensure the file is always owned by postgres on exit, even if the script fails midway.
trap 'chown postgres:postgres "$CONF_FILE" 2>/dev/null || true' EXIT

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
  # TODO: this awk command fails
  TOTAL_RAM_MB=$(grep MemTotal /proc/meminfo | awk '{printf "%.0f", $2 / 1024}')
fi

# Split safety checks for clearer logging
if [ -z "$TOTAL_RAM_MB" ]; then
  echo "ParadeDB auto-tune: WARNING: Could not detect system RAM. Skipping."
  exit 0
fi

if [ "$TOTAL_RAM_MB" -lt 512 ]; then
  echo "ParadeDB auto-tune: System RAM (${TOTAL_RAM_MB}MB) below 512MB minimum. Skipping."
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

# shared_buffers: 25% of RAM, capped at 16GB (16384MB) to prevent diminishing returns
SHARED_BUFFERS_MB=$(awk "BEGIN {s=int($TOTAL_RAM_MB * 0.25); print (s > 16384 ? 16384 : s)}")
EFFECTIVE_CACHE_SIZE_MB=$(awk "BEGIN {print int($TOTAL_RAM_MB * 0.75)}")
MAINTENANCE_WORK_MEM_MB=$(awk "BEGIN {m=int($TOTAL_RAM_MB / 16); print (m > 2048 ? 2048 : m)}")
MAX_CONN=${PG_TUNE_MAX_CONNECTIONS:-100}
WORK_MEM_MB=$(awk "BEGIN {w=int(($TOTAL_RAM_MB - $SHARED_BUFFERS_MB) / ($MAX_CONN * 3)); print (w < 4 ? 4 : w)}")

# Global worker pools: 5x CPU count, capped at 128 to prevent excessive shared memory usage. +8 for background tasks.
PARALLEL_WORKERS=$(awk "BEGIN {w=int($CPU_COUNT * 5); print (w > 128 ? 128 : w)}")
WORKER_PROCESSES=$(awk "BEGIN {print int($PARALLEL_WORKERS + 8)}")

# Per-query workers (half of CPUs, min 1)
PARALLEL_GATHER=$(awk "BEGIN {p=int($CPU_COUNT / 2); print (p < 1 ? 1 : p)}")
# Maintenance workers (half of CPUs, min 2, max 8)
PMW=$(awk "BEGIN {p=int($CPU_COUNT / 2); print (p > 8 ? 8 : (p < 2 ? 2 : p))}")

echo "ParadeDB auto-tune: Applying settings for $TOTAL_RAM_MB MB RAM and $CPU_COUNT CPUs"

echo "ParadeDB auto-tune: Writing configuration to $CONF_FILE"
tee -a "$CONF_FILE" <<EOF
# Begin ParadeDB tuning recommendations
shared_buffers = '${SHARED_BUFFERS_MB}MB'
effective_cache_size = '${EFFECTIVE_CACHE_SIZE_MB}MB'
maintenance_work_mem = '${MAINTENANCE_WORK_MEM_MB}MB'
work_mem = '${WORK_MEM_MB}MB'
max_parallel_workers = '$PARALLEL_WORKERS'
max_worker_processes = '$WORKER_PROCESSES'
max_parallel_workers_per_gather = '$PARALLEL_GATHER'
max_parallel_maintenance_workers = '$PMW'
# End ParadeDB tuning recommendations
EOF

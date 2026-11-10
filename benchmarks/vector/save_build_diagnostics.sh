#!/usr/bin/env bash
set -euo pipefail

output=benchmark-artifacts/index-build
mkdir -p "$output"
docker logs --timestamps "$PARADEDB_CONTAINER" >"$output/postgres.log" 2>&1 || true
docker inspect --format '{{json .State}}' "$PARADEDB_CONTAINER" >"$output/container-state.json" || true
docker exec "$PARADEDB_CONTAINER" sh -c '
  for path in /sys/fs/cgroup/memory.events /sys/fs/cgroup/memory.peak; do
    if [ -r "$path" ]; then echo "$path"; cat "$path"; fi
  done
' >"$output/cgroup-memory.log" 2>&1 || true
docker events --since "$(docker inspect --format '{{.Created}}' "$PARADEDB_CONTAINER")" \
  --until "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --filter "container=$PARADEDB_CONTAINER" \
  --filter event=oom --format '{{json .}}' >"$output/oom-events.jsonl" || true
sudo dmesg -T | tail -200 >"$output/kernel.log" || true

shopt -s nullglob
for run in "$output"/runs/load/*_load; do
  pdbench-report html "$run" -o "$run/report.html" || true
done

#!/usr/bin/env bash
# Local Cohere 1M audit: builds the image, creates the data volume once from the table on the
# benchmarking server, runs run.sh in a container and leaves the results under out/1m.
#
#   SOAR_DATA     held-out queries and base vectors (base.npy, ids.npy, queries.npy, ...)
#   PDBENCH_DATA  directory holding queries/cohere_queries.parquet
#   PDBENCH_SRC   pdbench checkout; PDBENCH_REF is what gets built
#   SOURCE_TABLE  table on SOURCE_URL with the Cohere schema
#   SKIP_BUILD    set to reuse the image
#   HOST_SCRIPTS  set to run this directory's scripts instead of the image's
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
WORKSPACE="$(cd "$REPO/../.." && pwd)"
IMAGE=pg-search-router-audit:local
VOLUME=router-audit-data
CONTAINER=router-audit
SOAR_DATA="${SOAR_DATA:-$WORKSPACE/soar/gate/data}"
PDBENCH_DATA="${PDBENCH_DATA:-$WORKSPACE/benchmarks/soar-20261106/pdbench/data}"
PDBENCH_SRC="${PDBENCH_SRC:-$WORKSPACE/pdbench}"
PDBENCH_REF="${PDBENCH_REF:-e1bd111cb1c2aa64303493c60f067b22a43f039d}"
SOURCE_URL="${SOURCE_URL:-postgresql://localhost:28818/pg_search}"
SOURCE_TABLE="${SOURCE_TABLE:-cohere_1m_q11}"
OUT="$HERE/out"

# Docker Desktop's VM must hold the buffers and the build.
memory=$(docker info --format '{{.MemTotal}}')
cpus=$(docker info --format '{{.NCPU}}')
if [ "$memory" -lt $((16 * 1024 * 1024 * 1024 - 1024 * 1024 * 1024)) ] || [ "$cpus" -lt 8 ]; then
  echo "Docker Desktop has $((memory / 1024 / 1024 / 1024)) GiB and $cpus CPUs;" \
    "raise it to at least 16 GiB and 8 CPUs" >&2
  exit 1
fi

mkdir -p "$OUT/dump" "$OUT/1m"
chmod 777 "$OUT/1m"

if [ -z "${SKIP_BUILD:-}" ]; then
  pdbench_context="$OUT/pdbench-src"
  rm -rf "$pdbench_context"
  mkdir -p "$pdbench_context"
  git -C "$PDBENCH_SRC" archive "$PDBENCH_REF" | tar -x -C "$pdbench_context"
  docker build --platform linux/arm64 --progress=plain -f "$HERE/Dockerfile" \
    --build-context pdbench="$pdbench_context" -t "$IMAGE" "$REPO"
fi

start() {
  local scripts=()
  [ -z "${HOST_SCRIPTS:-}" ] || scripts=(-v "$HERE:/opt/router-audit/benchmarks/router-audit:ro")
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
  docker run -d --name "$CONTAINER" --platform linux/arm64 --shm-size=9g \
    -v "$VOLUME:/var/lib/postgresql" \
    -v "$OUT/dump:/dump:ro" -v "$SOAR_DATA:/data:ro" -v "$PDBENCH_DATA:/pdbench-data:ro" \
    -v "$OUT/1m:/out" ${scripts[@]+"${scripts[@]}"} \
    -e POSTGRES_PASSWORD=audit -e POSTGRES_DB=pg_search -e PDB_TUNE=false \
    "$IMAGE" postgres -c shared_buffers=8GB -c maintenance_work_mem=12GB \
    -c max_parallel_maintenance_workers=8 -c max_parallel_workers=8 -c max_worker_processes=32 \
    -c max_parallel_workers_per_gather=8 -c jit=off -c autovacuum=off -c max_wal_size=100GB \
    -c logging_collector=on -c log_directory=/var/lib/postgresql/log \
    -c log_filename=postgres.log >/dev/null
  for _ in $(seq 1 300); do
    logs=$(docker logs "$CONTAINER" 2>&1)
    if echo "$logs" | grep -qE "init process complete|Skipping initialization" &&
      docker exec "$CONTAINER" pg_isready -U postgres -q 2>/dev/null; then
      return
    fi
    sleep 1
  done
  echo "the container did not come up" >&2
  docker logs --tail 40 "$CONTAINER" >&2
  exit 1
}

load_table() {
  local dump="$OUT/dump/$SOURCE_TABLE.dump"
  [ -s "$dump" ] || pg_dump "$SOURCE_URL" --format=custom -t "$SOURCE_TABLE" -f "$dump"
  docker exec "$CONTAINER" pg_restore -U postgres -d pg_search --section=pre-data --section=data \
    --no-owner --no-privileges "/dump/$SOURCE_TABLE.dump"
  docker exec "$CONTAINER" psql -U postgres -d pg_search -X -q -v ON_ERROR_STOP=1 \
    -c "ALTER TABLE $SOURCE_TABLE RENAME TO cohere_wiki" -c "ANALYZE cohere_wiki" -c CHECKPOINT
}

docker volume inspect "$VOLUME" >/dev/null 2>&1 || docker volume create "$VOLUME" >/dev/null
start
loaded=$(docker exec "$CONTAINER" psql -U postgres -d pg_search -X -At \
  -c "SELECT to_regclass('cohere_wiki') IS NOT NULL")
[ "$loaded" = t ] || load_table

# pdbench's DuckDB wants an S3 credential even for local parquet; nothing here reads S3.
docker exec -e AWS_ACCESS_KEY_ID=unused -e AWS_SECRET_ACCESS_KEY=unused -e AWS_REGION=us-east-1 \
  "$CONTAINER" bash -c 'cd /tmp && /opt/router-audit/benchmarks/router-audit/run.sh --size 1m \
  --url postgresql://postgres@localhost:5432/pg_search --data /data --pdbench-data /pdbench-data \
  --allow-settings-drift --pg-log /var/lib/postgresql/log/postgres.log --out /out'
docker stop -t 60 "$CONTAINER" >/dev/null
docker rm "$CONTAINER" >/dev/null
echo "results: $OUT/1m/REPORT.md"

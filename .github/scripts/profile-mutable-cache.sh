#!/usr/bin/env bash
set -euo pipefail
readonly root="$PWD"
readonly output="$root/benchmark-profiles"
readonly pristine="$RUNNER_TEMP/mutable-cache-pristine"
readonly duration_seconds=180
readonly warmup_seconds=30
readonly profile_seconds=30
readonly profile_warmup_seconds=5
readonly startup_timeout_seconds=300
readonly database_url=postgres://postgres:postgres@127.0.0.1:5432/benchmark
readonly script="$root/benchmarker/datasets/_ci_config/k6/search.js"
test "$DATASET" = stackexchange
test "$PARADEDB_DATA_DIR" = "$RUNNER_TEMP/benchmark-pgdata"
mkdir -p "$output"
cp benchmark-artifacts/runtime-compose.yml "$output/"
docker exec paradedb psql -X -U postgres -d benchmark -c CHECKPOINT
docker compose stop paradedb
sudo cp -a --reflink=auto "$PARADEDB_DATA_DIR" "$pristine"

measure() {
  local directory="$1" seconds="$2" warmup="$3" updates="$4"
  mkdir -p "$directory"
  (
    cd "$directory"
    taskset -c 8-11 "$root/benchmarker/k6" run --out dashboard=json,html \
      --summary-export summary.json \
      -e PARADEDB_URL="$database_url" -e PARADEDB_CONTAINER=paradedb \
      -e WORKLOAD=topk -e QUERY_STYLE=disjunction -e VUS=8 \
      -e UPDATES_PER_SECOND="$updates" -e WARMUP_SECONDS="$warmup" \
      -e DURATION_SECONDS="$seconds" "$script"
  ) >"$directory/k6.log" 2>&1
  if grep -Eq '[1-9][0-9]* interrupted iterations' "$directory/k6.log"; then
    cat "$directory/k6.log"
    return 1
  fi
}

for point in baseline:64:2:1000:on larger:512:2:1000:on serial:64:0:1000:on readonly:64:2:0:on disabled:64:2:1000:off; do
  IFS=: read -r name capacity workers updates enabled <<<"$point"
  directory="$output/$name"
  mkdir -p "$directory"
  docker compose stop paradedb
  sudo rm -rf "$PARADEDB_DATA_DIR"
  sudo cp -a --reflink=auto "$pristine" "$PARADEDB_DATA_DIR"
  docker compose up -d --wait --wait-timeout "$startup_timeout_seconds" paradedb
  docker exec paradedb psql -X -U postgres -d benchmark -v ON_ERROR_STOP=1 \
    -c "ALTER SYSTEM SET paradedb.mutable_segment_cache_size = '$((capacity * 1024 * 1024))';" \
    -c "ALTER DATABASE benchmark SET max_parallel_workers_per_gather = '$workers';" \
    -c "ALTER DATABASE benchmark SET paradedb.enable_mutable_segment_cache = '$enabled';"
  docker restart paradedb
  docker compose up -d --wait --wait-timeout "$startup_timeout_seconds" paradedb
  docker exec paradedb psql -X -U postgres -d benchmark -v ON_ERROR_STOP=1 \
    -c "SELECT name,setting,unit FROM pg_settings WHERE name LIKE '%mutable%' OR name LIKE '%parallel%' OR name IN ('shared_buffers','autovacuum');" \
    -c "SELECT mutable,count(*),sum(num_docs),sum(byte_size) FROM paradedb.index_info('documents_body_bm25_idx') GROUP BY 1;" \
    >"$directory/configuration.txt"
  docker exec -i paradedb psql -XAt -U postgres -d benchmark >"$directory/activity.jsonl" <<'SQL' &
SELECT json_build_object('time',clock_timestamp(),'cache',paradedb.mutable_cache_diagnostics(),'activity',(SELECT json_agg(s) FROM (SELECT backend_type,state,wait_event_type,wait_event,count(*) FROM pg_stat_activity WHERE datname=current_database() AND pid<>pg_backend_pid() GROUP BY 1,2,3,4) s));
\watch 1
SQL
  monitor_pid=$!
  start_time="$(date -u +%FT%TZ)"
  measure "$directory/measurement" "$duration_seconds" "$warmup_seconds" "$updates"
  kill "$monitor_pid" || true
  wait "$monitor_pid" || true
  docker logs --since "$start_time" paradedb >"$directory/postgres.log" 2>&1
  docker exec paradedb psql -X -U postgres -d benchmark -v ON_ERROR_STOP=1 \
    -c "EXPLAIN (ANALYZE, BUFFERS, VERBOSE, FORMAT JSON) SELECT id, body, pdb.score(id) AS score FROM documents WHERE body ||| 'optimize OR throughput' ORDER BY score DESC LIMIT 10;" \
    -c "SELECT mutable,count(*),sum(num_docs),sum(byte_size) FROM paradedb.index_info('documents_body_bm25_idx') GROUP BY 1;" \
    >"$directory/after.txt"
  mkdir -p "$directory/profile"
  bash "$root/.github/scripts/profile-postgres.sh" paradedb "$directory/profile" -- \
    taskset -c 8-11 "$root/benchmarker/k6" run \
      -e PARADEDB_URL="$database_url" -e WORKLOAD=topk -e QUERY_STYLE=disjunction -e VUS=8 \
      -e UPDATES_PER_SECOND="$updates" -e WARMUP_SECONDS="$profile_warmup_seconds" \
      -e DURATION_SECONDS="$profile_seconds" "$script" >"$directory/profile.log" 2>&1
  echo "Completed $point"
  tail -65 "$directory/measurement/k6.log"
done

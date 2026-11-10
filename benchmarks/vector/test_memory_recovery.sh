#!/usr/bin/env bash
set -euo pipefail

output=$(realpath "${1:-benchmark-artifacts/memory-recovery-test}")
mkdir -p "$output/dataset/postgres"
container="pdbench-memory-test-${GITHUB_RUN_ID:-$$}"
trap 'docker logs "$container" > "$output/postgres.log" 2>&1 || true; docker rm -fv "$container" >/dev/null 2>&1 || true' EXIT

docker run -d --name "$container" --memory=128m --memory-swap=128m \
  -e POSTGRES_HOST_AUTH_METHOD=trust -p 127.0.0.1::5432 \
  postgres:18 postgres -c shared_buffers=16MB -c max_connections=10 -c huge_pages=off
port=$(docker port "$container" 5432/tcp | cut -d: -f2)
url="postgres://postgres@127.0.0.1:$port/postgres"
for _ in {1..60}; do
  if psql "$url" -XqAt -c 'SELECT 1' >/dev/null 2>&1; then break; fi
  sleep 1
done
psql "$url" -X -v ON_ERROR_STOP=1 \
  -c "CREATE TABLE fixture (body text); INSERT INTO fixture VALUES ('x');"

cat > "$output/dataset/schema.yaml" <<'YAML'
table: fixture
columns:
  body: text
YAML
: > "$output/dataset/postgres/pre.sql"
echo 'SELECT pg_sleep(3); CREATE INDEX fixture_idx ON fixture (body);' > "$output/dataset/postgres/post.sql"
jq -n --arg url "$url" --arg container "$container" --arg output "$output" '{
  datasource: {path: "s3://unused/"},
  datastore: {type: "postgres", url: $url, container: $container, dataset: ($output + "/dataset")},
  output: ($output + "/success")
}' > "$output/load.json"
timeout 90 pdbench-load finalize "$output/load.json" > "$output/success.log" 2>&1
jq -e '.status == "ok"' "$output"/success/load/*_load/run.json

echo "SELECT pg_sleep(3); CREATE INDEX oom_idx ON fixture ((repeat(body, 300000000)));" \
  > "$output/dataset/postgres/post.sql"
set +e
timeout 90 pdbench-load finalize "$output/load.json" --set "output=$output/failure" \
  > "$output/failure.log" 2>&1
status=$?
set -e
test "$status" -ne 0 && test "$status" -ne 124

docker events --since "$(docker inspect --format '{{.Created}}' "$container")" \
  --until "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --filter "container=$container" \
  --filter event=oom --format '{{json .}}' > "$output/oom-events.jsonl"
jq -se 'length > 0' "$output/oom-events.jsonl"
jq -e '.status == "failed" and .error != null' "$output"/failure/load/*_load/run.json
jq -se '[.[] | select(.type == "Point" and .metric == "container_memory" and .value > 0)] | length >= 2' \
  "$output"/failure/load/*_load/points.jsonl
pdbench-report html "$output/failure/load" -o "$output/failed-build.html"
test -s "$output/failed-build.html"

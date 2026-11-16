#!/usr/bin/env bash
set -euo pipefail

IFS=',' read -r -a probes <<<"$DIAGNOSTIC_PROBES"
for probe in "${probes[@]}"; do
  if [[ ! "$probe" =~ ^[1-9][0-9]*$ ]]; then
    echo "Expected comma-separated positive integers for diagnostic_probes" >&2
    exit 1
  fi
done

parallel_workers="$(python3 -c 'import os, tomllib; print(tomllib.load(open(os.environ["HN_CONFIG"], "rb"))["search"]["session"]["max_parallel_workers_per_gather"])')"
repetitions="${DIAGNOSTIC_REPETITIONS:-3}"
phases=(current)
output=benchmark-artifacts/fixed-probes
comparison_commit="${DIAGNOSTIC_COMPARISON_COMMIT:-}"
database_stop_timeout_seconds=60
database_start_timeout_seconds=300
if [[ -n "$comparison_commit" ]]; then
  phases=(current comparison comparison current)
  repetitions=1
  mkdir -p "$output"
  cat >"$output/index-identity.sql" <<'SQL'
SELECT json_build_object(
  'system_identifier', (SELECT system_identifier::text FROM pg_control_system()),
  'index', (SELECT json_build_object('oid', oid, 'relfilenode', relfilenode,
    'size', pg_relation_size(oid)) FROM pg_class WHERE oid = 'hn_items_bm25_idx'::regclass),
  'segments', (SELECT json_agg(v ORDER BY v.segno)
    FROM paradedb.vector_info('hn_items_bm25_idx', 'emb') AS v));
SQL
  psql "$PARADEDB_URL" -XqAt -v ON_ERROR_STOP=1 -f "$output/index-identity.sql" >"$output/index-before.json"
fi

for phase in "${!phases[@]}"; do
  destination="$output"
  if [[ -n "$comparison_commit" ]]; then
    destination="$output/phase-$((phase + 1))-${phases[$phase]}"
    mkdir -p "$destination"
    image="$PARADEDB_IMAGE"
    commit="$BENCHMARK_REF"
    if [[ "${phases[$phase]}" == comparison ]]; then
      image="$DIAGNOSTIC_COMPARISON_IMAGE"
      commit="$comparison_commit"
    fi
    docker compose stop --timeout "$database_stop_timeout_seconds" paradedb
    PARADEDB_IMAGE="$image" docker compose up -d --force-recreate --wait --wait-timeout "$database_start_timeout_seconds" paradedb
    image_id="$(docker inspect paradedb --format '{{.Image}}')"
    test "$image_id" = "$(docker image inspect "$image" --format '{{.Id}}')"
    jq -n --arg commit "$commit" --arg image "$image_id" --arg data_dir "$PARADEDB_DATA_DIR" \
      '{commit: $commit, image: $image, data_dir: $data_dir}' >"$destination/runtime.json"
    psql "$PARADEDB_URL" -XqAt -v ON_ERROR_STOP=1 -f "$output/index-identity.sql" >"$destination/index-before.json"
    cmp "$output/index-before.json" "$destination/index-before.json"
  fi
  for ((repeat = 1; repeat <= repetitions; repeat++)); do
    workers=("$parallel_workers" 0)
    if (((phase + repeat) % 2 == 0)); then
      workers=(0 "$parallel_workers")
    fi
    for sql in benchmarks/assets/hn/queries/range_*.sql; do
      query="$(basename "$sql" .sql)"
      for probe in "${probes[@]}"; do
        for worker_count in "${workers[@]}"; do
          pdbench search --url "$PARADEDB_URL" --config "$HN_CONFIG" \
            --assets-dir benchmarks/assets \
            --runs-dir "${destination}/${query}/${probe}/workers-${worker_count}/repeat-${repeat}" \
            --set dataset.s3_base_path=benchmark-artifacts/fixtures \
            --set "search.session.max_parallel_workers_per_gather=${worker_count}" \
            --query "$query" --probe "$probe" --no-compare --html
        done
      done
    done
  done
  if [[ -n "$comparison_commit" ]]; then
    psql "$PARADEDB_URL" -XqAt -v ON_ERROR_STOP=1 -f "$output/index-identity.sql" >"$destination/index-after.json"
    cmp "$output/index-before.json" "$destination/index-after.json"
  fi
done

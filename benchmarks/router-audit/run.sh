#!/usr/bin/env bash
# Router quality audit of one Cohere size: builds a one-segment index, exports its router, runs the
# offline audit, sweeps the online operating points and writes REPORT.md. Each step leaves a marker
# under --out, so a rerun resumes at the first unfinished one. README.md describes the outputs.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
ASSETS_SOURCE="${ASSETS_SOURCE:-$REPO/benchmarks/cohere/assets}"
ROUTER_AUDIT="${ROUTER_AUDIT:-router-audit}"
TABLE=cohere_wiki
INDEX=cohere_wiki_bm25_idx
FIELD=emb
STEM=knn_top10_unfiltered

SIZE="" URL="" DATA="" OUT="" PG_LOG="" PDBENCH_DATA=""
SETTINGS=()
while [ $# -gt 0 ]; do
  case "$1" in
    --size) SIZE="$2"; shift 2 ;;
    --url) URL="$2"; shift 2 ;;
    --data) DATA="$2"; shift 2 ;;
    --out) OUT="$2"; shift 2 ;;
    --pg-log) PG_LOG="$2"; shift 2 ;;
    --pdbench-data) PDBENCH_DATA="$2"; shift 2 ;;
    --allow-settings-drift) SETTINGS+=(--allow-settings-drift); shift ;;
    *) echo "unknown argument $1" >&2; exit 2 ;;
  esac
done
[ -n "$SIZE" ] && [ -n "$URL" ] && [ -n "$OUT" ] || {
  echo "usage: run.sh --size 1m|10m --url <pg> --out <dir> [--data <dir>] [--pdbench-data <dir>]" >&2
  exit 2
}
[ "$SIZE" = 10m ] || [ -n "$DATA" ] || { echo "--size 1m needs --data" >&2; exit 2; }
[ -z "$PDBENCH_DATA" ] || SETTINGS+=(--set "dataset.s3_base_path=$PDBENCH_DATA")

mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
PROFILE="$HERE/profile-$SIZE.toml"
ASSETS="$OUT/assets"
RUNS="$OUT/pdbench"

sql() { psql "$URL" -X -q -At -v ON_ERROR_STOP=1 -c "$1"; }

# Runs step $1 unless its marker exists.
step() {
  local name="$1"
  shift
  if [ -e "$OUT/.done-$name" ]; then
    echo "### $name: done"
    return
  fi
  echo "### $name: $(date -u +%H:%M:%S)"
  "$@"
  touch "$OUT/.done-$name"
}

stop() {
  echo "STOP: $*" >&2
  exit 3
}

build() {
  rm -rf "$ASSETS"
  cp -R "$ASSETS_SOURCE" "$ASSETS"
  cp "$HERE/index.sql" "$ASSETS/cohere/index.sql"
  # The build's timing line is a NOTICE; the server log only keeps it at this level.
  sql "ALTER SYSTEM SET log_min_messages = notice"
  sql "SELECT pg_reload_conf()" >/dev/null
  pdbench build --url "$URL" --config "$PROFILE" --assets-dir "$ASSETS" --runs-dir "$RUNS" \
    --force-rebuild "${SETTINGS[@]}" | tee "$OUT/build.log"
  if [ -n "$PG_LOG" ] && [ -r "$PG_LOG" ]; then
    grep 'ivf_build' "$PG_LOG" | tail -1 > "$OUT/ivf_build.log" || true
    cat "$OUT/ivf_build.log"
  fi
}

check_segments() {
  local ivf
  ivf=$(sql "SELECT count(*) FROM paradedb.vector_info('$INDEX', '$FIELD') WHERE vector_format = 'ivf'")
  sql "SELECT segno, vector_format, vector_num_vectors, vector_num_centroids
       FROM paradedb.vector_info('$INDEX', '$FIELD')" | tee "$OUT/segments.txt"
  [ "$ivf" = 1 ] || stop "$INDEX holds $ivf IVF segments, not one"
}

export_router() {
  sql "SELECT paradedb.vector_router_export('$INDEX', '$FIELD', '$OUT/export')" >/dev/null
  psql "$URL" -X -q -c "\\copy (SELECT ctid, _id FROM $TABLE) TO '$OUT/ids.tsv'"
  sql "SELECT to_json(cluster_sizes) FROM paradedb.vector_clusters('$INDEX', '$FIELD')
       WHERE cluster_sizes IS NOT NULL" > "$OUT/vector_clusters.json"
  python3 - "$OUT" <<'PYTHON' || stop "exported cluster sizes differ from paradedb.vector_clusters"
import json, struct, sys
out = sys.argv[1]
with open(f"{out}/export/sizes.u32", "rb") as handle:
    raw = handle.read()
sizes = list(struct.unpack(f"<{len(raw) // 4}I", raw))
with open(f"{out}/vector_clusters.json") as handle:
    assert sizes == json.load(handle)
meta = json.load(open(f"{out}/export/meta.json"))
assert meta["router_kind"] == "quantized_stacked", meta
assert meta["quantization"] == [1, 1], meta
assert meta["spill"] is False, meta
PYTHON
}

prepare_queries() {
  local data=()
  [ -z "$DATA" ] || data=(--data "$DATA")
  python3 "$HERE/prepare_queries.py" --size "$SIZE" --url "$URL" --out "$OUT" "${data[@]}"
  cat "$OUT/queries.meta.json"
}

audit() {
  "$ROUTER_AUDIT" run --export "$OUT/export" --ids "$OUT/ids.tsv" --queries "$OUT/queries.jsonl" \
    --output "$OUT/audit.json" || stop "router-audit failed; see the message above"
}

sweep_factor() {
  local factor="$1"
  local profile="$OUT/profiles/k3-$factor.toml"
  mkdir -p "$OUT/profiles"
  awk -v factor="$factor" '{ print }
    /^\[search\.session\]$/ { printf "\"paradedb.vector_router_rerank_factor\" = \"%s\"\n", factor }' \
    "$PROFILE" > "$profile"
  pdbench search --url "$URL" --config "$profile" --assets-dir "$ASSETS" --runs-dir "$RUNS" \
    --query "$STEM" --no-compare "${SETTINGS[@]}" | tee "$OUT/sweep-k3-$factor.log"
}

sweeps() {
  sweep_factor 2
  python3 "$HERE/prepare_queries.py" --size "$SIZE" --url "$URL" --out "$OUT" --check-loaded \
    || stop "pdbench's held-out queries are not the audited ones"
  sweep_factor 4
  python3 "$HERE/collect_perf.py" --url "$URL" --profile "$OUT/profiles/k3-2.toml" \
    --assets "$ASSETS" --runs-dir "$RUNS" --queries "$OUT/queries.jsonl" \
    --clusters "$(python3 -c "import json; print(json.load(open('$OUT/export/meta.json'))['clusters'])")" \
    --out "$OUT"
  "$ROUTER_AUDIT" online --export "$OUT/export" --ids "$OUT/ids.tsv" --queries "$OUT/queries.jsonl" \
    --points "$OUT/points.json" --output "$OUT/online.json"
}

report() {
  python3 "$HERE/report.py" --out "$OUT" || stop "report checks failed; see REPORT.md"
}

step 1-build build
step 2-segments check_segments
step 3-export export_router
step 4-queries prepare_queries
step 5-audit audit
step 6-sweeps sweeps
step 7-report report
echo "### report: $OUT/REPORT.md"

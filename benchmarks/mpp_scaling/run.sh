#!/bin/bash
# MPP scaling micro-benchmark runner.
# Usage: ./run.sh [n_runs]

set -euo pipefail

N_RUNS=${1:-5}

COMMON="SET paradedb.enable_aggregate_custom_scan=on;
SET paradedb.enable_join_custom_scan=on;
SET min_parallel_table_scan_size=0;
SET parallel_setup_cost=0;
SET parallel_tuple_cost=0;
SET max_parallel_workers=16;
SET work_mem='512MB'"

declare -a CONFIGS=(
  "baseline:SET paradedb.enable_mpp=off; SET max_parallel_workers_per_gather=0"
  "mpp_n2:SET paradedb.enable_mpp=on; SET paradedb.mpp_worker_count=3; SET max_parallel_workers_per_gather=2"
  "mpp_n4:SET paradedb.enable_mpp=on; SET paradedb.mpp_worker_count=5; SET max_parallel_workers_per_gather=4"
  "mpp_n8:SET paradedb.enable_mpp=on; SET paradedb.mpp_worker_count=9; SET max_parallel_workers_per_gather=8"
)

declare -a QUERIES=(
  "q1_join_count:SELECT COUNT(*) FROM mpp_bench p JOIN mpp_bench_child c ON p.id = c.parent_id WHERE p.body @@@ 'term'"
  "q2_join_low_gb:SELECT p.category, COUNT(*) FROM mpp_bench p JOIN mpp_bench_child c ON p.id = c.parent_id WHERE p.body @@@ 'term' GROUP BY p.category ORDER BY p.category"
  "q3_join_high_gb:SELECT p.user_id, COUNT(*) FROM mpp_bench p JOIN mpp_bench_child c ON p.id = c.parent_id WHERE p.body @@@ 'term' GROUP BY p.user_id ORDER BY p.user_id LIMIT 10"
  "q4_join_multi_agg:SELECT COUNT(*), SUM(c.amount), MIN(c.amount), MAX(c.amount) FROM mpp_bench p JOIN mpp_bench_child c ON p.id = c.parent_id WHERE p.body @@@ 'term'"
)

printf "%-22s %-12s" "query" "config"
for i in $(seq 1 "$N_RUNS"); do printf " run%d_ms" "$i"; done
printf " median_ms\n"

psql -h localhost -p 28818 -U "$USER" -d pg_search -X -t -A -c "SELECT pg_prewarm('mpp_bench'); SELECT pg_prewarm('mpp_bench_idx'); SELECT pg_prewarm('mpp_bench_child'); SELECT pg_prewarm('mpp_bench_child_idx');" > /dev/null 2>&1 || true

for q in "${QUERIES[@]}"; do
  qid="${q%%:*}"; qsql="${q#*:}"
  for c in "${CONFIGS[@]}"; do
    cid="${c%%:*}"; csql="${c#*:}"
    printf "%-22s %-12s" "$qid" "$cid"
    times=()
    for i in $(seq 1 "$N_RUNS"); do
      ms=$(psql -h localhost -p 28818 -U "$USER" -d pg_search -X -t -A -c "$COMMON; $csql; EXPLAIN (ANALYZE, FORMAT JSON, TIMING off, BUFFERS off) $qsql" 2>&1 | grep -oE '"Execution Time": [0-9.]+' | grep -oE '[0-9.]+' | head -1)
      if [ -z "$ms" ]; then ms="ERR"; fi
      times+=("$ms")
      printf " %8s" "$ms"
    done
    median=$(printf '%s\n' "${times[@]}" | sort -n | awk -v n="$N_RUNS" 'NR==int((n+1)/2){print}')
    printf " %9s\n" "$median"
  done
  echo "---"
done

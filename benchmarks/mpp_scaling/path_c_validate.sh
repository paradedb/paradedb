#!/bin/bash
# Path C validation harness.
#
# Asks one question: does MPP's partitioned-shuffle topology still win against
# Postgres-native parallel + DataFusion CollectLeft when the build side is wide
# enough that CollectLeft can't fit it in one process? And how do higher
# producer counts (12 / 16) compare?
#
# Three queries, three build widths:
#   q_narrow: parent has 1.5M matches (30%) — today's bench shape, CollectLeft fits
#   q_medium: parent has 3.5M matches (70%) — borderline, CollectLeft may spill
#   q_wide:   parent has 5M matches (100%)  — forces partitioned even on baseline
#
# Six configs:
#   baseline_serial — paradedb.enable_mpp=off + max_parallel_workers_per_gather=0
#   pg_parallel_n4  — paradedb.enable_mpp=off + max_parallel_workers_per_gather=4
#   mpp_n4 / n8 / n12 / n16 — paradedb.enable_mpp=on, mpp_worker_count=N+1

set -euo pipefail

N_RUNS=${1:-3}

COMMON="SET paradedb.enable_aggregate_custom_scan=on;
SET paradedb.enable_join_custom_scan=on;
SET min_parallel_table_scan_size=0;
SET parallel_setup_cost=0;
SET parallel_tuple_cost=0;
SET max_parallel_workers=32;
SET work_mem='512MB';
SET paradedb.mpp_target_partitions=2"

declare -a CONFIGS=(
    "baseline_serial:SET paradedb.enable_mpp=off; SET max_parallel_workers_per_gather=0"
    "pg_parallel_n4:SET paradedb.enable_mpp=off; SET max_parallel_workers_per_gather=4"
    "pg_parallel_n8:SET paradedb.enable_mpp=off; SET max_parallel_workers_per_gather=8"
    "mpp_n4:SET paradedb.enable_mpp=on; SET paradedb.mpp_worker_count=5; SET max_parallel_workers_per_gather=4"
    "mpp_n8:SET paradedb.enable_mpp=on; SET paradedb.mpp_worker_count=9; SET max_parallel_workers_per_gather=8"
    "mpp_n12:SET paradedb.enable_mpp=on; SET paradedb.mpp_worker_count=13; SET max_parallel_workers_per_gather=12"
    # mpp_n16 crashes the server: N² mesh edges × 64 MB = ~49 GB DSM. Out of scope for
    # path C; needs paradedb.mpp_queue_size dropped or the DSM allocator capped first.
)

declare -a QUERIES=(
    "q_narrow_count:SELECT COUNT(*) FROM mpp_bench p JOIN mpp_bench_child c ON p.id = c.parent_id WHERE p.body @@@ 'term'"
    "q_medium_count:SELECT COUNT(*) FROM mpp_bench p JOIN mpp_bench_child c ON p.id = c.parent_id WHERE p.body @@@ 'other'"
    "q_wide_count:SELECT COUNT(*) FROM mpp_bench p JOIN mpp_bench_child c ON p.id = c.parent_id WHERE p.body @@@ 'filler'"
    "q_narrow_gb:SELECT p.category, COUNT(*) FROM mpp_bench p JOIN mpp_bench_child c ON p.id = c.parent_id WHERE p.body @@@ 'term' GROUP BY p.category ORDER BY p.category"
    "q_medium_gb:SELECT p.category, COUNT(*) FROM mpp_bench p JOIN mpp_bench_child c ON p.id = c.parent_id WHERE p.body @@@ 'other' GROUP BY p.category ORDER BY p.category"
    "q_wide_gb:SELECT p.category, COUNT(*) FROM mpp_bench p JOIN mpp_bench_child c ON p.id = c.parent_id WHERE p.body @@@ 'filler' GROUP BY p.category ORDER BY p.category"
)

psql -h localhost -p 28818 -U "$USER" -d pg_search -X -t -A -c "SELECT pg_prewarm('mpp_bench'); SELECT pg_prewarm('mpp_bench_idx'); SELECT pg_prewarm('mpp_bench_child'); SELECT pg_prewarm('mpp_bench_child_idx');" > /dev/null 2>&1 || true

printf "%-18s %-18s" "query" "config"
for i in $(seq 1 "$N_RUNS"); do printf " run%d_ms" "$i"; done
printf " median_ms\n"

for q in "${QUERIES[@]}"; do
    qid="${q%%:*}"; qsql="${q#*:}"
    for c in "${CONFIGS[@]}"; do
        cid="${c%%:*}"; csql="${c#*:}"
        printf "%-18s %-18s" "$qid" "$cid"
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

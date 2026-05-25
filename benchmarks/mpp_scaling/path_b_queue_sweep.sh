#!/bin/bash
# Path B lever 2: DSM queue-size sweep.
#
# Question: does lowering `paradedb.mpp_queue_size` let MPP scale past
# producers=4 by shrinking DSM allocation latency?  The Path C bench showed
# GROUP BY queries peak at mpp_n4 (~1.5x speedup) and then collapse at n=8/12
# (worse than baseline). The DSM mesh is `n_procs^2 * mpp_queue_size`:
#   n=5  procs:  25 queues * 64 MiB = 1.6 GiB
#   n=9  procs:  81 queues * 64 MiB = 5.2 GiB
#   n=13 procs: 169 queues * 64 MiB = 10.8 GiB
#
# Hypothesis: at n>4 the DSM-init cost (mmap + page-fault-in on first touch)
# dominates the parallelism gain. Smaller queues -> less DSM -> faster init.
#
# Tests two GROUP BY queries (where MPP wins at n=4) at producers=4/8/12 across
# queue sizes 4MB / 8MB / 16MB / 32MB / 64MB (default).
#
# Scalar count is excluded because Path C established it doesn't deploy MPP
# regardless.

set -euo pipefail

N_RUNS=${1:-3}

COMMON="SET paradedb.enable_aggregate_custom_scan=on;
SET paradedb.enable_join_custom_scan=on;
SET min_parallel_table_scan_size=0;
SET parallel_setup_cost=0;
SET parallel_tuple_cost=0;
SET max_parallel_workers=32;
SET work_mem='512MB';
SET paradedb.mpp_target_partitions=2;
SET paradedb.enable_mpp=on"

declare -a CONFIGS=(
    "n4_q4MB:SET paradedb.mpp_worker_count=5; SET max_parallel_workers_per_gather=4; SET paradedb.mpp_queue_size='4MB'"
    "n4_q8MB:SET paradedb.mpp_worker_count=5; SET max_parallel_workers_per_gather=4; SET paradedb.mpp_queue_size='8MB'"
    "n4_q16MB:SET paradedb.mpp_worker_count=5; SET max_parallel_workers_per_gather=4; SET paradedb.mpp_queue_size='16MB'"
    "n4_q64MB:SET paradedb.mpp_worker_count=5; SET max_parallel_workers_per_gather=4; SET paradedb.mpp_queue_size='64MB'"
    "n8_q4MB:SET paradedb.mpp_worker_count=9; SET max_parallel_workers_per_gather=8; SET paradedb.mpp_queue_size='4MB'"
    "n8_q8MB:SET paradedb.mpp_worker_count=9; SET max_parallel_workers_per_gather=8; SET paradedb.mpp_queue_size='8MB'"
    "n8_q16MB:SET paradedb.mpp_worker_count=9; SET max_parallel_workers_per_gather=8; SET paradedb.mpp_queue_size='16MB'"
    "n8_q64MB:SET paradedb.mpp_worker_count=9; SET max_parallel_workers_per_gather=8; SET paradedb.mpp_queue_size='64MB'"
    "n12_q4MB:SET paradedb.mpp_worker_count=13; SET max_parallel_workers_per_gather=12; SET paradedb.mpp_queue_size='4MB'"
    "n12_q8MB:SET paradedb.mpp_worker_count=13; SET max_parallel_workers_per_gather=12; SET paradedb.mpp_queue_size='8MB'"
    "n12_q16MB:SET paradedb.mpp_worker_count=13; SET max_parallel_workers_per_gather=12; SET paradedb.mpp_queue_size='16MB'"
    "n12_q64MB:SET paradedb.mpp_worker_count=13; SET max_parallel_workers_per_gather=12; SET paradedb.mpp_queue_size='64MB'"
)

declare -a QUERIES=(
    "q_narrow_gb:SELECT p.category, COUNT(*) FROM mpp_bench p JOIN mpp_bench_child c ON p.id = c.parent_id WHERE p.body @@@ 'term' GROUP BY p.category ORDER BY p.category"
    "q_wide_gb:SELECT p.category, COUNT(*) FROM mpp_bench p JOIN mpp_bench_child c ON p.id = c.parent_id WHERE p.body @@@ 'filler' GROUP BY p.category ORDER BY p.category"
)

psql -h localhost -p 28818 -U "$USER" -d pg_search -X -t -A -c "SELECT pg_prewarm('mpp_bench'); SELECT pg_prewarm('mpp_bench_idx'); SELECT pg_prewarm('mpp_bench_child'); SELECT pg_prewarm('mpp_bench_child_idx');" > /dev/null 2>&1 || true

printf "%-14s %-14s" "query" "config"
for i in $(seq 1 "$N_RUNS"); do printf " run%d_ms" "$i"; done
printf " median_ms\n"

for q in "${QUERIES[@]}"; do
    qid="${q%%:*}"; qsql="${q#*:}"
    for c in "${CONFIGS[@]}"; do
        cid="${c%%:*}"; csql="${c#*:}"
        printf "%-14s %-14s" "$qid" "$cid"
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

-- Smoke test for the three mpp_trace instruments added in 1667f21e3 / 0e87ea7ea / b1c324ca8.
-- Confirms each line emits sensible values at N=4 and N=8 against the local 5M/25M bench data.
-- Run via: PGVER=18.1 ./scripts/pg_search_run.sh -f benchmarks/mpp_scaling/smoke_instruments.sql

\timing on

SET paradedb.enable_aggregate_custom_scan = on;
SET paradedb.enable_join_custom_scan = on;
SET min_parallel_table_scan_size = 0;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
SET max_parallel_workers = 32;
SET work_mem = '512MB';

SET paradedb.enable_mpp = on;
SET paradedb.mpp_trace = on;
SET paradedb.mpp_target_partitions = 2;

\echo '========================================'
\echo 'N=4 (mpp_worker_count=5, peers=4): expect mesh_init with peer_attach_calls=8, slots_created=25'
\echo '========================================'
SET paradedb.mpp_worker_count = 5;
SET max_parallel_workers_per_gather = 4;
SELECT p.category, COUNT(*)
FROM mpp_bench p JOIN mpp_bench_child c ON p.id = c.parent_id
WHERE p.body @@@ 'term'
GROUP BY p.category ORDER BY p.category;

\echo ''
\echo '========================================'
\echo 'N=8 (mpp_worker_count=9, peers=8): expect peer_attach_calls=16, slots_created=81'
\echo '========================================'
SET paradedb.mpp_worker_count = 9;
SET max_parallel_workers_per_gather = 8;
SELECT p.category, COUNT(*)
FROM mpp_bench p JOIN mpp_bench_child c ON p.id = c.parent_id
WHERE p.body @@@ 'term'
GROUP BY p.category ORDER BY p.category;

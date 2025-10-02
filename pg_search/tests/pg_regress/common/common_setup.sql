CREATE EXTENSION IF NOT EXISTS pg_search;

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;
SET paradedb.enable_mixed_fast_field_exec = true;

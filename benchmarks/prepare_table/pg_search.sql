CREATE EXTENSION IF NOT EXISTS pg_search;
DROP INDEX IF EXISTS benchmark_logs_idx;
SET maintenance_work_mem = '8GB';

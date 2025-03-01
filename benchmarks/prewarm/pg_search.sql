CREATE EXTENSION IF NOT EXISTS pg_prewarm;
SELECT pg_prewarm('benchmark_logs_idx');

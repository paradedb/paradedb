-- Cleanup for advanced composite type tests
DROP SCHEMA IF EXISTS composite_adv CASCADE;
RESET search_path;
RESET max_parallel_workers_per_gather;
RESET max_parallel_maintenance_workers;

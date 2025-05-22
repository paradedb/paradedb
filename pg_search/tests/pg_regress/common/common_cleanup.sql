-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_mixed_fast_field_exec;
SELECT 'Common tests cleanup complete' AS status; 

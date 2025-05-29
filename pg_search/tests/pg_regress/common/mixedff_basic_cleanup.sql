-- Cleanup for basic mixed fast fields tests (01-04)

-- Drop the table used in these tests
DROP TABLE IF EXISTS mixed_numeric_string_test CASCADE;

-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_mixed_fast_field_exec;
RESET paradedb.mixed_fast_field_exec_column_threshold;
SELECT 'Basic tests cleanup complete' AS status; 

-- Cleanup for basic columnar storage tests (01-04)

-- Drop the table used in these tests
DROP TABLE IF EXISTS mixed_numeric_string_test CASCADE;

-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_columnar_exec;
RESET paradedb.columnar_exec_column_threshold;
SELECT 'Basic tests cleanup complete' AS status; 
